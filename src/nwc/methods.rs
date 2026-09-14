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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetBalanceResponse {
    /// The balance in msats.
    pub balance: u64,
    /// Msats held in channels. **`nwc-onchain.md`.**
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lightning_balance: Option<u64>,
    /// Sats held on chain. **`nwc-onchain.md`.**
    ///
    /// Sats, not msats, and the name says so — the chain has no smaller
    /// unit. See `nwc-units.md`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onchain_balance_sats: Option<u64>,
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
    /// Which notifications this connection may receive. **NWC-02.**
    ///
    /// The `get_info` half of notification discovery; the info event's
    /// `notifications` tag is the other half, and a wallet should answer
    /// the same list in both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notifications: Option<Vec<String>>,
    /// Which numbered extension specifications this connection supports.
    ///
    /// Absent while a wallet implements only core, and while an extension
    /// it implements has no number — `CONTRIBUTING.md` upstream says
    /// maintainers assign them, so `nwc-onchain.md` claims none and
    /// `pay_onchain` appears in `methods` alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
    /// Which BIP-321 instruction types this wallet pays. **`nwc-bip321.md`.**
    ///
    /// The capability discovery a polymorphic `pay_bip321` cannot carry:
    /// implementing the method says nothing about which instructions the
    /// wallet will actually pay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bip321_methods: Option<Vec<Bip321Capability>>,
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
    /// Application-defined metadata. NWC-06.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
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
    /// Application-defined metadata. NWC-06.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

// ── NWC-04, Keysend Payments ────────────────────────────────────────────

/// A TLV record carried with a keysend payment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TlvRecord {
    /// TLV type.
    #[serde(rename = "type")]
    pub tlv_type: u64,
    /// Hex-encoded value.
    pub value: String,
}

/// `pay_keysend` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayKeysendRequest {
    /// Value in msats.
    pub amount: u64,
    /// The payee's public key.
    pub pubkey: String,
    /// The preimage, if the caller chose it.
    ///
    /// Keysend carries the preimage to the payee rather than deriving the
    /// payment from an invoice, so whoever supplies it decides what
    /// settles the payment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preimage: Option<String>,
    /// TLV records to carry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tlv_records: Option<Vec<TlvRecord>>,
}

/// `pay_keysend` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayKeysendResponse {
    /// The preimage of the completed payment.
    pub preimage: String,
    /// Routing fees in msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fees_paid: Option<u64>,
}

// ── NWC-05, Transaction History ─────────────────────────────────────────

/// `list_transactions` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListTransactionsRequest {
    /// Inclusive start, unix seconds. Defaults to 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<u64>,
    /// Inclusive end, unix seconds. Defaults to now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<u64>,
    /// How many to return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    /// Where to start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    /// Include unpaid invoices. Defaults to false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unpaid: Option<bool>,
    /// Restrict to one direction. Both when absent.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub transaction_type: Option<TransactionType>,
}

/// `list_transactions` response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListTransactionsResponse {
    /// Newest first.
    pub transactions: Vec<Transaction>,
    /// How many match the filters, ignoring pagination.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_count: Option<u64>,
}

// ── NWC-02, Notifications ───────────────────────────────────────────────

/// The `payment_received` notification payload. NWC-02.
///
/// The same record `make_invoice` and `lookup_invoice` return, which is
/// what the specification shows — a notification is the wallet saying a
/// transaction reached a state, not a different object.
pub type PaymentReceived = Transaction;

/// The `payment_sent` notification payload. NWC-02.
pub type PaymentSent = Transaction;

// ── `nwc-onchain.md`, addresses and fees ────────────────────────────────

/// `make_new_address` request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeNewAddressRequest {
    /// Which kind of address. The wallet chooses when absent.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub address_type: Option<String>,
}

/// `make_new_address` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeNewAddressResponse {
    /// The generated address.
    pub address: String,
    /// What was actually created.
    ///
    /// Present even when the request named no type, so a caller that asked
    /// for nothing still learns what it got.
    #[serde(rename = "type")]
    pub address_type: String,
}

/// One payment to an address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressTransaction {
    /// The transaction id.
    pub txid: String,
    /// Value in sats.
    pub amount_sats: u64,
    /// **Zero while unconfirmed.** An unconfirmed output is counted in
    /// `total_received_sats` and can still disappear.
    pub confirmations: u64,
    /// Unix seconds.
    pub timestamp: u64,
}

/// `lookup_address` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LookupAddressRequest {
    /// The address to look up.
    pub address: String,
}

/// `lookup_address` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LookupAddressResponse {
    /// The address.
    pub address: String,
    /// Its kind.
    #[serde(rename = "type")]
    pub address_type: String,
    /// Value in sats, **including unconfirmed**.
    pub total_received_sats: u64,
    /// What paid it.
    pub transactions: Vec<AddressTransaction>,
}

/// One entry of `list_addresses`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressRecord {
    /// The address.
    pub address: String,
    /// Its kind.
    #[serde(rename = "type")]
    pub address_type: String,
    /// Value in sats.
    pub total_received_sats: u64,
    /// When it was generated.
    pub created_at: u64,
}

