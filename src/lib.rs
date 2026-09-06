//! The service side of NNC ([NIP-XX]) and NWC ([NIP-47]).
//!
//! A node implements a handler; this crate owns everything between a relay
//! and that handler — decoding, grants, limits, dispatch and the info
//! events. Mission 13.1 builds the access layer: what a grant says, who is
//! allowed to say it, and what is left.
//!
//! **The checks cannot be skipped.** A [`VerifiedGrant`] cannot be
//! constructed without the owner set and the node's own pubkey, so a
//! consumer cannot forget the check that
//! [dln-node#1](https://github.com/DarkWebDivingClub/dln-node/issues/1)
//! forgot — the type it needs does not exist until the check has run. And
//! absent configuration makes that type unconstructable rather than
//! permissive: an empty owner list denies everything.
//!
//! [NIP-XX]: https://github.com/DarkWebDivingClub/nips/blob/master/XX.md
//! [NIP-47]: https://github.com/nostr-protocol/nips/blob/master/47.md

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod grant;
pub mod nnc;
pub mod service;

/// Generates `methods()` from an impl block. See [`nostr_ln_macros::service`].
pub use nostr_ln_macros::service;
pub mod limit;
pub mod profile;
pub mod subscription;

pub use grant::{Grants, VerifiedGrant, GRANT_KIND};
pub use nnc::{Method, Notification, NotificationType, Request, Response};
pub use service::{Caller, ControlService, Handler, Prepared, Usage, WalletService};
pub use limit::{Bucket, RateLimitRule};
pub use profile::{Denied, MethodAccessRule, UsageProfile, OTHERS};
pub use subscription::{Subscriptions, SUBSCRIPTION_KIND};
