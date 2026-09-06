//! The handle an asynchronous command returns.
//!
//! `open_channel` and `close_channel` are acknowledged, not completed. The
//! outcome arrives later as a kind `23200` notification carrying an `e` tag
//! with the request's id — a MUST since NIP-XX `514c196`, and the reason
//! this can be a handle rather than a guess. Without it, two concurrent
//! opens to the same peer would be indistinguishable.
//!
//! **It is owned, not borrowed.** A handle holding `&Client` could only be
//! awaited inline, because `tokio::spawn` needs `Send + 'static` — and a
//! dashboard clicking *open channel* cannot block for six blocks.

use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use nostr::event::EventId;
use nostr::key::PublicKey;
use nostr::signer::NostrSigner;
use nostr_sdk::prelude::*;
use serde::de::DeserializeOwned;

use super::Error;
use crate::nnc::Notification;

/// The outcome of an asynchronous command, once it happens.
///
/// Await it, spawn it, or drop it. Dropping unsubscribes, so a caller that
/// stops caring costs the client nothing further.
///
/// Note that dropping is **not** the same as asking for no notification:
/// the node still sends one nobody reads. Use the
/// `_without_notification` form to tell it not to bother.
#[must_use = "the outcome arrives as a notification — await this, or use the \
              _without_notification form of the call"]
pub struct Pending<T> {
    client: Client,
    signer: Arc<dyn NostrSigner>,
    subscription: SubscriptionId,
    request_id: EventId,
    service: PublicKey,
    timeout: Duration,
    _outcome: PhantomData<fn() -> T>,
}

impl<T> Pending<T> {
    pub(crate) fn new(
        client: Client,
        signer: Arc<dyn NostrSigner>,
        subscription: SubscriptionId,
        request_id: EventId,
        service: PublicKey,
        timeout: Duration,
    ) -> Self {
        Self {
            client,
            signer,
            subscription,
            request_id,
            service,
            timeout,
            _outcome: PhantomData,
        }
    }

    /// The request this is the outcome of.
    pub fn request_id(&self) -> EventId {
        self.request_id
    }

    /// How long awaiting will wait.
    ///
    /// Much longer than a request timeout by default: this waits for an
    /// on-chain confirmation, not a round trip.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Change it.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl<T: DeserializeOwned + Send> Pending<T> {
    async fn wait(self) -> Result<T, Error> {
        let deadline = tokio::time::Instant::now() + self.timeout;
        let mut notifications = self.client.notifications();

        let outcome = loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                break Err(Error::TimedOut(self.timeout));
            }
            let next = tokio::time::timeout(left, notifications.next()).await;
            let Ok(Some(ClientNotification::Event { event, .. })) = next else {
                if next.is_err() {
                    break Err(Error::TimedOut(self.timeout));
                }
                continue;
            };
            if event.kind != Kind::Custom(crate::nnc::client::NOTIFICATION_KIND) {
                continue;
            }
            // The e tag is what ties an outcome to the command that caused
            // it. Two concurrent opens to the same peer are otherwise
            // indistinguishable, which is why the spec makes it a MUST.
            let matches = event.tags.iter().any(|t| {
                let s = t.as_slice();
                s.first().map(String::as_str) == Some("e")
                    && s.get(1).map(String::as_str) == Some(self.request_id.to_hex().as_str())
            });
            if !matches {
                continue;
            }
            let plaintext = self
                .signer
                .nip44_decrypt(&self.service, &event.content)
                .await
                .map_err(Error::Signer)?;
            let n: Notification = serde_json::from_str(&plaintext)?;
            break n.as_typed::<T>().map_err(Error::Json);
        };

        // Explicit rather than relying on Drop, so the unsubscribe is
        // awaited rather than fired into a runtime that may be shutting
        // down.
        let _ = self.client.unsubscribe(&self.subscription).await;
        outcome
    }
}

impl<T: DeserializeOwned + Send + 'static> IntoFuture for Pending<T> {
    type Output = Result<T, Error>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.wait())
    }
}

impl<T> Drop for Pending<T> {
    /// Dropping a live handle unsubscribes.
    ///
    /// `Drop` cannot await, so this spawns the unsubscribe. If there is no
    /// runtime — the handle outlived it — the subscription dies with the
    /// client anyway.
    fn drop(&mut self) {
        let client = self.client.clone();
        let id = self.subscription.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = client.unsubscribe(&id).await;
            });
        }
    }
}
