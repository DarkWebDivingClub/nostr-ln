//! Shapes shared between NWC messages.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Whether a transaction is money coming in or going out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    /// An invoice.
    Incoming,
    /// A payment.
    Outgoing,
}

/// Where a transaction has got to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionState {
    /// Not settled yet.
    Pending,
    /// Settled.
    Settled,
    /// Held, awaiting settlement. Hold invoices only — NWC-03.
    Accepted,
    /// An invoice that expired unpaid.
    Expired,
    /// A payment that failed.
    Failed,
    /// A state this implementation does not know.
    #[serde(untagged)]
    Unknown(String),
}

/// An invoice or a payment.
///
/// One type because `make_invoice` and `lookup_invoice` return the same
/// object; the specification writes it out twice and they agree except
/// that `make_invoice` has no `settled_at`, which is why that field is
/// optional here.
///
/// Fields the specification does not mark optional are still `Option` where
/// a wallet plainly may not have them — a lesson from `channel_opened`,
/// where a field present in the type and absent from the specification's
/// own example failed a generated vector and passed a hand-written round
/// trip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    /// Incoming or outgoing.
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    /// Where it has got to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<TransactionState>,
    /// The encoded invoice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoice: Option<String>,
    /// Its description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Its description hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_hash: Option<String>,
    /// The preimage, once known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preimage: Option<String>,
    /// The payment hash.
    pub payment_hash: String,
    /// Value in msats.
    pub amount: u64,
    /// Fees in msats.
    pub fees_paid: u64,
    /// When it was created.
    pub created_at: u64,
    /// When it expires, if it does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// When it settled, if it has.
    ///
    /// Absent from `make_invoice`'s response, which is the difference
    /// between the two messages this type serves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled_at: Option<u64>,
    /// Whatever the wallet attaches — zap details, a payer note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}