/// `list_addresses` request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListAddressesRequest {
    /// How many to return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    /// Where to start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

/// `list_addresses` response.
///
/// What the wallet **generated**, not everything it can spend from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListAddressesResponse {
    /// The addresses.
    pub addresses: Vec<AddressRecord>,
}

/// `estimate_onchain_fees` request. Takes nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EstimateOnchainFeesRequest {}

/// `estimate_onchain_fees` response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EstimateOnchainFeesResponse {
    /// Confirmation target in blocks, to sat/vbyte.
    ///
    /// **Which targets appear is the wallet's choice**, so a caller reads
    /// the keys rather than assuming a set. One entry is a complete
    /// answer, not a degraded one — which is why this is a map and not a
    /// struct of named priorities.
    pub fees: std::collections::BTreeMap<String, f64>,
}

// ── `nwc-invoices.md` ───────────────────────────────────────────────────

/// `list_invoices` request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListInvoicesRequest {
    /// Inclusive start, unix seconds, filtering on **creation**.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<u64>,
    /// Inclusive end, unix seconds, filtering on **creation**.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<u64>,
    /// How many to return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    /// Where to start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    /// Restrict to one state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<InvoiceState>,
}

/// The state of an invoice entity.
///
/// Three, not [`TransactionState`]'s six: this lists invoices, and
/// `failed` is a thing that happens to a payment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvoiceState {
    /// Unpaid and not yet expired.
    Pending,
    /// Paid.
    Settled,
    /// Past `expires_at` and unpaid.
    Expired,
    /// A state this implementation does not know.
    #[serde(untagged)]
    Unknown(String),
}

/// One entry of `list_invoices`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceRecord {
    /// The encoded invoice.
    pub invoice: String,
    /// Its description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The payment hash.
    pub payment_hash: String,
    /// Value in msats.
    pub amount: u64,
    /// Where it has got to.
    pub state: InvoiceState,
    /// Present exactly when `state` is `settled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preimage: Option<String>,
    /// When it was created.
    pub created_at: u64,
    /// When it stops accepting payment.
    pub expires_at: u64,
    /// Present exactly when `state` is `settled`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled_at: Option<u64>,
}

/// `list_invoices` response.
///
/// **Not a payment history.** An invoice nobody paid did not happen, so it
/// is absent from `list_transactions` and present here, which is the
/// reason both exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListInvoicesResponse {
    /// The invoices.
    pub invoices: Vec<InvoiceRecord>,
}

// ── `nwc-bip321.md` ─────────────────────────────────────────────────────

/// `pay_bip321` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayBip321Request {
    /// The `bitcoin:` URI.
    pub uri: String,
}

/// `pay_bip321` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayBip321Response {
    /// Which instruction was used.
    ///
    /// **Required**, because the outcomes differ in kind: a Lightning
    /// payment is final on the preimage, an on-chain one still needs
    /// confirmations. A caller that assumed one and got the other would
    /// report success too early.
    pub payment_method: String,
    /// Present for `bolt11` and `bolt12`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preimage: Option<String>,
    /// Present for `onchain` and `sp`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub txid: Option<String>,
    /// Fees in msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fees_paid: Option<u64>,
}

/// One instruction to include in a generated URI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bip321Method {
    /// `bolt11`, `bolt12`, `sp` or `onchain`.
    pub method: String,
    /// Seconds, for `bolt11` and `bolt12`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry: Option<u64>,
    /// For `onchain`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_type: Option<String>,
}

/// `make_bip321` request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeBip321Request {
    /// Value in msats. Required for `bolt11`, which is skipped without it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u64>,
    /// Names the payee. Appears **only** in the URI's `label=`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The URI message, and the invoice and offer description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Ordered: the payee's preference. Everything available when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub methods: Option<Vec<Bip321Method>>,
}

/// `make_bip321` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeBip321Response {
    /// The generated URI.
    pub uri: String,
}

/// What instruction types a wallet can pay. `get_info`, `nwc-bip321.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bip321Capability {
    /// `bolt11`, `bolt12`, `sp` or `onchain`.
    pub method: String,
    /// For `onchain`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_types: Option<Vec<String>>,
}

// ── NWC-12, BOLT12 Offers ───────────────────────────────────────────────

/// `make_offer` request. NWC-12.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MakeOfferRequest {
    /// Msats per payment. **Absent or null means a variable-amount offer**,
    /// which is a deliberate request rather than a default — it is what a
    /// donation address is — and a wallet that cannot issue one must
    /// refuse rather than substitute an amount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u64>,
    /// Required by BOLT12 whenever `amount` is present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Human-readable issuer, encoded into the offer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    /// Ask that at most one payment settle. Wallet-enforced policy, **not**
    /// a property of the encoded offer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub single_use: Option<bool>,
    /// Absolute expiry, unix seconds. Must be in the future.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// Application-defined metadata. NWC-06.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// `make_offer` response. NWC-12.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MakeOfferResponse {
    /// The 32-byte BOLT12 offer id, lowercase hex. **This identifies the
    /// offer everywhere else**, not the encoded string.
    pub offer_id: String,
    /// The raw offer.
    pub offer: String,
    /// Msats, or null for a variable-amount offer.
    pub amount: Option<u64>,
    /// The encoded description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The encoded issuer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    /// Whether the wallet will let only one payment settle.
    pub single_use: bool,
    /// When it was created.
    pub created_at: u64,
    /// When it stops answering invoice requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
}

