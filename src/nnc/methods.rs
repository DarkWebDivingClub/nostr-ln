//! The sixteen NIP-XX methods, as request and response types.

use serde::{Deserialize, Serialize};

use super::types::*;

/// Methods whose request or response carries no fields.
///
/// `{}` on the wire, and a unit-like struct here rather than `()`, so each
/// one has a name that says which method it belongs to.
macro_rules! empty {
    ($($(#[$m:meta])* $name:ident),* $(,)?) => {$(
        $(#[$m])*
        #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name {}
    )*};
}

// ── Channel management ───────────────────────────────────────────────

empty! {
    /// `list_channels` takes no parameters.
    ListChannelsRequest,
}

/// The channels this node has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListChannelsResponse {
    /// Every channel.
    pub channels: Vec<Channel>,
}

/// Open a channel to a peer. **Asynchronous** — the response acknowledges,
/// and `channel_opened` carries the outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenChannelRequest {
    /// The peer.
    pub pubkey: String,
    /// Capacity, **sats**.
    pub amount_sats: u64,
    /// Pushed to the peer, msats.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub push_amount: Option<u64>,
    /// Whether it should be unannounced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,
    /// The peer's address, for auto-connect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// Cooperative close address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_address: Option<String>,
    /// Whether to send `channel_opened`. Defaults to **true**.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notify: Option<bool>,
}

/// Close a channel. **Asynchronous** — the response acknowledges, and
/// `channel_closed` carries the outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseChannelRequest {
    /// Which channel.
    pub id: String,
    /// Whether to force it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
    /// Where the balance should go.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_address: Option<String>,
    /// Whether to send `channel_closed`. Defaults to **true**.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notify: Option<bool>,
}

empty! {
    /// `open_channel` acknowledges with an empty result. **It does not mean
    /// the channel exists** — see `channel_opened`.
    OpenChannelResponse,
    /// `close_channel` acknowledges with an empty result. **It does not mean
    /// the channel is closed** — see `channel_closed`.
    CloseChannelResponse,
}

// ── Peer management ──────────────────────────────────────────────────

empty! {
    /// `list_peers` takes no parameters.
    ListPeersRequest,
    /// `connect_peer` acknowledges with an empty result.
    ConnectPeerResponse,
    /// `disconnect_peer` acknowledges with an empty result.
    DisconnectPeerResponse,
}

/// The peers this node knows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListPeersResponse {
    /// Every peer.
    pub peers: Vec<Peer>,
}

/// Connect to a peer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectPeerRequest {
    /// Its pubkey.
    pub pubkey: String,
    /// Its `host:port`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
}

/// Disconnect from a peer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisconnectPeerRequest {
    /// Its pubkey.
    pub pubkey: String,
}

// ── Fees and routing ─────────────────────────────────────────────────

/// Read fee policy. Omit `id` for every channel.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetChannelFeesRequest {
    /// One channel, or all of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// The fee policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetChannelFeesResponse {
    /// One entry per channel.
    pub fees: Vec<ChannelFees>,
}

/// Set fee policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetChannelFeesRequest {
    /// One channel, or all of them if omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
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
}

empty! {
    /// `set_channel_fees` acknowledges with an empty result.
    SetChannelFeesResponse,
    /// `get_pending_htlcs` takes no parameters.
    GetPendingHtlcsRequest,
}

/// Forwarding history, optionally windowed and paged.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetForwardingHistoryRequest {
    /// Inclusive start, unix seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<u64>,
    /// Inclusive end, unix seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<u64>,
    /// How many at most.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    /// Where to start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

/// What was forwarded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetForwardingHistoryResponse {
    /// One entry per forward.
    pub forwards: Vec<ForwardingEvent>,
}

/// HTLCs currently in flight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetPendingHtlcsResponse {
    /// One entry per HTLC.
    pub htlcs: Vec<PendingHtlc>,
}

/// Find routes to a destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryRoutesRequest {
    /// Where to.
    pub destination: String,
    /// How much, msats.
    pub amount: u64,
    /// How many routes at most. Defaults to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_routes: Option<u64>,
}

/// The routes found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryRoutesResponse {
    /// One entry per route.
    pub routes: Vec<Route>,
}

// ── Network graph ────────────────────────────────────────────────────

/// List graph nodes, paged.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListNetworkNodesRequest {
    /// How many at most.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    /// Where to start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

/// The graph nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListNetworkNodesResponse {
    /// One entry per node.
    pub nodes: Vec<NetworkNode>,
}

empty! {
    /// `get_network_stats` takes no parameters.
    GetNetworkStatsRequest,
}

/// Aggregate graph statistics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetNetworkStatsResponse {
    /// How many nodes.
    pub num_nodes: u64,
    /// How many channels.
    pub num_channels: u64,
    /// Their total capacity.
    pub total_capacity: u64,
    /// Mean channel size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg_channel_size: Option<u64>,
    /// Largest channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_channel_size: Option<u64>,
}

/// One node from the graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetNetworkNodeRequest {
    /// Which node.
    pub pubkey: String,
}

/// That node. Flattened, as the specification shows it.
pub type GetNetworkNodeResponse = NetworkNode;

/// One channel from the graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetNetworkChannelRequest {
    /// Which channel.
    pub short_channel_id: String,
}

/// That channel, with both sides' policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetNetworkChannelResponse {
    /// Its short channel id.
    pub short_channel_id: String,
    /// Its capacity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u64>,
    /// The lexicographically first node.
    pub node1_pubkey: String,
    /// The other.
    pub node2_pubkey: String,
    /// First node's policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node1_policy: Option<ChannelPolicy>,
    /// Second node's policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node2_policy: Option<ChannelPolicy>,
}

// ── Node identity ────────────────────────────────────────────────────

/// Sign a message with the node's **Lightning identity key**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignMessageRequest {
    /// What to sign. Caller-chosen, which is why the scheme prefixes it.
    pub message: String,
}

/// The signature.
///
/// `pubkey` is the Lightning **`node_id`** — 33 bytes compressed, 66 hex
/// characters beginning `02` or `03` — and **not** the Nostr service key,
/// which is 32 bytes x-only. See [`NodeId`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignMessageResponse {
    /// The message, echoed.
    pub message: String,
    /// zbase32 of a recoverable signature over
    /// `sha256d("Lightning Signed Message:" || message)`.
    pub signature: String,
    /// The signing `node_id`.
    pub pubkey: NodeId,
}

/// A Lightning node identity key.
///
/// A newtype rather than a `String`, because the specification warns that
/// this is confusable with the Nostr service key and they are different
/// lengths — 33 bytes compressed against 32 bytes x-only. A Nostr pubkey
/// cannot be assigned here by mistake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(String);

impl NodeId {
    /// Parse a `node_id`: 66 hex characters beginning `02` or `03`.
    pub fn parse(s: &str) -> Result<Self, InvalidNodeId> {
        let ok = s.len() == 66
            && (s.starts_with("02") || s.starts_with("03"))
            && s.bytes().all(|b| b.is_ascii_hexdigit());
        if ok {
            Ok(Self(s.to_string()))
        } else {
            Err(InvalidNodeId)
        }
    }
    /// Its hex form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A string that is not a Lightning `node_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidNodeId;

impl std::fmt::Display for InvalidNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("not a Lightning node_id: expected 66 hex characters beginning 02 or 03")
    }
}

impl std::error::Error for InvalidNodeId {}
