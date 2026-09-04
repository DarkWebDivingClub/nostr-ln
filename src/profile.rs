//! `UsageProfile` — what a grant says a controller may do.
//!
//! NIP-XX, *UsageProfile JSON*. Three permission maps and a quota:
//!
//! | | governs | one token is |
//! |---|---|---|
//! | `methods` | NWC methods (NIP-47), kind 23194 | one call |
//! | `control` | NNC methods (NIP-XX), kind 23198 | one call |
//! | `notifications` | notification types a controller may receive | — |
//! | `quota` | controller-wide spending | one satoshi |
//!
//! **Deny by default, everywhere.** Absent or empty grants nothing. The
//! three maps are separate so that a spending grant cannot authorise an
//! administrative call, or either authorise a notification — they are
//! structurally identical, and a single branch choosing between them would
//! eventually pick wrong.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::limit::RateLimitRule;

/// The key meaning "everything not named".
///
/// As a method key it is the rule for methods with no entry of their own.
/// As a controller — `d = <node_pubkey>:OTHERS` — it is the grant for keys
/// with no grant of their own, instantiated per controller.
pub const OTHERS: &str = "OTHERS";

/// What a grant says about one method.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodAccessRule {
    /// Per-method rate limit. Missing means no rate limit.
    ///
    /// Named `rate`, as the spec says. `dln-node` calls it `access_rate`,
    /// so a conforming grant deserialises there to no limit at all — the
    /// failure is silent and in the permissive direction, which is
    /// [dln-node#2](https://github.com/DarkWebDivingClub/dln-node/issues/2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<RateLimitRule>,
}

/// Why a call is not allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denied {
    /// No grant applies to this caller at all.
    Unauthorized,
    /// A grant applies, but does not permit this method.
    Restricted,
}

/// The permissions a grant carries.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageProfile {
    /// NWC methods. Absent or empty grants none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub methods: Option<HashMap<String, MethodAccessRule>>,
    /// NNC methods. Absent or empty grants none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control: Option<HashMap<String, MethodAccessRule>>,
    /// Notification types this controller may receive. Absent or empty
    /// permits none.
    ///
    /// A kind 30199 subscription says what a controller *wants*; this says
    /// what it *may have*. Delivery is the intersection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notifications: Option<HashMap<String, MethodAccessRule>>,
    /// Controller-wide spending quota. Absent means no quota.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota: Option<RateLimitRule>,
}

fn lookup(
    map: Option<&HashMap<String, MethodAccessRule>>,
    name: &str,
) -> Result<Option<RateLimitRule>, Denied> {
    let map = map.ok_or(Denied::Restricted)?;
    let rule = map
        .get(name)
        .or_else(|| map.get(OTHERS))
        .ok_or(Denied::Restricted)?;
    Ok(rule.rate.clone())
}

impl UsageProfile {
    /// Every numeric value in the profile is usable.
    ///
    /// `per_secs = 0` invalidates the whole profile — it MUST NOT fall back
    /// to a default, and MUST NOT leave an older grant standing. A caller
    /// under an invalid profile is denied `UNAUTHORIZED`, not `RESTRICTED`:
    /// the grant does not apply at all.
    pub fn is_valid(&self) -> bool {
        let rules_ok = |m: &Option<HashMap<String, MethodAccessRule>>| {
            m.as_ref().map_or(true, |m| {
                m.values()
                    .all(|r| r.rate.as_ref().map_or(true, |r| r.is_valid()))
            })
        };
        rules_ok(&self.methods)
            && rules_ok(&self.control)
            && rules_ok(&self.notifications)
            && self.quota.as_ref().map_or(true, |q| q.is_valid())
    }

    /// The rate rule for an NWC method, if it is permitted at all.
    ///
    /// `Ok(None)` means permitted with no rate limit.
    pub fn allows_wallet(&self, method: &str) -> Result<Option<RateLimitRule>, Denied> {
        self.guard()?;
        lookup(self.methods.as_ref(), method)
    }

    /// The same, for an NNC control method.
    pub fn allows_control(&self, method: &str) -> Result<Option<RateLimitRule>, Denied> {
        self.guard()?;
        lookup(self.control.as_ref(), method)
    }

    /// Whether this controller may receive a notification type.
    pub fn allows_notification(&self, notification_type: &str) -> bool {
        self.guard().is_ok() && lookup(self.notifications.as_ref(), notification_type).is_ok()
    }

    /// Revocation is an empty grant: no permissions of any kind.
    pub fn is_empty(&self) -> bool {
        let none = |m: &Option<HashMap<String, MethodAccessRule>>| {
            m.as_ref().map_or(true, |m| m.is_empty())
        };
        none(&self.methods) && none(&self.control) && none(&self.notifications)
    }

    fn guard(&self) -> Result<(), Denied> {
        if self.is_valid() {
            Ok(())
        } else {
            Err(Denied::Unauthorized)
        }
    }
}
