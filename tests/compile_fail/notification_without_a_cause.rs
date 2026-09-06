//! A notification with no cause must not be expressible.
//!
//! NIP-XX requires a notification to name what caused it. The event that
//! names nothing is the expensive one: it is byte-identical to a
//! legitimate subscription delivery, so a client cannot tell anything is
//! wrong — it waits for an outcome that already arrived and was discarded,
//! and reports a timeout. That cost twenty seconds of silence in 17.3,
//! from passing `None` where an id belonged.
//!
//! `Cause` has no variant for "nothing", so the state is unconstructable
//! rather than merely tested for — the same as `VerifiedGrant`, where the
//! type does not exist until the check has run.
//!
//! Only the type mismatch is asserted, and deliberately: an unknown-variant
//! error carries a `help:` line naming a path inside the local rustc build,
//! which would make this test pass or fail by toolchain rather than by the
//! property it is about.

use nostr_ln::service::Cause;

/// Stands for `Notifier::notify`, which takes a cause and not an option.
fn takes_a_cause(_: Cause) {}

fn main() {
    // The old shape: an `Option` whose `None` meant "no cause".
    let _: Cause = None;
    takes_a_cause(None);
}
