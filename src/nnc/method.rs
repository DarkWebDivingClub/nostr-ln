//! The method names NIP-XX defines.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Every method NIP-XX defines, whether or not a given node serves one.
///
/// **Exhaustive over the specification, not over what we implement.** One
/// crate serves several nodes with different method sets, a client must be
/// able to name any method in order to discover `NOT_IMPLEMENTED`, and the
/// subset a node actually serves lives in its info event rather than here.
///
/// [`Method::Unknown`] means a method name never fails to parse — a request
/// naming something invented tomorrow is answered `NOT_IMPLEMENTED` rather
/// than failing to deserialise with nowhere to put the error.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Method {
    /// List the node's channels.
    ListChannels,
    /// Open a channel. **Asynchronous.**
    OpenChannel,
    /// Close a channel. **Asynchronous.**
    CloseChannel,
    /// List connected and known peers.
    ListPeers,
    /// Connect to a peer.
    ConnectPeer,
    /// Disconnect from a peer.
    DisconnectPeer,
    /// Read a channel's fee policy.
    GetChannelFees,
    /// Set a channel's fee policy.
    SetChannelFees,
    /// Forwarding history.
    GetForwardingHistory,
    /// HTLCs currently in flight.
    GetPendingHtlcs,
    /// Find routes to a destination.
    QueryRoutes,
    /// List nodes in the network graph.
    ListNetworkNodes,
    /// Aggregate network graph statistics.
    GetNetworkStats,
    /// One node from the network graph.
    GetNetworkNode,
    /// One channel from the network graph.
    GetNetworkChannel,
    /// Sign a message with the node's Lightning identity key.
    SignMessage,
    /// A method this implementation does not know.
    Unknown(String),
}

impl Method {
    /// Every method the specification defines. Excludes [`Method::Unknown`].
    pub const ALL: [Method; 16] = [
        Method::ListChannels,
        Method::OpenChannel,
        Method::CloseChannel,
        Method::ListPeers,
        Method::ConnectPeer,
        Method::DisconnectPeer,
        Method::GetChannelFees,
        Method::SetChannelFees,
        Method::GetForwardingHistory,
        Method::GetPendingHtlcs,
        Method::QueryRoutes,
        Method::ListNetworkNodes,
        Method::GetNetworkStats,
        Method::GetNetworkNode,
        Method::GetNetworkChannel,
        Method::SignMessage,
    ];

    /// Whether a command calling this method is acknowledged rather than
    /// completed, with the outcome arriving as a notification.
    ///
    /// Only `open_channel` and `close_channel`. Both wait on an on-chain
    /// confirmation, which is why they cannot answer synchronously — and a
    /// caller that reads their response as "done" has made the mistake the
    /// specification spends a paragraph on.
    pub fn is_asynchronous(&self) -> bool {
        matches!(self, Method::OpenChannel | Method::CloseChannel)
    }

    /// The notification this method's outcome arrives as, if it is
    /// asynchronous.
    pub fn notification(&self) -> Option<super::NotificationType> {
        match self {
            Method::OpenChannel => Some(super::NotificationType::ChannelOpened),
            Method::CloseChannel => Some(super::NotificationType::ChannelClosed),
            _ => None,
        }
    }

    /// Its wire spelling.
    pub fn as_str(&self) -> &str {
        match self {
            Method::ListChannels => "list_channels",
            Method::OpenChannel => "open_channel",
            Method::CloseChannel => "close_channel",
            Method::ListPeers => "list_peers",
            Method::ConnectPeer => "connect_peer",
            Method::DisconnectPeer => "disconnect_peer",
            Method::GetChannelFees => "get_channel_fees",
            Method::SetChannelFees => "set_channel_fees",
            Method::GetForwardingHistory => "get_forwarding_history",
            Method::GetPendingHtlcs => "get_pending_htlcs",
            Method::QueryRoutes => "query_routes",
            Method::ListNetworkNodes => "list_network_nodes",
            Method::GetNetworkStats => "get_network_stats",
            Method::GetNetworkNode => "get_network_node",
            Method::GetNetworkChannel => "get_network_channel",
            Method::SignMessage => "sign_message",
            Method::Unknown(s) => s,
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Method {
    /// Parsing a method name cannot fail — an unknown one becomes
    /// [`Method::Unknown`].
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "list_channels" => Method::ListChannels,
            "open_channel" => Method::OpenChannel,
            "close_channel" => Method::CloseChannel,
            "list_peers" => Method::ListPeers,
            "connect_peer" => Method::ConnectPeer,
            "disconnect_peer" => Method::DisconnectPeer,
            "get_channel_fees" => Method::GetChannelFees,
            "set_channel_fees" => Method::SetChannelFees,
            "get_forwarding_history" => Method::GetForwardingHistory,
            "get_pending_htlcs" => Method::GetPendingHtlcs,
            "query_routes" => Method::QueryRoutes,
            "list_network_nodes" => Method::ListNetworkNodes,
            "get_network_stats" => Method::GetNetworkStats,
            "get_network_node" => Method::GetNetworkNode,
            "get_network_channel" => Method::GetNetworkChannel,
            "sign_message" => Method::SignMessage,
            other => Method::Unknown(other.to_string()),
        })
    }
}

impl Serialize for Method {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Method {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s: String = String::deserialize(d)?;
        Ok(Method::from_str(&s).expect("infallible"))
    }
}