// ── NWC-09, Payment Lookup ──────────────────────────────────────────────

/// `lookup_payment` request. NWC-09.
///
/// Exactly one selector form: `transaction_id`; or the BOLT11 fields
/// `payment_hash` and/or `invoice`; or `payment_type` with `lookup`.
/// Mixing them is `BAD_REQUEST`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LookupPaymentRequest {
    /// The wallet-scoped id. Every implementation supports this one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
    /// BOLT11 compatibility selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_hash: Option<String>,
    /// BOLT11 compatibility selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice: Option<String>,
    /// Which extension's selectors `lookup` uses. Absent implies `bolt11`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_type: Option<String>,
    /// Extension-defined selector fields.
    ///
    /// Untyped because NWC-09 defines the envelope and each payment-type
    /// extension defines what goes in here — a closed type would refuse a
    /// payment type this crate has not adopted, which is the opposite of
    /// what the envelope is for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lookup: Option<Value>,
}

/// Where a payment record has got to. NWC-09.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentState {
    /// Exists, has not met the wallet's settlement policy.
    Pending,
    /// Funds committed, settlement deliberately held.
    Accepted,
    /// Final, under the wallet's policy.
    Settled,
    /// An outgoing attempt ended unsuccessfully.
    Failed,
    /// The request expired without settling.
    Expired,
    /// Cancelled.
    Canceled,
    /// A state this implementation does not know.
    #[serde(untagged)]
    Unknown(String),
}

/// `lookup_payment` response. NWC-09.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LookupPaymentResponse {
    /// Stable, wallet-scoped, and the same id other methods report.
    pub transaction_id: String,
    /// Incoming or outgoing.
    #[serde(rename = "type")]
    pub payment_direction: TransactionType,
    /// Where it has got to.
    pub state: PaymentState,
    /// `bolt11`, `bolt12`, or another extension's type.
    pub payment_type: String,
    /// Msats, whatever the payment type's native unit.
    pub amount: u64,
    /// Msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fees_paid: Option<u64>,
    /// When the wallet created the record.
    pub created_at: u64,
    /// Last material update.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
    /// When the payment expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// When it settled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled_at: Option<u64>,
    /// Required when `state` is `failed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    /// Application-defined metadata. NWC-06.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    /// Defined by the payment type. Untyped for the same reason `lookup`
    /// is: the envelope is generic and the extensions are not.
    pub details: Value,
}

// ── `nwc-offers.md`, what NWC-12 does not cover ─────────────────────────

/// `pay_offer` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayOfferRequest {
    /// The encoded offer.
    ///
    /// The offer itself, not an `offer_id`: paying is the one operation
    /// where the wallet did not create the object and has no id for it
    /// until it parses one.
    pub offer: String,
    /// Msats, required where the offer fixes no amount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u64>,
    /// Shown to the payee.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payer_note: Option<String>,
}

/// `pay_offer` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayOfferResponse {
    /// NWC-09's id, so the payer can reconcile this afterwards.
    pub transaction_id: String,
    /// The preimage of the completed payment.
    pub preimage: String,
    /// Msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fees_paid: Option<u64>,
}

/// One entry of `list_offers`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfferRecord {
    /// NWC-12's offer id.
    pub offer_id: String,
    /// The raw offer.
    pub offer: String,
    /// Its description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Its issuer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    /// Msats, or null for a variable-amount offer.
    pub amount: Option<u64>,
    /// False once disabled or expired. **Not a statement about the
    /// network** — the string is still valid and payers still hold it.
    pub active: bool,
    /// Whether the wallet lets only one payment settle.
    pub single_use: bool,
    /// **Settled** payments only.
    pub num_payments_received: u64,
    /// Msats, settled only.
    pub total_received: u64,
    /// When it was created.
    pub created_at: u64,
    /// When it expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
}

/// `list_offers` request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListOffersRequest {
    /// Defaults to **false**, so the unfiltered call is the complete one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_only: Option<bool>,
    /// How many to return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    /// Where to start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

/// `list_offers` response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListOffersResponse {
    /// The offers.
    pub offers: Vec<OfferRecord>,
}

/// `disable_offer` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisableOfferRequest {
    /// Which offer. NWC-12's id.
    pub offer_id: String,
}

/// `disable_offer` response. Empty.
///
/// **Not revocation.** The string is published; this makes the wallet stop
/// issuing invoices against it. Idempotent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisableOfferResponse {}
