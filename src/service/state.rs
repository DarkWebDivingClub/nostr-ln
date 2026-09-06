//! Live buckets, per controller.
//!
//! Checks never mutate; only [`Usage::charge_rate`] and
//! [`Usage::charge_quota`] do, and the pipeline calls them only after a
//! request has succeeded.

use std::collections::HashMap;

use nostr::key::PublicKey;

use crate::limit::{Bucket, RateLimitRule};

/// What each controller has spent.
///
/// A bucket is created full the first time a controller is seen, which is
/// the right default: a key nobody has met has spent nothing.
///
/// Under an `OTHERS` grant these are **per controller**, not shared. A
/// single shared bucket would let one caller starve every other.
#[derive(Debug, Default)]
pub struct Usage {
    rate: HashMap<(String, String), Bucket>,
    quota: HashMap<String, Bucket>,
}

impl Usage {
    /// Empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a call is within its rate limit. **Does not mutate.**
    pub fn rate_allows(
        &mut self,
        controller: &PublicKey,
        method: &str,
        rule: &RateLimitRule,
        now: u64,
    ) -> bool {
        let key = (controller.to_hex(), method.to_string());
        self.rate
            .entry(key)
            .or_insert_with(|| Bucket::full(rule, now))
            .can_withdraw(1, now, rule)
    }

    /// Whether a spend is within the quota. **Does not mutate.**
    pub fn quota_allows(
        &mut self,
        controller: &PublicKey,
        cost_sats: u64,
        rule: &RateLimitRule,
        now: u64,
    ) -> bool {
        self.quota
            .entry(controller.to_hex())
            .or_insert_with(|| Bucket::full(rule, now))
            .can_withdraw(cost_sats, now, rule)
    }

    /// Charge one call against a method's rate. Only after success.
    pub fn charge_rate(
        &mut self,
        controller: &PublicKey,
        method: &str,
        rule: &RateLimitRule,
        now: u64,
    ) -> bool {
        let key = (controller.to_hex(), method.to_string());
        self.rate
            .entry(key)
            .or_insert_with(|| Bucket::full(rule, now))
            .withdraw(1, now, rule)
    }

    /// Charge a spend against the quota. Only after success, and only the
    /// figure that was checked.
    pub fn charge_quota(
        &mut self,
        controller: &PublicKey,
        cost_sats: u64,
        rule: &RateLimitRule,
        now: u64,
    ) -> bool {
        self.quota
            .entry(controller.to_hex())
            .or_insert_with(|| Bucket::full(rule, now))
            .withdraw(cost_sats, now, rule)
    }

    /// How many controllers have been seen.
    pub fn controllers_seen(&self) -> usize {
        self.quota.len().max(
            self.rate
                .keys()
                .map(|(c, _)| c.clone())
                .collect::<std::collections::HashSet<_>>()
                .len(),
        )
    }
}
