//! The method names NIP-47 defines, and the extensions we implement.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Every NWC method this crate knows, whether or not a given wallet serves
/// one.
///
/// **Exhaustive over published core plus the extensions we implement**, not
/// over what any one wallet serves. That is the same rule
/// [`nnc::Method`](crate::nnc::Method) follows, scoped to a modular
/// specification: since 2026-08-01 NIP-47 is a five-method core plus
/// numbered extension specifications, so "the specification" is core plus
/// whatever we have adopted.
///
/// Adopting an extension later **adds** variants. Nothing already here is
/// reinterpreted, which is what makes six methods now and twenty-five later
/// a sequence rather than a rewrite.
///
/// [`WalletMethod::Unknown`] means a method name never fails to parse — a
/// request naming a method from an extension we have not adopted is
/// answered `NOT_IMPLEMENTED` rather than failing to deserialise with
/// nowhere to put the error.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WalletMethod {
    // ── published NIP-47 core ────────────────────────────────────────
    /// Pay a BOLT11 invoice.
    PayInvoice,
    /// Create an invoice.
    MakeInvoice,
    /// Look one up.
    LookupInvoice,
    /// The wallet's balance.
    GetBalance,
    /// What the wallet is and does.
    GetInfo,

    // ── NWC-XX, On-chain Payments (`nwc-onchain.md`) ─────────────────
    /// Send an on-chain payment to an address.
    ///
    /// Ours, and proposed rather than adopted: it exists in no upstream
    /// specification. See `nwc-onchain.md`.
    PayOnchain,

    /// A method this implementation does not know.
    Unknown(String),
}

impl WalletMethod {
    /// Every method this crate knows. Excludes [`WalletMethod::Unknown`].
    pub const ALL: [WalletMethod; 6] = [
        WalletMethod::PayInvoice,
        WalletMethod::MakeInvoice,
        WalletMethod::LookupInvoice,
        WalletMethod::GetBalance,
        WalletMethod::GetInfo,
        WalletMethod::PayOnchain,
    ];

    /// Every method published NIP-47 core defines.
    ///
    /// Separate from [`ALL`](Self::ALL) because the distinction is real: a
    /// wallet serving only these is conforming and needs no `extensions`
    /// tag, and a test asserting we cover core must not be satisfied by an
    /// extension method.
    pub const CORE: [WalletMethod; 5] = [
        WalletMethod::PayInvoice,
        WalletMethod::MakeInvoice,
        WalletMethod::LookupInvoice,
        WalletMethod::GetBalance,
        WalletMethod::GetInfo,
    ];

    /// Whether this method is defined by core rather than an extension.
    pub fn is_core(&self) -> bool {
        Self::CORE.contains(self)
    }

    /// Its wire spelling.
    pub fn as_str(&self) -> &str {
        match self {
            WalletMethod::PayInvoice => "pay_invoice",
            WalletMethod::MakeInvoice => "make_invoice",
            WalletMethod::LookupInvoice => "lookup_invoice",
            WalletMethod::GetBalance => "get_balance",
            WalletMethod::GetInfo => "get_info",
            WalletMethod::PayOnchain => "pay_onchain",
            WalletMethod::Unknown(s) => s,
        }
    }
}

impl fmt::Display for WalletMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WalletMethod {
    /// Parsing a method name cannot fail — an unknown one becomes
    /// [`WalletMethod::Unknown`].
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "pay_invoice" => WalletMethod::PayInvoice,
            "make_invoice" => WalletMethod::MakeInvoice,
            "lookup_invoice" => WalletMethod::LookupInvoice,
            "get_balance" => WalletMethod::GetBalance,
            "get_info" => WalletMethod::GetInfo,
            "pay_onchain" => WalletMethod::PayOnchain,
            other => WalletMethod::Unknown(other.to_string()),
        })
    }
}

impl Serialize for WalletMethod {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for WalletMethod {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s: String = String::deserialize(d)?;
        Ok(WalletMethod::from_str(&s).expect("infallible"))
    }
}
