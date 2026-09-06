//! Request and response types, one pair per method.
//!
//! Written against **published NIP-47 core** for the five core methods and
//! against `nwc-onchain.md` for `pay_onchain` — not against any
//! implementation, and not against our forked `47.md`, which mission 18.2
//! realigns.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use super::types::*;

// ── pay_invoice ──────────────────────────────────────────────────────

/// `pay_invoice` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayInvoiceRequest {
    /// A BOLT11 invoice.
    pub invoice: String,
    /// Amount in msats, for an invoice that names none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u64>,
    /// Payer metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// `pay_invoice` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayInvoiceResponse {
    /// The preimage, which is the proof of payment.
    pub preimage: String,
    /// Fees in msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fees_paid: Option<u64>,
}

// ── make_invoice ─────────────────────────────────────────────────────

/// `make_invoice` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MakeInvoiceRequest {
    /// Value in msats.
    pub amount: u64,
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Description hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_hash: Option<String>,
    /// Expiry in seconds from creation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry: Option<u64>,
    /// Payer metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// `make_invoice` response — the invoice it made.
pub type MakeInvoiceResponse = Transaction;

// ── lookup_invoice ───────────────────────────────────────────────────

/// `lookup_invoice` request.
///
/// One of the two is required, which the type cannot express: making them
/// an enum would refuse a request naming both, and the specification does
/// not. A service answers `BAD_REQUEST` when neither is present.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LookupInvoiceRequest {
    /// The payment hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_hash: Option<String>,
    /// Or the encoded invoice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice: Option<String>,
}

/// `lookup_invoice` response — what it found.
pub type LookupInvoiceResponse = Transaction;

// ── get_balance ──────────────────────────────────────────────────────

/// `get_balance` request. No parameters.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GetBalanceRequest {}

/// `get_balance` response.
///
/// **One field, as published core defines it.** Our forked `47.md` adds
/// `lightning_balance` and `onchain_balance_sats`; that is divergence
/// inside core and belongs in an extension, which is 18.2's to decide. A
/// wallet holding only on-chain funds reports them here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetBalanceResponse {
    /// The balance in msats.
    pub balance: u64,
}

// ── get_info ─────────────────────────────────────────────────────────

/// `get_info` request. No parameters.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GetInfoRequest {}

/// `get_info` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetInfoResponse {
    /// The node's alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    /// Its colour, as hex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Its pubkey, as hex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pubkey: Option<String>,
    /// `mainnet`, `testnet`, `signet` or `regtest`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    /// Chain tip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_height: Option<u64>,
    /// Chain tip hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<String>,
    /// What this connection may call.
    pub methods: Vec<String>,
    /// Which numbered extension specifications this connection supports.
    ///
    /// Absent while a wallet implements only core, and while an extension
    /// it implements has no number — `CONTRIBUTING.md` upstream says
    /// maintainers assign them, so `nwc-onchain.md` claims none and
    /// `pay_onchain` appears in `methods` alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
}

// ── pay_onchain (nwc-onchain.md) ─────────────────────────────────────

/// `pay_onchain` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayOnchainRequest {
    /// The destination address.
    pub address: String,
    /// Amount in **sats**, not msats — on-chain outputs have no
    /// sub-satoshi precision, so a msat amount would have values that
    /// cannot be paid.
    pub amount_sats: u64,
    /// Fee rate in sat/vB. The wallet picks one if absent, and MUST NOT
    /// exceed this if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feerate: Option<u64>,
}

/// `pay_onchain` response.
///
/// The transaction is **broadcast, not confirmed**. A client needing
/// confirmation watches the chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayOnchainResponse {
    /// The transaction id.
    pub txid: String,
    /// Fee paid, in sats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee_sats: Option<u64>,
}
