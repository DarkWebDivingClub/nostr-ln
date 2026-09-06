//! Access grants: kind `30198`, authored by an owner.
//!
//! NIP-XX moved grants off kind `30078` for a reason worth restating here,
//! because it is invisible until it bites: NIP-78 says relays *"SHOULD only
//! serve these events to the authenticated owner"*. A grant's author is the
//! **owner**; the reader that must have it is the **node service**. On a
//! relay implementing NIP-78 as written, a node could never fetch its own
//! grants, and authorization would fail closed for reasons nothing in the
//! protocol can diagnose.

use std::collections::HashMap;

use nostr::event::Event;
use nostr::key::PublicKey;

use crate::profile::{Denied, UsageProfile, OTHERS};

/// Kind of an access grant. Addressable: newest per `(kind, pubkey, d)`.
pub const GRANT_KIND: u16 = 30198;

/// A grant that has been checked: signed by an owner, addressed to this
/// node, parsed.
///
/// **It exists only as the output of [`Grants::apply`]**, which is the only
/// place those checks live.
/// [dln-node#1](https://github.com/DarkWebDivingClub/dln-node/issues/1)
/// happened because nothing forced the code to consult its owner list — it
/// was declared, written by a setter nobody called, and never read. A type
/// that cannot be built without the check is harder to forget than a rule
/// that must be remembered.
#[derive(Debug, Clone)]
pub struct VerifiedGrant {
    profile: UsageProfile,
    created_at: u64,
    owner: PublicKey,
}

impl VerifiedGrant {
    /// What this controller may do.
    pub fn profile(&self) -> &UsageProfile {
        &self.profile
    }
    /// When the grant was signed. Used for idempotency.
    pub fn created_at(&self) -> u64 {
        self.created_at
    }
    /// Which owner signed it.
    pub fn owner(&self) -> &PublicKey {
        &self.owner
    }
}

/// Why a grant event was not applied. Every variant is a refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejected {
    /// No owners are configured, so no grant can be trusted.
    ///
    /// Missing configuration fails **closed**. Treating "no owners" as "any
    /// owner" is dln-node#1.
    NoOwnersConfigured,
    /// Signed by a key that is not an owner.
    NotAnOwner,
    /// No `p` tag naming this node service.
    NotAddressedToThisNode,
    /// The `d` tag is absent, malformed, or names another node.
    WrongTarget,
    /// Older than the grant already applied for this controller.
    Superseded,
    /// The content is not a `UsageProfile`.
    Unparseable,
    /// Not a kind-`30198` event.
    ///
    /// Kind `30078` used to carry grants and no longer does. An event of
    /// the old kind is not a grant that needs migrating, it is not a grant.
    WrongKind,
}

/// The grants a node service has applied, by controller pubkey.
#[derive(Debug, Default)]
pub struct Grants {
    node: Option<PublicKey>,
    owners: Vec<PublicKey>,
    by_controller: HashMap<String, VerifiedGrant>,
    others: Option<VerifiedGrant>,
}

impl Grants {
    /// Configure the node's own pubkey and the owners whose grants it will
    /// accept.
    ///
    /// An empty `owners` is allowed to be constructed but accepts nothing:
    /// every [`Grants::apply`] returns [`Rejected::NoOwnersConfigured`].
    pub fn new(node: PublicKey, owners: Vec<PublicKey>) -> Self {
        Self { node: Some(node), owners, by_controller: HashMap::new(), others: None }
    }

    /// Apply a kind-`30198` event.
    ///
    /// Every check that makes a grant trustworthy is here, and all of them
    /// fail closed.
    pub fn apply(&mut self, event: &Event) -> Result<String, Rejected> {
        let node = self.node.ok_or(Rejected::NoOwnersConfigured)?;

        if event.kind.as_u16() != GRANT_KIND {
            return Err(Rejected::WrongKind);
        }
        if self.owners.is_empty() {
            return Err(Rejected::NoOwnersConfigured);
        }
        if !self.owners.contains(&event.pubkey) {
            return Err(Rejected::NotAnOwner);
        }

        // "The event tags MUST include a p tag with the node service's
        // pubkey." Relying on the relay subscription to filter is not the
        // same as checking.
        let node_hex = node.to_hex();
        let p_tagged = event
            .tags
            .iter()
            .any(|t| tag_is(t, "p", &node_hex));
        if !p_tagged {
            return Err(Rejected::NotAddressedToThisNode);
        }

        let controller = target(&node_hex, event).ok_or(Rejected::WrongTarget)?;
        let created_at = event.created_at.as_secs();

        if let Some(existing) = self.get_raw(&controller) {
            if created_at < existing.created_at {
                return Err(Rejected::Superseded);
            }
        }

        // A grant that does not parse denies. It must not leave an older,
        // more generous grant standing.
        let profile = serde_json::from_str::<UsageProfile>(&event.content).unwrap_or_default();
        let parsed_ok = serde_json::from_str::<UsageProfile>(&event.content).is_ok();

        let verified = VerifiedGrant { profile, created_at, owner: event.pubkey };
        if controller == OTHERS {
            self.others = Some(verified);
        } else {
            self.by_controller.insert(controller.clone(), verified);
        }

        if parsed_ok {
            Ok(controller)
        } else {
            Err(Rejected::Unparseable)
        }
    }

    /// The grant that applies to a caller.
    ///
    /// The spec's step 1: the grant for this controller if one exists,
    /// otherwise the `OTHERS` grant, otherwise `UNAUTHORIZED`. **An
    /// explicit grant takes precedence**, including an explicit empty one —
    /// which is how a single controller is revoked while `OTHERS` stays in
    /// force.
    pub fn resolve(&self, controller: &PublicKey) -> Result<&VerifiedGrant, Denied> {
        self.by_controller
            .get(&controller.to_hex())
            .or(self.others.as_ref())
            .ok_or(Denied::Unauthorized)
    }

    /// How many controllers have a grant of their own.
    pub fn len(&self) -> usize {
        self.by_controller.len()
    }
    /// Whether any controller has a grant of their own.
    pub fn is_empty(&self) -> bool {
        self.by_controller.is_empty()
    }
    /// Whether an `OTHERS` grant is in force.
    pub fn has_others(&self) -> bool {
        self.others.is_some()
    }

    fn get_raw(&self, controller: &str) -> Option<&VerifiedGrant> {
        if controller == OTHERS {
            self.others.as_ref()
        } else {
            self.by_controller.get(controller)
        }
    }
}

fn tag_is(tag: &nostr::event::Tag, name: &str, value: &str) -> bool {
    let s = tag.as_slice();
    s.first().map(String::as_str) == Some(name) && s.get(1).map(String::as_str) == Some(value)
}

/// `d = <node_pubkey>:<controller_pubkey>`, and only if the first half is
/// us. A grant naming another node is not ours to apply.
fn target(node_hex: &str, event: &Event) -> Option<String> {
    let d = event.tags.iter().find_map(|t| {
        let s = t.as_slice();
        (s.first().map(String::as_str) == Some("d")).then(|| s.get(1).cloned()).flatten()
    })?;
    let (node, controller) = d.split_once(':')?;
    if node != node_hex || controller.is_empty() {
        return None;
    }
    Some(controller.to_string())
}
