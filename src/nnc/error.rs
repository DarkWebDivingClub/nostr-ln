//! Error codes, shared by NIP-XX and NIP-47.
//!
//! One enum because the response envelope is one envelope. The two
//! specifications overlap on most of the list and each names a few of its
//! own, which is marked below.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The error codes the specifications name.
///
/// `Unknown` exists for the same reason [`super::Method::Unknown`] does: a
/// code invented after this was written must be reportable, not a parse
/// failure with nowhere to put the error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    /// The client is sending commands too fast.
    #[serde(rename = "RATE_LIMITED")]
    RateLimited,
    /// The method is not known or is intentionally not implemented.
    #[serde(rename = "NOT_IMPLEMENTED")]
    NotImplemented,
    /// This public key is not allowed to do this operation.
    #[serde(rename = "RESTRICTED")]
    Restricted,
    /// This public key has no node connected.
    #[serde(rename = "UNAUTHORIZED")]
    Unauthorized,
    /// The controller has exceeded its spending quota.
    #[serde(rename = "QUOTA_EXCEEDED")]
    QuotaExceeded,
    /// The requested channel, peer or node was not found.
    #[serde(rename = "NOT_FOUND")]
    NotFound,
    /// A channel operation could not be completed.
    #[serde(rename = "CHANNEL_FAILED")]
    ChannelFailed,
    /// A peer could not be reached.
    #[serde(rename = "CONNECTION_FAILED")]
    ConnectionFailed,
    /// The wallet does not have enough funds to cover the payment or a fee
    /// reserve. **NIP-47.**
    #[serde(rename = "INSUFFICIENT_BALANCE")]
    InsufficientBalance,
    /// The payment failed — a timeout, no route, or a broadcast refused.
    /// **NIP-47**, and named by `nwc-onchain.md`.
    #[serde(rename = "PAYMENT_FAILED")]
    PaymentFailed,
    /// The encryption of the request is not supported. **NIP-47.**
    #[serde(rename = "UNSUPPORTED_ENCRYPTION")]
    UnsupportedEncryption,
    /// A parameter is invalid. **`nwc-onchain.md`.**
    #[serde(rename = "BAD_REQUEST")]
    BadRequest,
    /// The address or instruction is for a different Bitcoin network.
    /// **`nwc-onchain.md`.**
    #[serde(rename = "UNSUPPORTED_NETWORK")]
    UnsupportedNetwork,
    /// An internal error.
    #[serde(rename = "INTERNAL")]
    Internal,
    /// Other error.
    #[serde(rename = "OTHER")]
    Other,
    /// A code this implementation does not know.
    #[serde(untagged)]
    Unknown(String),
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::RateLimited => "RATE_LIMITED",
            Self::NotImplemented => "NOT_IMPLEMENTED",
            Self::Restricted => "RESTRICTED",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::QuotaExceeded => "QUOTA_EXCEEDED",
            Self::NotFound => "NOT_FOUND",
            Self::ChannelFailed => "CHANNEL_FAILED",
            Self::ConnectionFailed => "CONNECTION_FAILED",
            Self::InsufficientBalance => "INSUFFICIENT_BALANCE",
            Self::PaymentFailed => "PAYMENT_FAILED",
            Self::UnsupportedEncryption => "UNSUPPORTED_ENCRYPTION",
            Self::BadRequest => "BAD_REQUEST",
            Self::UnsupportedNetwork => "UNSUPPORTED_NETWORK",
            Self::Internal => "INTERNAL",
            Self::Other => "OTHER",
            Self::Unknown(s) => s,
        };
        f.write_str(s)
    }
}

/// The `error` field of a response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NncError {
    /// The code.
    pub code: ErrorCode,
    /// A human-readable message.
    pub message: String,
}

impl NncError {
    /// An error with a code and a message.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }

    /// The response to a method this node does not implement.
    pub fn not_implemented(method: impl fmt::Display) -> Self {
        Self::new(ErrorCode::NotImplemented, format!("{method} is not implemented"))
    }
}

impl fmt::Display for NncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for NncError {}
