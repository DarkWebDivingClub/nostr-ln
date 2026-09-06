//! NIP-XX (NNC) as types.
//!
//! The shape `nip47.rs` established, applied to node control: a [`Method`]
//! owning its wire spelling, a [`Request`] carrying typed params, a
//! [`Response`] carrying a result or an error, and a [`Notification`].
//!
//! Two of the sixteen methods are **asynchronous** — `open_channel` and
//! `close_channel` are acknowledged, and their outcome arrives as a
//! notification. [`Method::is_asynchronous`] says which, and their response
//! types say so in their documentation, because a caller reading an
//! acknowledgement as a result is the mistake the specification spends a
//! paragraph on.

use serde::{Deserialize, Serialize};

pub mod error;
pub mod methods;
pub mod method;
pub mod types;
pub mod uri;

#[cfg(feature = "client")]
pub mod client;

pub use error::{ErrorCode, NncError};
pub use method::Method;
pub use methods::*;
pub use types::*;
pub use uri::{NodeControlUri, UriError};

#[cfg(feature = "client")]
pub use client::{NostrNodeControl, Pending};

/// A notification type. **Not a [`Method`]** — these cannot be called.
///
/// Separate from `Method` because `Method` is the set of things a caller
/// can invoke: it is what a grant's `control` map is keyed on, what
/// dispatch matches over, and what the info event advertises. A
/// notification in that enum would produce a grant permitting something
/// uncallable, an unreachable dispatch arm, and an info event offering a
/// method nobody can call.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NotificationType {
    /// A channel is confirmed and active.
    ChannelOpened,
    /// A channel close is confirmed. May follow from no command at all —
    /// see [`CloseType::ForceRemote`] and [`CloseType::Breach`].
    ChannelClosed,
    /// A type this implementation does not know.
    Unknown(String),
}

impl NotificationType {
    /// Every type the specification defines.
    pub const ALL: [NotificationType; 2] =
        [NotificationType::ChannelOpened, NotificationType::ChannelClosed];

    /// Its wire spelling.
    pub fn as_str(&self) -> &str {
        match self {
            Self::ChannelOpened => "channel_opened",
            Self::ChannelClosed => "channel_closed",
            Self::Unknown(s) => s,
        }
    }
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for NotificationType {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "channel_opened" => Self::ChannelOpened,
            "channel_closed" => Self::ChannelClosed,
            other => Self::Unknown(other.to_string()),
        })
    }
}

impl Serialize for NotificationType {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for NotificationType {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s: String = String::deserialize(d)?;
        Ok(s.parse().expect("infallible"))
    }
}

/// A kind `23198` payload.
///
/// `params` is untyped here because it cannot be typed until `method` is
/// read — the same two-stage parse `nip47.rs` performs. Use
/// [`Request::params_as`] once the method is known.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// Which method.
    pub method: Method,
    /// Its parameters.
    #[serde(default)]
    pub params: serde_json::Value,
}

impl Request {
    /// Build a request for a method with typed parameters.
    pub fn new<P: Serialize>(method: Method, params: P) -> Result<Self, serde_json::Error> {
        Ok(Self { method, params: serde_json::to_value(params)? })
    }

    /// Read the parameters as a method's request type.
    pub fn params_as<P: for<'de> Deserialize<'de>>(&self) -> Result<P, serde_json::Error> {
        serde_json::from_value(self.params.clone())
    }
}

/// A kind `23199` payload.
///
/// A result and an error are mutually exclusive: NIP-XX inherits NIP-47's
/// rule that a successful command's `error` field is null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    /// The method being answered.
    pub result_type: Method,
    /// The result, absent on failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// The error, absent on success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<NncError>,
}

impl Response {
    /// A successful response.
    pub fn ok<R: Serialize>(result_type: Method, result: R) -> Result<Self, serde_json::Error> {
        Ok(Self {
            result_type,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    /// A failed response.
    pub fn err(result_type: Method, error: NncError) -> Self {
        Self { result_type, result: None, error: Some(error) }
    }

    /// Read the result as a method's response type.
    ///
    /// `Err` if the response carried an error instead.
    pub fn result_as<R: for<'de> Deserialize<'de>>(&self) -> Result<R, ResultError> {
        match (&self.result, &self.error) {
            (_, Some(e)) => Err(ResultError::Failed(e.clone())),
            (Some(v), None) => {
                serde_json::from_value(v.clone()).map_err(ResultError::Malformed)
            }
            (None, None) => Err(ResultError::Empty),
        }
    }

    /// Whether this is an acknowledgement rather than a result.
    ///
    /// True for the asynchronous methods on success: the outcome has not
    /// happened yet and arrives as a notification.
    pub fn is_acknowledgement(&self) -> bool {
        self.error.is_none() && self.result_type.is_asynchronous()
    }
}

/// Why a result could not be read.
#[derive(Debug)]
pub enum ResultError {
    /// The response carried an error.
    Failed(NncError),
    /// It carried neither a result nor an error.
    Empty,
    /// The result did not match the expected type.
    Malformed(serde_json::Error),
}

impl std::fmt::Display for ResultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed(e) => write!(f, "{e}"),
            Self::Empty => f.write_str("response carried neither result nor error"),
            Self::Malformed(e) => write!(f, "result does not match its type: {e}"),
        }
    }
}

impl std::error::Error for ResultError {}

/// A kind `23200` payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    /// Which notification.
    pub notification_type: NotificationType,
    /// Its payload.
    pub notification: serde_json::Value,
}

impl Notification {
    /// Build one with a typed payload.
    pub fn new<N: Serialize>(
        notification_type: NotificationType,
        notification: N,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self { notification_type, notification: serde_json::to_value(notification)? })
    }

    /// Read the payload as its type.
    pub fn as_typed<N: for<'de> Deserialize<'de>>(&self) -> Result<N, serde_json::Error> {
        serde_json::from_value(self.notification.clone())
    }
}

/// `channel_opened` — a channel is confirmed and active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelOpened {
    /// The channel.
    #[serde(flatten)]
    pub channel: Channel,
}

/// `channel_closed` — a close is confirmed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelClosed {
    /// Channel id.
    pub id: String,
    /// Short channel id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_channel_id: Option<String>,
    /// The peer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer_pubkey: Option<String>,
    /// Capacity, msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u64>,
    /// The closing transaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closing_txid: Option<String>,
    /// How it closed. `ForceRemote` and `Breach` follow from no command.
    pub close_type: CloseType,
}
