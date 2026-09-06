//! Everything between a relay and a handler.
//!
//! A node writes handlers; this runs the protocol around them. Presence of
//! a handler is the declaration at both levels — which protocols this
//! service speaks, and which methods within them:
//!
//! ```ignore
//! Service::new(signer, relay, owners)
//!     .control(node)      // 13198 published, 23198 served
//!     .run().await        // no .wallet() — 13194 never published,
//!                         // 23194 answered NOT_IMPLEMENTED
//! ```

use std::sync::{Arc, RwLock};

use nostr::key::PublicKey;
use nostr::signer::{IntoNostrSigner, NostrSigner};
use nostr_sdk::prelude::*;

use super::handler::{ControlService, WalletService};
use super::pipeline::{self, Handler};
use super::state::Usage;
use crate::nnc::{ErrorCode, Method, NncError, Notification, Request, Response};
use crate::{Grants, Subscriptions, GRANT_KIND, SUBSCRIPTION_KIND};

/// How often to look for a relay that has come back.
///
/// Reconnection is not in the SDK's notification stream, so it is observed
/// by watching relay status.
pub const RECONNECT_POLL: std::time::Duration = std::time::Duration::from_secs(5);

/// NWC info, kind 13194.
pub const WALLET_INFO_KIND: u16 = 13194;
/// NWC request, kind 23194.
pub const WALLET_REQUEST_KIND: u16 = 23194;
/// NWC response, kind 23195.
pub const WALLET_RESPONSE_KIND: u16 = 23195;
/// NNC info, kind 13198.
pub const CONTROL_INFO_KIND: u16 = 13198;
/// NNC request, kind 23198.
pub const CONTROL_REQUEST_KIND: u16 = 23198;
/// NNC response, kind 23199.
pub const CONTROL_RESPONSE_KIND: u16 = 23199;
/// NNC notification, kind 23200.
pub const NOTIFICATION_KIND: u16 = 23200;

/// What can go wrong starting or running a service.
#[derive(Debug)]
pub enum Error {
    /// The relay layer failed.
    Relay(String),
    /// The signer failed.
    Signer(nostr::signer::SignerError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Relay(e) => write!(f, "relay: {e}"),
            Self::Signer(e) => write!(f, "signer: {e}"),
        }
    }
}

impl std::error::Error for Error {}

/// Who is currently entitled to what, for the subscription route.
///
/// A **mirror**, not the authority: `run` owns the grants and
/// subscriptions it serves requests from, and copies them here whenever
/// they change. That is deliberate. The request path holds a `&Grants`
/// across the handler's execution, so if this were the authority a handler
/// that announced something would be waiting on a lock its own caller
/// holds. Mirroring costs a clone on a grant event — rare — and makes that
/// deadlock unrepresentable.
#[derive(Debug, Default)]
struct Delivery {
    grants: Grants,
    subs: Subscriptions,
}

/// A node service.
pub struct Service {
    signer: Arc<dyn NostrSigner>,
    client: Client,
    relays: Vec<String>,
    owners: Vec<PublicKey>,
    wallet: Option<Arc<dyn WalletService>>,
    control: Option<Arc<dyn ControlService>>,
    delivery: Arc<RwLock<Delivery>>,
}

/// Sends notifications on a node service's behalf.
///
/// A handler receives one when it is built, because notifications are
/// triggered by node work rather than by a request — a channel confirming,
/// a peer force-closing — and a handler that can only answer calls cannot
/// express that.
///
/// It is cheap to clone and `Send + 'static`, so a node can hold one in a
/// background task.
#[derive(Clone)]
pub struct Notifier {
    client: Client,
    signer: Arc<dyn NostrSigner>,
    delivery: Arc<RwLock<Delivery>>,
}

impl Notifier {
    /// Send a notification to one controller.
    ///
    /// `in_reply_to` is the request that caused it, for the deferred result
    /// of an asynchronous command — NIP-XX makes the `e` tag a MUST there,
    /// and it is what lets a client tie an outcome to the command that
    /// caused it. Pass `None` for a subscription delivery, which follows
    /// from no request.
    pub async fn notify(
        &self,
        to: &PublicKey,
        notification: &Notification,
        in_reply_to: Option<EventId>,
    ) -> Result<(), Error> {
        let json = serde_json::to_string(notification).map_err(|e| Error::Relay(e.to_string()))?;
        let ciphertext = self
            .signer
            .nip44_encrypt(to, &json)
            .await
            .map_err(Error::Signer)?;
        let mut tags = vec![Tag::public_key(*to)];
        if let Some(id) = in_reply_to {
            tags.push(Tag::event(id));
        }
        let event = EventBuilder::new(Kind::Custom(NOTIFICATION_KIND), ciphertext).tags(tags);
        self.client.send_event_builder(event).await.map_err(relay_err)?;
        Ok(())
    }

