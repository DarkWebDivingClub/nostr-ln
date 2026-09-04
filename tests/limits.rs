//! NIP-XX, *How a bucket behaves*. Several of these are filed bugs.

use nostr_ln::{Bucket, RateLimitRule};

const COIN: u64 = 100_000_000;
const WEEK: u64 = 604_800;

fn rule(amount: u64, per_secs: u64, max_capacity: u64) -> RateLimitRule {
    RateLimitRule { amount, per_secs, max_capacity }
}

#[test]
fn a_bucket_starts_full() {
    let r = rule(1, 60, 10);
    assert_eq!(Bucket::full(&r, 0).balance_at(0, &r), 10);
}

#[test]
fn checking_does_not_mutate() {
    // Pipeline step 4 checks; step 7 consumes. A refused or failed request
    // must leave every counter untouched.
    let r = rule(1, 1, 100);
    let b = Bucket::full(&r, 0);
    let before = b.clone();
    for t in [0, 5, 50, 5_000] {
        let _ = b.balance_at(t, &r);
        let _ = b.can_withdraw(1, t, &r);
    }
    assert_eq!(b, before, "checking must not change the bucket");
}

#[test]
fn since_advances_on_withdrawal_and_at_no_other_time() {
    // dln-node#4: the timestamp is written in the constructor and never
    // again, so the interval before each withdrawal is credited afresh at
    // every later check and the bucket refills faster than its rule says.
    let r = rule(1, 1, 1_000);
    let mut b = Bucket::full(&r, 0);

    assert!(b.withdraw(1_000, 0, &r), "empty it");
    assert_eq!(b.balance_at(0, &r), 0);

    // Ten seconds later: exactly ten tokens, not ten plus the interval
    // before the withdrawal counted again.
    assert_eq!(b.balance_at(10, &r), 10);

    // Checking repeatedly must not accumulate.
    assert_eq!(b.balance_at(10, &r), 10);
    assert_eq!(b.balance_at(10, &r), 10);
}

#[test]
fn refill_is_continuous_not_periodic() {
    // A week's allowance of one coin, checked mid-period. A periodic
    // implementation would pay nothing until the boundary.
    let r = rule(COIN, WEEK, COIN);
    let mut b = Bucket::full(&r, 0);
    assert!(b.withdraw(COIN, 0, &r));

    assert_eq!(b.balance_at(WEEK / 2, &r), COIN / 2, "half a week, half a coin");
    assert_eq!(b.balance_at(WEEK, &r), COIN, "and a whole one after a week");
}

#[test]
fn a_realistic_rate_is_expressible_at_all() {
    // dln-node#5: rate_per_micro made one coin a week round to zero, so
    // every grant intending an allowance was silently a one-shot.
    let r = rule(COIN, WEEK, COIN);
    let mut b = Bucket::full(&r, 0);
    assert!(b.withdraw(COIN, 0, &r));
    assert_eq!(b.balance_at(1, &r), COIN / WEEK, "and it moves after one second");
    assert!(b.balance_at(1, &r) > 0);
}

#[test]
fn max_capacity_is_the_ceiling() {
    let r = rule(10, 1, 100);
    let mut b = Bucket::full(&r, 0);
    assert!(b.withdraw(100, 0, &r));
    assert_eq!(b.balance_at(1_000_000, &r), 100, "never more than max_capacity");
}

#[test]
fn absent_and_zero_amount_are_opposites() {
    // The contrast is asserted because they look alike and are not.
    // No rule at all is unlimited; {amount: 0, max_capacity: 1000} is a
    // thousand ever — the strictest thing expressible.
    let once = rule(0, 1, 1_000);
    let mut b = Bucket::full(&once, 0);
    assert!(b.withdraw(1_000, 0, &once));
    assert_eq!(b.balance_at(u32::MAX as u64, &once), 0, "never refills");
}

#[test]
fn amount_times_elapsed_does_not_overflow() {
    // u64 would wrap for a large allowance over a long interval.
    let r = rule(u64::MAX / 2, 1, u64::MAX);
    let b = Bucket::full(&r, 0);
    assert_eq!(b.balance_at(u64::MAX, &r), u64::MAX, "saturates rather than wrapping");

    let r2 = rule(COIN, 1, u64::MAX);
    let mut b2 = Bucket::full(&r2, 0);
    assert!(b2.withdraw(u64::MAX, 0, &r2));
    assert_eq!(b2.balance_at(u64::MAX, &r2), u64::MAX);
}

#[test]
fn a_failed_withdrawal_changes_nothing() {
    let r = rule(0, 1, 10);
    let mut b = Bucket::full(&r, 0);
    let before = b.clone();
    assert!(!b.withdraw(11, 0, &r), "cannot cover it");
    assert_eq!(b, before, "and must not partially spend");
}

#[test]
fn per_secs_zero_yields_nothing_rather_than_dividing_by_zero() {
    let bad = rule(1, 0, 100);
    assert!(!bad.is_valid());
    let b = Bucket::full(&bad, 0);
    assert_eq!(b.balance_at(1_000, &bad), 0);
    assert!(!b.can_withdraw(1, 1_000, &bad));
}

#[test]
fn defaults_match_the_spec() {
    let r: RateLimitRule = serde_json::from_str("{}").unwrap();
    assert_eq!(r.amount, 0, "default amount is 0");
    assert_eq!(r.per_secs, 1, "default per_secs is 1");
    assert_eq!(r.max_capacity, u64::MAX, "default max_capacity is u64::MAX");
}
