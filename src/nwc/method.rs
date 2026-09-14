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

    // ── NWC-03, Hold Invoices ────────────────────────────────────────
    /// Create a hold invoice for a payment hash generated elsewhere.
    ///
    /// The caller supplies the **hash**, not the preimage: the secret
    /// belongs to whoever made it, which is what lets a hold invoice lock
    /// to a payment someone else will settle.
    MakeHoldInvoice,
    /// Settle one, with the preimage.
    SettleHoldInvoice,
    /// Cancel one, releasing the payer's funds.
    CancelHoldInvoice,

    // ── NWC-XX, Payment Quotation (`nwc-route.md`) ───────────────────
    /// What a payment would cost, without sending it.
    ///
    /// Ours, and proposed rather than adopted. See `nwc-route.md`.
    QuotePayment,

    // ── NWC-04, Keysend; NWC-05, Transaction History ─────────────────
    /// Send a spontaneous payment to a public key. NWC-04.
    PayKeysend,
    /// Payment history. NWC-05.
    ListTransactions,

    // ── Ours, drafted in `nips` ──────────────────────────────────────
    /// Generate a receiving address. See `nwc-onchain.md`.
    MakeNewAddress,
    /// What an address has received. See `nwc-onchain.md`.
    LookupAddress,
    /// Addresses this wallet generated. See `nwc-onchain.md`.
    ListAddresses,
    /// Fee rates by confirmation target. See `nwc-onchain.md`.
    EstimateOnchainFees,
    /// Invoice entities with state. See `nwc-invoices.md`.
    ListInvoices,
    /// Pay one instruction from a BIP-321 URI. NWC-321.
    Pay,
    /// Generate a BIP-321 URI. NWC-321.
    Receive,

    // ── NWC-12, BOLT12 Offers; NWC-09, Payment Lookup ────────────────
    /// Create a BOLT12 offer. NWC-12.
    MakeOffer,
    /// Look up one payment record. NWC-09.
    LookupPayment,

    // ── `nwc-offers.md`, what NWC-12 does not cover ──────────────────
    /// Pay a BOLT12 offer. See `nwc-offers.md`.
    PayOffer,
    /// Offers this wallet created. See `nwc-offers.md`.
    ListOffers,
    /// Stop an offer accepting payments. See `nwc-offers.md`.
    DisableOffer,

    /// A method this implementation does not know.
    Unknown(String),
}

impl WalletMethod {
    /// Every method this crate knows. Excludes [`WalletMethod::Unknown`].
    pub const ALL: [WalletMethod; 24] = [
        WalletMethod::PayInvoice,
        WalletMethod::MakeInvoice,
        WalletMethod::LookupInvoice,
        WalletMethod::GetBalance,
        WalletMethod::GetInfo,
        WalletMethod::PayOnchain,
        WalletMethod::MakeHoldInvoice,
        WalletMethod::SettleHoldInvoice,
        WalletMethod::CancelHoldInvoice,
        WalletMethod::QuotePayment,
        WalletMethod::PayKeysend,
        WalletMethod::ListTransactions,
        WalletMethod::MakeNewAddress,
        WalletMethod::LookupAddress,
        WalletMethod::ListAddresses,
        WalletMethod::EstimateOnchainFees,
        WalletMethod::ListInvoices,
        WalletMethod::Pay,
        WalletMethod::Receive,
        WalletMethod::MakeOffer,
        WalletMethod::LookupPayment,
        WalletMethod::PayOffer,
        WalletMethod::ListOffers,
        WalletMethod::DisableOffer,
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
            WalletMethod::MakeHoldInvoice => "make_hold_invoice",
            WalletMethod::SettleHoldInvoice => "settle_hold_invoice",
            WalletMethod::CancelHoldInvoice => "cancel_hold_invoice",
            WalletMethod::QuotePayment => "quote_payment",
            WalletMethod::PayKeysend => "pay_keysend",
            WalletMethod::ListTransactions => "list_transactions",
            WalletMethod::MakeNewAddress => "make_new_address",
            WalletMethod::LookupAddress => "lookup_address",
            WalletMethod::ListAddresses => "list_addresses",
            WalletMethod::EstimateOnchainFees => "estimate_onchain_fees",
            WalletMethod::ListInvoices => "list_invoices",
            WalletMethod::Pay => "pay",
            WalletMethod::Receive => "receive",
            WalletMethod::MakeOffer => "make_offer",
            WalletMethod::LookupPayment => "lookup_payment",
            WalletMethod::PayOffer => "pay_offer",
            WalletMethod::ListOffers => "list_offers",
            WalletMethod::DisableOffer => "disable_offer",
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
            "make_hold_invoice" => WalletMethod::MakeHoldInvoice,
            "settle_hold_invoice" => WalletMethod::SettleHoldInvoice,
            "cancel_hold_invoice" => WalletMethod::CancelHoldInvoice,
            "quote_payment" => WalletMethod::QuotePayment,
            "pay_keysend" => WalletMethod::PayKeysend,
            "list_transactions" => WalletMethod::ListTransactions,
            "make_new_address" => WalletMethod::MakeNewAddress,
            "lookup_address" => WalletMethod::LookupAddress,
            "list_addresses" => WalletMethod::ListAddresses,
            "estimate_onchain_fees" => WalletMethod::EstimateOnchainFees,
            "list_invoices" => WalletMethod::ListInvoices,
            "pay" => WalletMethod::Pay,
            "receive" => WalletMethod::Receive,
            "make_offer" => WalletMethod::MakeOffer,
            "lookup_payment" => WalletMethod::LookupPayment,
            "pay_offer" => WalletMethod::PayOffer,
            "list_offers" => WalletMethod::ListOffers,
            "disable_offer" => WalletMethod::DisableOffer,
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
