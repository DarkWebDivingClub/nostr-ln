//! Proc macros for `nostr-ln`.
//!
//! A separate crate because Rust requires proc macros to be, and re-exported
//! by `nostr-ln` so consumers write `#[nostr_ln::service]` and never name
//! this one.
//!
//! Empty until [mission 13.2], which adds `#[service]`: it emits `methods()`
//! from the method names in an impl block, so a node cannot advertise
//! something it does not implement.
//!
//! [mission 13.2]: https://github.com/DarkWebDivingClub/nostr-ln
