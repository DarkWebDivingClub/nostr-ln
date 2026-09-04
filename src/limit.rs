//! Rate limits and quotas: the rule, and the bucket it governs.
//!
//! NIP-XX, *How a bucket behaves*. Three properties carry the weight, and
//! each is a bug that has been filed against a real implementation:
//!
//! - **Refill is continuous**, not periodic. A bucket does not pay out in a
//!   lump at each period boundary.
//! - **Checking MUST NOT mutate.** Limits are evaluated at pipeline step 4
//!   and consumed at step 7, so a refused or failed request charges
//!   nothing.
//! - **`since` advances on withdrawal and at no other time.** Leaving it
//!   behind re-credits the same interval at every later check — that is
//!   [dln-node#4]. Advancing it on a mere check discards the truncated
//!   remainder and the bucket leaks instead.
//!
//! [dln-node#4]: https://github.com/DarkWebDivingClub/dln-node/issues/4

use serde::{Deserialize, Serialize};

/// A refill rule.
///
/// One token is one call for a method rate, and one satoshi for a quota —
/// the rule itself does not know which.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimitRule {
    /// Tokens added per period. Default `0`, meaning no refill at all.
    #[serde(default)]
    pub amount: u64,
    /// The period in seconds. Default `1`.
    ///
    /// MUST be greater than zero. A rule with `per_secs = 0` invalidates the
    /// whole profile — see [`RateLimitRule::is_valid`] and NIP-XX's
    /// *Invalid values*.
    #[serde(default = "one")]
    pub per_secs: u64,
    /// The ceiling. Default `u64::MAX`.
    ///
    /// This is what separates a rate limit from an allowance: equal to
    /// `amount` and a controller can never bank more than one period's
    /// worth; higher and unused allowance accumulates up to it.
    #[serde(default = "u64_max")]
    pub max_capacity: u64,
}

fn one() -> u64 {
    1
}
fn u64_max() -> u64 {
    u64::MAX
}

impl Default for RateLimitRule {
    fn default() -> Self {
        Self { amount: 0, per_secs: 1, max_capacity: u64::MAX }
    }
}

impl RateLimitRule {
    /// `per_secs` must be greater than zero.
    ///
    /// Zero is a division by zero in the refill and there is no sensible
    /// substitute: treating it as `1` invents a rate nobody asked for, and
    /// treating it as "no refill" silently converts a rate limit into a
    /// one-off allowance. An invalid rule invalidates the profile carrying
    /// it, and the caller is denied.
    pub fn is_valid(&self) -> bool {
        self.per_secs > 0
    }
}

/// A live bucket. Starts full, at `max_capacity`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bucket {
    balance: u64,
    /// The instant `balance` was true. Advances on withdrawal only.
    since: u64,
}

impl Bucket {
    /// A full bucket, as of `now` (unix seconds).
    pub fn full(rule: &RateLimitRule, now: u64) -> Self {
        Self { balance: rule.max_capacity, since: now }
    }

    /// What the bucket holds at `now`. **Does not mutate.**
    ///
    /// `min(balance + amount × elapsed / per_secs, max_capacity)`, computed
    /// in `u128` because `amount × elapsed` exceeds 64 bits for a large
    /// allowance over a long interval.
    pub fn balance_at(&self, now: u64, rule: &RateLimitRule) -> u64 {
        if !rule.is_valid() {
            return 0;
        }
        let elapsed = now.saturating_sub(self.since) as u128;
        let added = (rule.amount as u128)
            .saturating_mul(elapsed)
            .saturating_div(rule.per_secs as u128);
        let filled = (self.balance as u128).saturating_add(added);
        filled.min(rule.max_capacity as u128) as u64
    }

    /// Whether `cost` could be taken at `now`. **Does not mutate.**
    pub fn can_withdraw(&self, cost: u64, now: u64, rule: &RateLimitRule) -> bool {
        self.balance_at(now, rule) >= cost
    }

    /// Take `cost`, and advance `since` to `now`.
    ///
    /// Returns `false` and changes nothing if the bucket cannot cover it,
    /// so a failed withdrawal is not a partial one.
    pub fn withdraw(&mut self, cost: u64, now: u64, rule: &RateLimitRule) -> bool {
        let available = self.balance_at(now, rule);
        if available < cost {
            return false;
        }
        self.balance = available - cost;
        // The line dln-node#4 is missing. Without it the interval before
        // this withdrawal is credited again at every later check.
        self.since = now;
        true
    }
}