    /// Send a notification to everyone subscribed to its type.
    ///
    /// This is the other notification route: it follows from no request, so
    /// it carries **no** `e` tag, and it is addressed to whoever has both
    /// subscribed to the type and been granted it — the intersection, never
    /// one or the other. A node calls this when something happens to it
    /// rather than when somebody asks.
    ///
    /// Returns how many controllers it reached. Zero is normal and not an
    /// error: nobody is subscribed.
    pub async fn announce(&self, notification: &Notification) -> Result<usize, Error> {
        let ty = notification.notification_type.as_str().to_string();
        // Computed under the lock and released before any send, so a
        // handler announcing from inside a request cannot stall the loop.
        let to: Vec<PublicKey> = {
            let d = self.delivery.read().unwrap_or_else(|e| e.into_inner());
            d.subs.recipients(&ty, &d.grants)
        };
        let mut sent = 0;
        for controller in &to {
            // One failed recipient must not silence the rest.
            match self.notify(controller, notification, None).await {
                Ok(()) => sent += 1,
                Err(e) => tracing::warn!("could not announce {ty} to {controller}: {e}"),
            }
        }
        tracing::debug!("announced {ty} to {sent} of {} subscriber(s)", to.len());
        Ok(sent)
    }
}

impl Service {
    /// Build one.
    ///
    /// **An empty `owners` accepts no grants at all**, and therefore
    /// answers nothing. Absent configuration fails closed rather than
    /// treating "no owners" as "any owner", which is
    /// [dln-node#1](https://github.com/DarkWebDivingClub/dln-node/issues/1).
    pub fn new<S: IntoNostrSigner>(
        signer: S,
        relays: Vec<String>,
        owners: Vec<PublicKey>,
    ) -> Self {
        let signer = signer.into_nostr_signer();
        let client = Client::builder().signer(signer.clone()).build();
        Self {
            signer,
            client,
            relays,
            owners,
            wallet: None,
            control: None,
            delivery: Arc::new(RwLock::new(Delivery::default())),
        }
    }

    /// A handle for sending notifications.
    ///
    /// Available **before** `run`, so a handler can be built holding one.
    /// That is the constructor shape the epic asked 13.2 to leave room for:
    /// a handler is constructed with what it needs rather than bare.
    pub fn notifier(&self) -> Notifier {
        Notifier {
            client: self.client.clone(),
            signer: self.signer.clone(),
            delivery: self.delivery.clone(),
        }
    }

    /// Serve NWC. Publishes kind 13194 and answers 23194.
    pub fn wallet(mut self, handler: Arc<dyn WalletService>) -> Self {
        self.wallet = Some(handler);
        self
    }

    /// Serve NNC. Publishes kind 13198 and answers 23198.
    pub fn control(mut self, handler: Arc<dyn ControlService>) -> Self {
        self.control = Some(handler);
        self
    }

