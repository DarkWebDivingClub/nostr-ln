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

/// Why a controller is receiving a notification.
///
/// NIP-XX requires a notification to **name what caused it**, on both
/// routes: an `e` tag for the deferred result of an asynchronous command,
/// an `a` tag for a subscription delivery, and both when one notification
/// is both.
///
/// There is deliberately no way to spell *no cause*. An untagged
/// notification is malformed, and it is the malformed event that is
/// expensive: it is byte-identical to a legitimate subscription delivery,
/// so a client cannot tell that anything is wrong — it waits for an outcome
/// that already arrived and was discarded, and reports a timeout. That
/// exact failure cost twenty seconds of silence in
/// [17.3](https://github.com/DarkWebDivingClub/nostr-ln-e2e-test), from
/// passing `None` where an id belonged. Making the state unconstructable
/// is cheaper than testing for it, as with [`VerifiedGrant`].
///
/// [`VerifiedGrant`]: crate::VerifiedGrant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// The deferred result of an asynchronous command. Carries an `e` tag.
    Command(EventId),
    /// The delivery of a subscription. Carries an `a` tag.
    Subscription,
    /// Both: the controller issued the command *and* subscribed to the
    /// type. Carries both tags, in one notification rather than two.
    Both(EventId),
}

impl Cause {
    /// The request this is the outcome of, if it is one.
    pub fn request_id(&self) -> Option<EventId> {
        match self {
            Self::Command(id) | Self::Both(id) => Some(*id),
            Self::Subscription => None,
        }
    }

    /// Whether it is also a subscription delivery.
    pub fn is_subscription(&self) -> bool {
        matches!(self, Self::Subscription | Self::Both(_))
    }
}

impl Notifier {
    /// Send a notification to one controller, naming what caused it.
    ///
    /// The tags follow from `cause` mechanically, which is the point of
    /// taking one rather than an `Option<EventId>`: an `a` tag naming this
    /// controller's subscription, an `e` tag naming the request, or both.
    /// The `a` tag is per recipient — it names *their* subscription, since
    /// a notification is addressed and encrypted to exactly one controller.
    pub async fn notify(
        &self,
        to: &PublicKey,
        notification: &Notification,
        cause: Cause,
    ) -> Result<(), Error> {
        let json = serde_json::to_string(notification).map_err(|e| Error::Relay(e.to_string()))?;
        let ciphertext = self
            .signer
            .nip44_encrypt(to, &json)
            .await
            .map_err(Error::Signer)?;
        let mut tags = vec![Tag::public_key(*to)];
        if let Some(id) = cause.request_id() {
            tags.push(Tag::event(id));
        }
        if cause.is_subscription() {
            let me = self.signer.get_public_key().await.map_err(Error::Signer)?;
            tags.push(Tag::coordinate(
                Coordinate {
                    kind: Kind::Custom(SUBSCRIPTION_KIND),
                    public_key: *to,
                    identifier: me.to_hex(),
                },
                None,
            ));
        }
        let event = EventBuilder::new(Kind::Custom(NOTIFICATION_KIND), ciphertext).tags(tags);
        self.client.send_event_builder(event).await.map_err(relay_err)?;
        Ok(())
    }

    /// Deliver one notification by every route that applies to it.
    ///
    /// This is the whole of NIP-XX's delivery rule in one call, and the
    /// reason a node should prefer it to [`notify`](Self::notify):
    ///
    /// - `caller` — the controller that issued the command this is the
    ///   outcome of, with its request id — receives it whether or not it is
    ///   subscribed, as [`Cause::Both`] if it is and [`Cause::Command`] if
    ///   it is not.
    /// - every **other** subscriber receives it as
    ///   [`Cause::Subscription`].
    ///
    /// A caller that is also a subscriber gets **one** notification, not
    /// two, carrying both tags. That is what makes one delivery sufficient:
    /// nothing here has to notice the overlap and suppress a second send,
    /// and the client is not left to guess which route a single tag meant.
    ///
    /// Pass `None` for something nobody asked for — a peer force-closing —
    /// which is [`announce`](Self::announce).
    ///
    /// **Pass `None` for `"notify": false` as well, and still call this.**
    /// That flag declines the *correlated result*; it does not unsubscribe.
    /// A controller that set it and holds a subscription still receives the
    /// notification, with an `a` tag and no `e` tag. Skipping the call
    /// entirely — the obvious reading of "suppress the notification", and
    /// what this crate's own dummy did until an e2e scenario caught it —
    /// silently revokes a subscription the controller never withdrew.
    ///
    /// Returns how many controllers it reached.
    pub async fn deliver(
        &self,
        notification: &Notification,
        caller: Option<(PublicKey, EventId)>,
    ) -> Result<usize, Error> {
        let ty = notification.notification_type.as_str().to_string();
        // Computed under the lock and released before any send, so a
        // handler delivering from inside a request cannot stall the loop.
        let subscribers: Vec<PublicKey> = {
            let d = self.delivery.read().unwrap_or_else(|e| e.into_inner());
            d.subs.recipients(&ty, &d.grants)
        };

        let mut recipients: Vec<(PublicKey, Cause)> = Vec::with_capacity(subscribers.len() + 1);
        if let Some((who, request_id)) = caller {
            let also_subscribed = subscribers.contains(&who);
            recipients.push((
                who,
                if also_subscribed { Cause::Both(request_id) } else { Cause::Command(request_id) },
            ));
        }
        for s in &subscribers {
            if caller.map(|(who, _)| who) == Some(*s) {
                continue; // already queued, with both causes
            }
            recipients.push((*s, Cause::Subscription));
        }

        let mut sent = 0;
        for (controller, cause) in &recipients {
            // One failed recipient must not silence the rest.
            match self.notify(controller, notification, *cause).await {
                Ok(()) => sent += 1,
                Err(e) => tracing::warn!("could not deliver {ty} to {controller}: {e}"),
            }
        }
        tracing::debug!("delivered {ty} to {sent} of {} recipient(s)", recipients.len());
        Ok(sent)
    }

    /// Send a notification to everyone subscribed to its type.
    ///
    /// The subscription route alone, for something that follows from no
    /// command at all — a peer force-closing a channel. Addressed to
    /// whoever has both subscribed to the type and been granted it: the
    /// intersection, never one or the other.
    ///
    /// Returns how many controllers it reached. Zero is normal and not an
    /// error: nobody is subscribed.
    pub async fn announce(&self, notification: &Notification) -> Result<usize, Error> {
        self.deliver(notification, None).await
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
                let _ = subs.apply(&event, grants, self.signer.as_ref()).await;
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
                let _ = subs.apply(event, grants, self.signer.as_ref()).await;
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
                        ErrorCode::UnsupportedEncryption,
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
