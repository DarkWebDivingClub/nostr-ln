//! The seven steps, run once for both protocols.
//!
//! **This is not the order `XX.md` gives**, which is defective — see
//! [nostr-ln#1](https://github.com/DarkWebDivingClub/nostr-ln/issues/1).
//! Building the specified order would reproduce a quota that under-counts
//! by the fee on every spending call.
//!
//! ```text
//! 1 decode      NIP-44, parse                       (13.3 supplies this)
//! 2 resolve     not in methods() -> NOT_IMPLEMENTED
//! 3 authorize   grant, else OTHERS, else UNAUTHORIZED
//! 4 validate    a cost from unvalidated parameters is garbage
//! 5 limits      a. rate     first, so that preparing is not free
//!               b. prepare  the node selects; nothing moves
//!               c. quota    absolute, against the prepared cost
//! 6 execute     the prepared operation
//! 7 commit      the quoted cost — the number that was checked
//! ```
//!
//! Two orderings carry weight beyond tidiness. **Authorize precedes
//! validate**, so a caller with no grant learns nothing about which
//! parameters are acceptable. **Rate precedes prepare**, so nobody can make
//! the node compute a thousand routes for free.

use serde_json::Value;

use super::handler::{Caller, ControlService, Prepared, WalletService};
use super::state::Usage;
use crate::nnc::{ErrorCode, Method, NncError};
use crate::{Denied, Grants};

use nostr::key::PublicKey;

/// Which protocol a request arrived on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// NIP-47, kind 23194. Checked against `methods`.
    Wallet,
    /// NIP-XX, kind 23198. Checked against `control`.
    Control,
}

/// What a node implements, for one request.
pub enum Handler<'a> {
    /// An NWC handler.
    Wallet(&'a dyn WalletService),
    /// An NNC handler.
    Control(&'a dyn ControlService),
}

impl Handler<'_> {
    fn protocol(&self) -> Protocol {
        match self {
            Handler::Wallet(_) => Protocol::Wallet,
            Handler::Control(_) => Protocol::Control,
        }
    }

    fn implements(&self, method: &str) -> bool {
        let methods = match self {
            Handler::Wallet(s) => s.methods(),
            Handler::Control(s) => s.methods(),
        };
        methods.contains(&method)
    }
}

/// Run one request through the pipeline.
///
/// Returns the result payload, or the error to put in the response. It
/// never panics on caller-supplied input: everything a controller sends is
/// input, not configuration.
pub async fn handle(
    handler: &Handler<'_>,
    grants: &Grants,
    usage: &mut Usage,
    controller: &PublicKey,
    method: &Method,
    params: &Value,
    now: u64,
) -> Result<Value, NncError> {
    let caller = Caller { controller };
    let name = method.as_str();

    // ── 2. resolve ───────────────────────────────────────────────────
    if !handler.implements(name) {
        return Err(NncError::not_implemented(name));
    }

    // ── 3. authorize ─────────────────────────────────────────────────
    // Before validate, so a caller with no grant learns nothing about
    // which parameters are acceptable.
    let grant = grants.resolve(controller).map_err(deny)?;
    let profile = grant.profile();
    let rule = match handler.protocol() {
        Protocol::Wallet => profile.allows_wallet(name),
        Protocol::Control => profile.allows_control(name),
    }
    .map_err(deny)?;

    // ── 4. validate ──────────────────────────────────────────────────
    // Deserialising the params into the method's type is the validation
    // the specification asks for; a cost computed from unvalidated input
    // would be garbage.
    validate(handler, method, params)?;

    // ── 5a. rate ─────────────────────────────────────────────────────
    // First, so that preparing — which makes the node find a route or pick
    // a feerate — cannot be provoked for free.
    if let Some(ref rule) = rule {
        if !usage.rate_allows(controller, name, rule, now) {
            return Err(NncError::new(
                ErrorCode::RateLimited,
                format!("{name} is rate limited for this controller"),
            ));
        }
    }

    // ── 5b. prepare ──────────────────────────────────────────────────
    // The node selects a route or a feerate and says what it will cost.
    // Nothing moves.
    let prepared = match handler {
        Handler::Wallet(s) => s.prepare(name, params, caller).await?,
        Handler::Control(s) => s.prepare(name, params, caller).await?,
    };

    // ── 5c. quota ────────────────────────────────────────────────────
    // Absolute, against the prepared cost rather than a figure guessed
    // from the request.
    if prepared.cost_sats > 0 {
        if let Some(quota) = profile.quota.as_ref() {
            if !usage.quota_allows(controller, prepared.cost_sats, quota, now) {
                return Err(NncError::new(
                    ErrorCode::QuotaExceeded,
                    format!(
                        "{} sats exceeds this controller's remaining quota",
                        prepared.cost_sats
                    ),
                ));
            }
        }
    }
    let cost = prepared.cost_sats;

    // ── 6. execute ───────────────────────────────────────────────────
    let result = match handler {
        Handler::Wallet(s) => super::dispatch::dispatch_wallet(*s, name, params, caller).await,
        Handler::Control(s) => super::dispatch::dispatch_control(*s, method, params, caller).await,
    };

    // ── 7. commit ────────────────────────────────────────────────────
    // Only on success, and the quoted cost — the number that was checked.
    // A refused or failed request charges nothing, so a caller is never
    // billed for something they did not receive.
    match result {
        Ok(value) => {
            if let Some(ref rule) = rule {
                usage.charge_rate(controller, name, rule, now);
            }
            if cost > 0 {
                if let Some(quota) = profile.quota.as_ref() {
                    usage.charge_quota(controller, cost, quota, now);
                }
            }
            Ok(value)
        }
        Err(e) => Err(e),
    }
}

