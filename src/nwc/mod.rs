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

/// A notification a **wallet** service sends.
///
/// Parallel to [`crate::nnc::NotificationType`] rather than folded into it,
/// for the same reason [`WalletMethod`] is parallel to
/// [`crate::nnc::Method`]: they are different protocols, carried on
/// different kinds, and a type that spanned both would let a wallet
/// announce a node's notification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WalletNotificationType {
    /// A payer has locked in a hold invoice. NWC-03.
    ///
    /// **This is time-bounded.** The payload carries a `settle_deadline`
    /// block, after which settling or cancelling is no longer safe, and a
    /// recipient that sits on one holds somebody else's channel funds.
    HoldInvoiceAccepted,
    /// A type this implementation does not know.
    Unknown(String),
}

impl WalletNotificationType {
    /// Every type this crate knows.
    pub const ALL: [WalletNotificationType; 1] = [WalletNotificationType::HoldInvoiceAccepted];

    /// Its wire spelling.
    pub fn as_str(&self) -> &str {
        match self {
            Self::HoldInvoiceAccepted => "hold_invoice_accepted",
            Self::Unknown(s) => s,
        }
    }
}

impl std::fmt::Display for WalletNotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for WalletNotificationType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "hold_invoice_accepted" => Self::HoldInvoiceAccepted,
            other => Self::Unknown(other.to_string()),
        })
    }
}
