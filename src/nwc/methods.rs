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

// ── NWC-03, Hold Invoices ───────────────────────────────────────────────

/// `make_hold_invoice` request.
///
/// The caller supplies a **payment hash**, never a preimage. The secret
/// belongs to whoever generated it — which is the whole point: a hold
/// invoice locks to a payment that someone else will settle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeHoldInvoiceRequest {
    /// Value in msats.
    pub amount: u64,
    /// The hash to lock to. Generated elsewhere.
    pub payment_hash: String,
    /// Invoice description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Hash of a description too long to carry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_hash: Option<String>,
    /// Seconds from creation within which a payment must be **initiated**.
    ///
    /// This is not how long the payment may be held — see
    /// `settle_deadline` on the acceptance notification, which is what
    /// bounds that.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry: Option<u64>,
    /// Minimum CLTV delta for the final hop.
    ///
    /// A caller that must settle this invoice only after completing some
    /// other payment sets this above that payment's own cost, so its
    /// claim outlives its obligation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_cltv_expiry_delta: Option<u32>,
}

/// `make_hold_invoice` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeHoldInvoiceResponse {
    /// Always `incoming` for an invoice.
    #[serde(rename = "type")]
    pub kind: String,
    /// The encoded invoice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice: Option<String>,
    /// The hash it locks to — the one the caller supplied.
    pub payment_hash: String,
    /// Value in msats.
    pub amount: u64,
    /// When it was created.
    pub created_at: u64,
    /// When it stops accepting a payment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// Invoice description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Hash of a description too long to carry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_hash: Option<String>,
}

/// `settle_hold_invoice` request.
///
/// Identified by the preimage alone — producing it *is* the authorisation,
/// and it determines which invoice is meant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettleHoldInvoiceRequest {
    /// The preimage. Producing it is the authorisation.
    pub preimage: String,
}

/// `settle_hold_invoice` response. Empty by specification.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettleHoldInvoiceResponse {}

/// `cancel_hold_invoice` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelHoldInvoiceRequest {
    /// Which invoice to cancel.
    pub payment_hash: String,
}

/// `cancel_hold_invoice` response. Empty by specification.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelHoldInvoiceResponse {}

// ── NWC-XX, Payment Quotation (`nwc-route.md`) ──────────────────────────

/// `quote_payment` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotePaymentRequest {
    /// The invoice that would be paid.
    pub invoice: String,
    /// Required only where the invoice carries no amount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u64>,
}

/// `quote_payment` response.
///
/// **An estimate, never a guarantee.** Gossip carries channel capacities
/// rather than balances, so a route that looks viable may fail, and a
/// later `pay_invoice` may cost more than this reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotePaymentResponse {
    /// Msats that would reach the destination.
    pub amount: u64,
    /// Routing fee the wallet expects to pay.
    pub fee_msat: u64,
    /// Blocks the route would consume, **excluding** the invoice's own
    /// final CLTV.
    pub cltv_expiry_delta: u32,
    /// Whether a route was found at all. `false` with the other fields
    /// zeroed is a result, not an error — "cannot reach" is what the
    /// caller asked.
    pub route_found: bool,
}

/// The `hold_invoice_accepted` notification payload. NWC-03.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldInvoiceAccepted {
    /// Always `incoming`.
    #[serde(rename = "type")]
    pub kind: String,
    /// `accepted`, where the wallet reports one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// The encoded invoice.
    pub invoice: String,
    /// The hash it locks to.
    pub payment_hash: String,
    /// Value in msats.
    pub amount: u64,
    /// When the invoice was created.
    pub created_at: u64,
    /// When the invoice stops accepting a payment.
    pub expires_at: u64,
    /// **The block by which this must be settled or cancelled.**
    ///
    /// Past it, neither is safe: the HTLC can expire on the payer's side
    /// while the recipient still believes it holds. A recipient waiting on
    /// something else — another payment, a counterparty — must finish
    /// before this, not merely intend to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settle_deadline: Option<u64>,
    /// Invoice description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Hash of a description too long to carry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_hash: Option<String>,
}
