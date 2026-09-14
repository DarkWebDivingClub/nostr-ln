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

impl serde::Serialize for WalletNotificationType {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for WalletNotificationType {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s: String = serde::Deserialize::deserialize(d)?;
        Ok(s.parse().expect("infallible"))
    }
}

/// A kind `23197` payload.
///
/// The wallet counterpart of [`crate::nnc::Notification`], and the same
/// shape: NIP-47 and NIP-XX agree on `notification_type` plus an untyped
/// payload, so a client reads the type and then reads the payload as what
/// the type says it is.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WalletNotification {
    /// Which notification.
    pub notification_type: WalletNotificationType,
    /// Its payload.
    pub notification: serde_json::Value,
}

impl WalletNotification {
    /// Build one with a typed payload.
    pub fn new<N: serde::Serialize>(
        notification_type: WalletNotificationType,
        notification: N,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self { notification_type, notification: serde_json::to_value(notification)? })
    }

    /// Read the payload as its type.
    pub fn as_typed<N: for<'de> serde::Deserialize<'de>>(
        &self,
    ) -> Result<N, serde_json::Error> {
        serde_json::from_value(self.notification.clone())
    }
}
