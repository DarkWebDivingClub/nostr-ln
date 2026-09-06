//! Lightning domain types shared across NIP-XX methods.
//!
//! Defined here rather than shared with `nip47.rs`'s near-equivalents.
//! Sharing would couple two protocols' **wire formats**: a field added to
//! an NWC type for an NWC reason would silently change what NNC sends, and
//! the two specifications are not revised together.

use serde::{Deserialize, Serialize};

/// A channel's state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelState {
    /// Usable.
    Active,
    /// Known but not usable.
    Inactive,
    /// Funding not yet confirmed.
    PendingOpen,
    /// Closing cooperatively.
    PendingClose,
    /// Closing unilaterally.
    ForceClosing,
    /// A state this implementation does not know.
    #[serde(untagged)]
    Unknown(String),
}

/// How a channel was closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseType {
    /// Both parties agreed.
    Cooperative,
    /// This node forced it.
    ForceLocal,
    /// **The peer forced it.** Follows from no command at all.
    ForceRemote,
    /// **The peer broadcast a revoked state.** Follows from no command, and
    /// is the event a monitoring controller most needs.
    Breach,
    /// A type this implementation does not know.
    #[serde(untagged)]
    Unknown(String),
}

/// A channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Channel {
    /// Implementation-specific channel id.
    pub id: String,
    /// Short channel id, once confirmed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_channel_id: Option<String>,
    /// The remote peer.
    pub peer_pubkey: String,
    /// Its state.
    ///
    /// Present in `list_channels`. **Absent in `channel_opened`**, where
    /// the notification's own definition — *"confirmed on-chain and is now
    /// active"* — makes it redundant. The specification shows it in one and
    /// not the other, so it is optional here rather than two near-identical
    /// structs that can drift apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<ChannelState>,
    /// Whether it is unannounced.
    #[serde(default)]
    pub is_private: bool,
    /// Total capacity, msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u64>,
    /// Our balance, msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_balance: Option<u64>,
    /// Their balance, msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_balance: Option<u64>,
    /// Funding transaction id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub funding_txid: Option<String>,
    /// Any further fields the specification carries.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A peer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Peer {
    /// Its pubkey.
    pub pubkey: String,
    /// Its address, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Whether a connection is up.
    #[serde(default)]
    pub connected: bool,
    /// Its announced alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    /// How many channels we have with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_channels: Option<u64>,
}

/// A channel's fee policy, as this node sets it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelFees {
    /// Channel id.
    pub id: String,
    /// Short channel id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_channel_id: Option<String>,
    /// The remote peer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer_pubkey: Option<String>,
    /// Base fee, msats.
    pub base_fee: u64,
    /// Proportional fee, millionths.
    pub fee_rate: u64,
    /// Smallest HTLC, msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_htlc: Option<u64>,
    /// Largest HTLC, msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_htlc: Option<u64>,
    /// Any further fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// One forwarded payment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForwardingEvent {
    /// Where it came in.
    pub incoming_channel_id: String,
    /// Where it went out.
    pub outgoing_channel_id: String,
    /// Amount in, msats.
    pub incoming_amount: u64,
    /// Amount out, msats.
    pub outgoing_amount: u64,
    /// What we kept, msats.
    pub fee_earned: u64,
    /// When, unix seconds.
    pub settled_at: u64,
}

/// An HTLC in flight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingHtlc {
    /// Which channel.
    pub channel_id: String,
    /// Incoming or outgoing.
    pub direction: String,
    /// Amount, msats.
    pub amount: u64,
    /// The payment hash.
    pub hash_lock: String,
    /// Its CLTV expiry.
    pub expiry_height: u64,
}

/// One hop of a route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hop {
    /// The node at this hop.
    pub pubkey: String,
    /// The channel used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_channel_id: Option<String>,
    /// Fee for this hop, msats.
    #[serde(default)]
    pub fee: u64,
    /// Any further fields.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A route to a destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    /// Total fee, msats.
    pub total_fee: u64,
    /// Total CLTV delta.
    pub total_time_lock: u64,
    /// The hops, in order.
    pub hops: Vec<Hop>,
}

/// A node in the network graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkNode {
    /// Its pubkey.
    pub pubkey: String,
    /// Its alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    /// Its colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// How many channels it announces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_channels: Option<u64>,
    /// Their total capacity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_capacity: Option<u64>,
    /// Its announced addresses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addresses: Option<Vec<String>>,
    /// When it last announced, unix seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_update: Option<u64>,
    /// Any further fields — `features` among them.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// One side's policy on a graph channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelPolicy {
    /// Base fee, msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_fee: Option<u64>,
    /// Proportional fee, millionths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee_rate: Option<u64>,
    /// Smallest HTLC, msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_htlc: Option<u64>,
    /// Largest HTLC, msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_htlc: Option<u64>,
    /// CLTV delta.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_lock_delta: Option<u64>,
    /// Whether this direction is disabled.
    #[serde(default)]
    pub disabled: bool,
    /// When it last updated, unix seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_update: Option<u64>,
}
