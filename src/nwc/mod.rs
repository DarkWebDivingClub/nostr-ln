//! NIP-47, Nostr Wallet Connect.
//!
//! Published core is five methods; everything else is a numbered extension
//! specification in `github.com/nostr-wallet-connect/nwc`. This module
//! implements core plus the extensions we have adopted, and
//! [`WalletMethod`] is exhaustive over exactly that.
//!
//! The shape mirrors [`crate::nnc`] deliberately — the same
//! `method` / `methods` / `types` split — so that a reader who knows one
//! knows the other. NNC was originally modelled on NWC; this is the return
//! trip, with what NNC learned in between.

mod method;
pub mod methods;
pub mod types;

pub use method::WalletMethod;
