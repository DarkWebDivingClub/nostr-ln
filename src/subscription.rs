//! Subscriptions: kind `30199`, authored by the controller.
//!
//! A subscription says what a controller **wants**; its grant says what it
//! **may have**. Delivery is the intersection, and computing that is this
//! module's whole job — sending anything is mission 13.3.
//!
//! **A subscription authorizes nothing.** A controller with no grant, or
//! one whose grant permits no notification types, receives nothing however
//! it subscribes. There is no response to a published event, so an
//! unauthorized subscription is silent rather than refused.

use std::collections::HashMap;

use nostr::event::Event;
use nostr::key::PublicKey;

use crate::grant::Grants;

/// Kind of a subscription. Addressable: newest per `(kind, pubkey, d)`.
pub const SUBSCRIPTION_KIND: u16 = 30199;

/// Why a subscription event was not applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejected {
    /// The `d` tag is absent or names another node.
    WrongTarget,
    /// No `p` tag naming this node service.
    NotAddressedToThisNode,
    /// The content is not a JSON array of strings.
    Unparseable,
    /// The author holds no grant on this node.
    ///
    /// Anyone may publish a kind `30199` naming any node, so a registry
    /// keyed by whoever published one is unbounded. Keyed by controllers
    /// that hold grants, it is bounded by what the owner wrote.
    NoGrant,
    /// Older than the subscription already applied for this controller.
    Superseded,
    /// Not a kind-`30199` event.
    WrongKind,
}

#[derive(Debug, Clone)]
struct Wanted {
    types: Vec<String>,
    created_at: u64,
}

/// What each controller has asked to be told about.
#[derive(Debug, Clone, Default)]
pub struct Subscriptions {
    node: Option<PublicKey>,
    by_controller: HashMap<String, Wanted>,
}

impl Subscriptions {
    /// Configure the node's own pubkey.
    pub fn new(node: PublicKey) -> Self {
        Self { node: Some(node), by_controller: HashMap::new() }
    }

    /// Apply a kind-`30199` event, given the grants in force.
    ///
    /// The author *is* the subscriber — there is nothing else it could
    /// mean — so no separate authorship check is needed beyond requiring
    /// that the author holds a grant.
    pub fn apply(&mut self, event: &Event, grants: &Grants) -> Result<usize, Rejected> {
        let node = self.node.ok_or(Rejected::WrongTarget)?;
        if event.kind.as_u16() != SUBSCRIPTION_KIND {
            return Err(Rejected::WrongKind);
        }
        let node_hex = node.to_hex();

        let d = event
            .tags
            .iter()
            .find_map(|t| {
                let s = t.as_slice();
                (s.first().map(String::as_str) == Some("d"))
                    .then(|| s.get(1).cloned())
                    .flatten()
            })
            .ok_or(Rejected::WrongTarget)?;
        if d != node_hex {
            return Err(Rejected::WrongTarget);
        }

        let p_tagged = event.tags.iter().any(|t| {
            let s = t.as_slice();
            s.first().map(String::as_str) == Some("p")
                && s.get(1).map(String::as_str) == Some(node_hex.as_str())
        });
        if !p_tagged {
            return Err(Rejected::NotAddressedToThisNode);
        }

        if grants.resolve(&event.pubkey).is_err() {
            return Err(Rejected::NoGrant);
        }

        let types: Vec<String> =
            serde_json::from_str(&event.content).map_err(|_| Rejected::Unparseable)?;
        let created_at = event.created_at.as_secs();
        let controller = event.pubkey.to_hex();

        if let Some(existing) = self.by_controller.get(&controller) {
            if created_at < existing.created_at {
                return Err(Rejected::Superseded);
            }
        }

        let n = types.len();
        if n == 0 {
            // An empty array unsubscribes from everything.
            self.by_controller.remove(&controller);
        } else {
            self.by_controller.insert(controller, Wanted { types, created_at });
        }
        Ok(n)
    }

    /// Whether a controller should receive a notification type **now**.
    ///
    /// The intersection: it must have asked for the type, and its grant
    /// must permit it. A grant narrowed after the subscription was
    /// published stops delivery at once — the subscription event, which the
    /// node does not own and cannot delete, simply becomes inert. A
    /// subscription that outlived its grant would be a revocation that does
    /// not revoke.
    pub fn should_receive(
        &self,
        controller: &PublicKey,
        notification_type: &str,
        grants: &Grants,
    ) -> bool {
        let Some(wanted) = self.by_controller.get(&controller.to_hex()) else {
            return false;
        };
        if !wanted.types.iter().any(|t| t == notification_type) {
            return false;
        }
        grants
            .resolve(controller)
            .map(|g| g.profile().allows_notification(notification_type))
            .unwrap_or(false)
    }

    /// Every controller that should receive this notification type now.
    pub fn recipients(&self, notification_type: &str, grants: &Grants) -> Vec<PublicKey> {
        self.by_controller
            .keys()
            .filter_map(|hex| PublicKey::from_hex(hex).ok())
            .filter(|pk| self.should_receive(pk, notification_type, grants))
            .collect()
    }

    /// How many controllers hold a subscription.
    pub fn len(&self) -> usize {
        self.by_controller.len()
    }
    /// Whether any controller holds a subscription.
    pub fn is_empty(&self) -> bool {
        self.by_controller.is_empty()
    }
}