    /// Connect, publish info events, and answer requests until stopped.
    pub async fn run(self) -> Result<(), Error> {
        let me = self.signer.get_public_key().await.map_err(Error::Signer)?;
        let client = self.client.clone();
        for relay in &self.relays {
            client.add_relay(relay.as_str()).await.map_err(relay_err)?;
        }
        client.connect().await;

        self.publish_info(&client).await?;
        self.subscribe(&client, me).await?;

        let mut grants = Grants::new(me, self.owners.clone());
        let mut subs = Subscriptions::new(me);
        let mut usage = Usage::new();

        // The state a reconnect must restore. Both kinds are addressable,
        // so current state is one query and no history is needed.
        self.refresh(&client, me, &mut grants, &mut subs).await;
        self.mirror(&grants, &subs);

        // The SDK's notification stream carries events, relay messages and
        // shutdown — no relay status. So reconnection is observed by
        // watching `Relay::status()` for a transition, rather than by
        // polling `refresh` on a timer and hoping.
        let mut connected = self.connected_now(&client).await;

        let mut notifications = client.notifications();
        let mut tick = tokio::time::interval(RECONNECT_POLL);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                n = notifications.next() => {
                    match n {
                        Some(ClientNotification::Event { event, .. }) => {
                            self.on_event(&client, me, &event, &mut grants, &mut subs, &mut usage)
                                .await;
                        }
                        Some(ClientNotification::Shutdown) | None => break,
                        _ => {}
                    }
                }
                _ = tick.tick() => {
                    let now_connected = self.connected_now(&client).await;
                    // A relay that was away and is back. Re-read rather
                    // than resume: a subscription with `since(now)` would
                    // silently miss a revocation published while we were
                    // gone, and the service would keep enforcing a grant
                    // that no longer exists.
                    let returned: Vec<_> =
                        now_connected.difference(&connected).cloned().collect();
                    if !returned.is_empty() {
                        tracing::info!(
                            "reconnected to {returned:?} — re-reading grants and subscriptions"
                        );
                        let _ = self.subscribe(&client, me).await;
                        self.refresh(&client, me, &mut grants, &mut subs).await;
                        self.mirror(&grants, &subs);
                    }
                    connected = now_connected;
                }
            }
        }
        Ok(())
    }

    /// Which relays are connected right now.
    async fn connected_now(&self, client: &Client) -> std::collections::HashSet<String> {
        client
            .relays()
            .await
            .into_iter()
            .filter(|(_, r)| r.status().is_connected())
            .map(|(url, _)| url.to_string())
            .collect()
    }

    /// Copy the current entitlements to where a [`Notifier`] can read them.
    ///
    /// Called after anything that changes who may receive what — including
    /// a revocation, which has to end the subscription route as well as the
    /// request route.
    fn mirror(&self, grants: &Grants, subs: &Subscriptions) {
        let mut d = self.delivery.write().unwrap_or_else(|e| e.into_inner());
        d.grants = grants.clone();
        d.subs = subs.clone();
    }

    /// Read current grants and subscriptions.
    ///
    /// Called at startup and on every reconnect. Both kinds are
    /// addressable — only the latest per `(kind, pubkey, d)` exists — so
    /// this is exact rather than a best effort at replaying history.
    async fn refresh(
        &self,
        client: &Client,
        me: PublicKey,
        grants: &mut Grants,
        subs: &mut Subscriptions,
    ) {
        let filter = Filter::new()
            .kinds([Kind::Custom(GRANT_KIND), Kind::Custom(SUBSCRIPTION_KIND)])
            .pubkey(me);
        let found = client
            .fetch_events(filter)
            .timeout(std::time::Duration::from_secs(10))
            .await;
        let Ok(events) = found else {
            tracing::warn!("could not refresh grants: {:?}", found.err());
            return;
        };
        let mut list: Vec<_> = events.into_iter().collect();
        // Oldest first, so newer grants supersede rather than being
        // rejected as superseded.
        list.sort_by_key(|e| e.created_at);
        for event in list {
            if event.kind == Kind::Custom(GRANT_KIND) {
                let _ = grants.apply(&event);
            } else {
                let _ = subs.apply(&event, grants);
            }
        }
    }

    async fn publish_info(&self, client: &Client) -> Result<(), Error> {
        if let Some(w) = &self.wallet {
            let content = w.methods().join(" ");
            let e = EventBuilder::new(Kind::Custom(WALLET_INFO_KIND), content)
                .tag(Tag::parse(vec!["encryption".to_string(), "nip44_v2".to_string()]).unwrap());
            client.send_event_builder(e).await.map_err(relay_err)?;
        }
        if let Some(c) = &self.control {
            let content = c.methods().join(" ");
            let e = EventBuilder::new(Kind::Custom(CONTROL_INFO_KIND), content)
                .tag(Tag::parse(vec!["encryption".to_string(), "nip44_v2".to_string()]).unwrap());
            client.send_event_builder(e).await.map_err(relay_err)?;
        }
        Ok(())
    }

    async fn subscribe(&self, client: &Client, me: PublicKey) -> Result<(), Error> {
        let mut kinds = vec![Kind::Custom(GRANT_KIND), Kind::Custom(SUBSCRIPTION_KIND)];
        if self.wallet.is_some() {
            kinds.push(Kind::Custom(WALLET_REQUEST_KIND));
        }
        if self.control.is_some() {
            kinds.push(Kind::Custom(CONTROL_REQUEST_KIND));
        }
        client
            .subscribe(Filter::new().kinds(kinds).pubkey(me))
            .await
            .map_err(relay_err)?;
        Ok(())
    }

    async fn on_event(
        &self,
        client: &Client,
        me: PublicKey,
        event: &Event,
        grants: &mut Grants,
        subs: &mut Subscriptions,
        usage: &mut Usage,
    ) {
        let kind = event.kind.as_u16();
        match kind {
            GRANT_KIND => {
                // A grant arriving over the relay takes effect at once,
                // including revocation by empty grant. No restart.
                match grants.apply(event) {
                    Ok(c) => tracing::info!("applied a grant for {c}"),
                    Err(e) => tracing::debug!("ignored a grant: {e:?}"),
                }
                // A revocation must end the subscription route too, not
                // only the request route — delivery is the intersection.
                self.mirror(grants, subs);
            }
            SUBSCRIPTION_KIND => {
                let _ = subs.apply(event, grants);
                self.mirror(grants, subs);
            }
            WALLET_REQUEST_KIND | CONTROL_REQUEST_KIND => {
                let response = self.answer(event, kind, grants, usage).await;
                if let Err(e) = self.reply(client, event, kind, response).await {
                    tracing::warn!("could not reply: {e}");
                }
            }
            _ => {}
        }
        let _ = me;
    }

    /// Steps 1 through 7, for one request.
    async fn answer(
        &self,
        event: &Event,
        kind: u16,
        grants: &Grants,
        usage: &mut Usage,
    ) -> Response {
        // ── 1. decode ────────────────────────────────────────────────
        // NIP-44 only. A request that is not NIP-44 is refused rather than
        // guessed at — dln-node#3 is a default that silently falls back to
        // NIP-04, so a conforming client cannot read the answer.
        let plaintext = match self.signer.nip44_decrypt(&event.pubkey, &event.content).await {
            Ok(p) => p,
            Err(_) => {
                return Response::err(
                    Method::Unknown(String::new()),
                    NncError::new(
                        ErrorCode::Unknown("UNSUPPORTED_ENCRYPTION".into()),
                        "this service speaks NIP-44 only",
                    ),
                )
            }
        };
        let request: Request = match serde_json::from_str(&plaintext) {
            Ok(r) => r,
            Err(e) => {
                return Response::err(
                    Method::Unknown(String::new()),
                    NncError::new(ErrorCode::Other, format!("malformed request: {e}")),
                )
            }
        };

        let now = nostr::types::Timestamp::now().as_secs();
        let handler = match kind {
            WALLET_REQUEST_KIND => match &self.wallet {
                Some(w) => Handler::Wallet(w.as_ref()),
                None => {
                    return Response::err(
                        request.method.clone(),
                        NncError::not_implemented(&request.method),
                    )
                }
            },
            _ => match &self.control {
                Some(c) => Handler::Control(c.as_ref()),
                None => {
                    return Response::err(
                        request.method.clone(),
                        NncError::not_implemented(&request.method),
                    )
                }
            },
        };

        match pipeline::handle(
            &handler,
            grants,
            usage,
            &event.pubkey,
            Some(event.id),
            &request.method,
            &request.params,
            now,
        )
        .await
        {
            Ok(value) => Response {
                result_type: request.method,
                result: Some(value),
                error: None,
            },
            Err(e) => Response::err(request.method, e),
        }
    }

    async fn reply(
        &self,
        client: &Client,
        request: &Event,
        request_kind: u16,
        response: Response,
    ) -> Result<(), Error> {
        let json = serde_json::to_string(&response)
            .map_err(|e| Error::Relay(e.to_string()))?;
        let ciphertext = self
            .signer
            .nip44_encrypt(&request.pubkey, &json)
            .await
            .map_err(Error::Signer)?;
        let kind = if request_kind == WALLET_REQUEST_KIND {
            WALLET_RESPONSE_KIND
        } else {
            CONTROL_RESPONSE_KIND
        };
        let event = EventBuilder::new(Kind::Custom(kind), ciphertext)
            .tags([Tag::public_key(request.pubkey), Tag::event(request.id)]);
        client.send_event_builder(event).await.map_err(relay_err)?;
        Ok(())
    }
}

fn relay_err<E: std::fmt::Display>(e: E) -> Error {
    Error::Relay(e.to_string())
}
