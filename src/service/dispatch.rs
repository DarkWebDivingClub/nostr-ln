//! Where a method name becomes a typed call.
//!
//! The only place it happens, and the only place `NOT_IMPLEMENTED` is
//! returned for a name nothing implements.

use serde_json::Value;

use super::handler::{Caller, ControlService, WalletService};
use crate::nnc::{Method, NncError};

macro_rules! dispatch_arms {
    ($svc:ident, $method:ident, $params:ident, $caller:ident, $( $name:ident => $variant:ident ),* $(,)?) => {
        match $method {
            $(
                Method::$variant => {
                    let request = serde_json::from_value($params.clone())
                        .map_err(|e| NncError::new(crate::nnc::ErrorCode::Other, format!("bad params: {e}")))?;
                    let response = $svc.$name(request, $caller).await?;
                    serde_json::to_value(response)
                        .map_err(|e| NncError::new(crate::nnc::ErrorCode::Internal, e.to_string()))
                }
            )*
            Method::Unknown(name) => Err(NncError::not_implemented(name)),
        }
    };
}

/// Dispatch an NNC request to a [`ControlService`].
pub async fn dispatch_control(
    service: &dyn ControlService,
    method: &Method,
    params: &Value,
    caller: Caller<'_>,
) -> Result<Value, NncError> {
    dispatch_arms!(service, method, params, caller,
        list_channels => ListChannels,
        open_channel => OpenChannel,
        close_channel => CloseChannel,
        list_peers => ListPeers,
        connect_peer => ConnectPeer,
        disconnect_peer => DisconnectPeer,
        get_channel_fees => GetChannelFees,
        set_channel_fees => SetChannelFees,
        get_forwarding_history => GetForwardingHistory,
        get_pending_htlcs => GetPendingHtlcs,
        query_routes => QueryRoutes,
        list_network_nodes => ListNetworkNodes,
        get_network_stats => GetNetworkStats,
        get_network_node => GetNetworkNode,
        get_network_channel => GetNetworkChannel,
        sign_message => SignMessage,
    )
}

/// Dispatch an NWC request to a [`WalletService`].
pub async fn dispatch_wallet(
    service: &dyn WalletService,
    method: &str,
    params: &Value,
    caller: Caller<'_>,
) -> Result<Value, NncError> {
    service.call(method, params, caller).await
}