/// Step 4: the parameters must deserialise into the method's own type.
fn validate(handler: &Handler<'_>, method: &Method, params: &Value) -> Result<(), NncError> {
    use crate::nnc::methods::*;
    fn check<T: for<'de> serde::Deserialize<'de>>(params: &Value) -> Result<(), NncError> {
        serde_json::from_value::<T>(params.clone())
            .map(|_| ())
            .map_err(|e| NncError::new(ErrorCode::Other, format!("bad params: {e}")))
    }
    if matches!(handler.protocol(), Protocol::Wallet) {
        // NWC params are typed by the handler, which owns those types until
        // mission 18 brings them here.
        return Ok(());
    }
    match method {
        Method::ListChannels => check::<ListChannelsRequest>(params),
        Method::OpenChannel => check::<OpenChannelRequest>(params),
        Method::CloseChannel => check::<CloseChannelRequest>(params),
        Method::ListPeers => check::<ListPeersRequest>(params),
        Method::ConnectPeer => check::<ConnectPeerRequest>(params),
        Method::DisconnectPeer => check::<DisconnectPeerRequest>(params),
        Method::GetChannelFees => check::<GetChannelFeesRequest>(params),
        Method::SetChannelFees => check::<SetChannelFeesRequest>(params),
        Method::GetForwardingHistory => check::<GetForwardingHistoryRequest>(params),
        Method::GetPendingHtlcs => check::<GetPendingHtlcsRequest>(params),
        Method::QueryRoutes => check::<QueryRoutesRequest>(params),
        Method::ListNetworkNodes => check::<ListNetworkNodesRequest>(params),
        Method::GetNetworkStats => check::<GetNetworkStatsRequest>(params),
        Method::GetNetworkNode => check::<GetNetworkNodeRequest>(params),
        Method::GetNetworkChannel => check::<GetNetworkChannelRequest>(params),
        Method::SignMessage => check::<SignMessageRequest>(params),
        Method::Unknown(n) => Err(NncError::not_implemented(n)),
    }
}

fn deny(d: Denied) -> NncError {
    match d {
        Denied::Unauthorized => NncError::new(
            ErrorCode::Unauthorized,
            "no grant applies to this controller",
        ),
        Denied::Restricted => NncError::new(
            ErrorCode::Restricted,
            "this controller may not call that method",
        ),
    }
}

/// The unused `Prepared` selection is carried to `execute` by a node that
/// needs it; this crate only reads the cost.
const _: fn(Prepared) -> u64 = |p| p.cost_sats;
