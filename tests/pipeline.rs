//! The pipeline, against a stub handler. No chain, no node, no relay.
//!
//! The order under test is **not** `XX.md`'s, which is defective — see
//! nostr-ln#1. These tests assert the corrected one, and several of them
//! fail under the specified order, which is the point.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use nostr::event::{EventBuilder, Kind, Tag};
use nostr::key::{Keys, PublicKey};
use nostr::types::Timestamp;
use nostr_ln::nnc::{methods::*, ErrorCode, Method, NncError};
use nostr_ln::service::*;
use nostr_ln::*;
use serde_json::{json, Value};

// ── a stub that records what it was asked ────────────────────────────

#[derive(Default)]
struct Stub {
    prepared: AtomicUsize,
    executed: AtomicUsize,
    cost: AtomicUsize,
    fail_execute: Mutex<Option<ErrorCode>>,
    seen: Mutex<Vec<String>>,
}

impl Stub {
    fn with_cost(cost: u64) -> Self {
        let s = Self::default();
        s.cost.store(cost as usize, Ordering::SeqCst);
        s
    }
    fn prepares(&self) -> usize {
        self.prepared.load(Ordering::SeqCst)
    }
    fn executes(&self) -> usize {
        self.executed.load(Ordering::SeqCst)
    }
}

#[nostr_ln::service]
impl ControlService for Stub {
    fn list_channels<'a>(
        &'a self,
        _r: ListChannelsRequest,
        _c: Caller<'a>,
    ) -> handler::Fut<'a, Result<ListChannelsResponse, NncError>> {
        Box::pin(async move {
            self.executed.fetch_add(1, Ordering::SeqCst);
            self.seen.lock().unwrap().push("list_channels".into());
            if let Some(code) = self.fail_execute.lock().unwrap().clone() {
                return Err(NncError::new(code, "stub was told to fail"));
            }
            Ok(ListChannelsResponse { channels: vec![] })
        })
    }

    fn open_channel<'a>(
        &'a self,
        _r: OpenChannelRequest,
        _c: Caller<'a>,
    ) -> handler::Fut<'a, Result<OpenChannelResponse, NncError>> {
        Box::pin(async move {
            self.executed.fetch_add(1, Ordering::SeqCst);
            self.seen.lock().unwrap().push("open_channel".into());
            if let Some(code) = self.fail_execute.lock().unwrap().clone() {
                return Err(NncError::new(code, "stub was told to fail"));
            }
            Ok(OpenChannelResponse {})
        })
    }

    fn prepare<'a>(
        &'a self,
        _method: &'a str,
        _params: &'a Value,
        _c: Caller<'a>,
    ) -> handler::Fut<'a, Result<Prepared, NncError>> {
        Box::pin(async move {
            self.prepared.fetch_add(1, Ordering::SeqCst);
            Ok(Prepared::new(self.cost.load(Ordering::SeqCst) as u64, ()))
        })
    }
}

// ── setup ────────────────────────────────────────────────────────────

struct World {
    node: Keys,
    owner: Keys,
    controller: Keys,
    grants: Grants,
    usage: Usage,
}

fn world(profile: &str) -> World {
    let node = Keys::generate();
    let owner = Keys::generate();
    let controller = Keys::generate();
    let mut grants = Grants::new(node.public_key(), vec![owner.public_key()]);
    let d = format!("{}:{}", node.public_key().to_hex(), controller.public_key().to_hex());
    let e = EventBuilder::new(Kind::Custom(GRANT_KIND), profile)
        .tags([
            Tag::identifier(d),
            Tag::parse(vec!["p".to_string(), node.public_key().to_hex()]).unwrap(),
        ])
        .custom_created_at(Timestamp::from(1))
        .sign_with_keys(&owner)
        .unwrap();
    grants.apply(&e).unwrap();
    World { node, owner, controller, grants, usage: Usage::new() }
}

async fn run(
    w: &mut World,
    stub: &Stub,
    method: Method,
    params: Value,
    now: u64,
) -> Result<Value, NncError> {
    let handler = Handler::Control(stub);
    nostr_ln::service::handle(
        &handler,
        &w.grants,
        &mut w.usage,
        &w.controller.public_key(),
        None,
        &method,
        &params,
        now,
    )
    .await
}

const FULL: &str = r#"{"control":{"OTHERS":{}}}"#;

// ── the macro ────────────────────────────────────────────────────────

#[test]
fn methods_are_generated_from_the_impl_block() {
    let stub = Stub::default();
    let mut m = stub.methods().to_vec();
    m.sort();
    assert_eq!(m, vec!["list_channels", "open_channel"]);
    assert!(
        !stub.methods().contains(&"prepare"),
        "prepare is a pipeline hook, not a method a controller can call"
    );
    assert!(
        !stub.methods().contains(&"get_network_stats"),
        "a node must not advertise what it does not implement"
    );
}

// ── the pipeline, step by step ───────────────────────────────────────

#[tokio::test]
async fn a_method_the_handler_does_not_implement_never_reaches_it() {
    let mut w = world(FULL);
    let stub = Stub::default();
    let e = run(&mut w, &stub, Method::GetNetworkStats, json!({}), 0).await.unwrap_err();
    assert_eq!(e.code, ErrorCode::NotImplemented);
    assert_eq!(stub.executes(), 0);
    assert_eq!(stub.prepares(), 0, "and nothing was prepared for it either");
}

#[tokio::test]
async fn a_caller_with_no_grant_is_unauthorized_and_learns_nothing_else() {
    let mut w = world(FULL);
    let stranger = Keys::generate();
    let stub = Stub::default();
    let handler = Handler::Control(&stub);
    let e = nostr_ln::service::handle(
        &handler, &w.grants, &mut w.usage, &stranger.public_key(),
        None,
        &Method::ListChannels, &json!({}), 0,
    )
    .await
    .unwrap_err();
    assert_eq!(e.code, ErrorCode::Unauthorized);
    assert_eq!(stub.executes(), 0);
}

#[tokio::test]
async fn authorization_precedes_validation() {
    // A caller with no grant sends malformed parameters. It must be told
    // UNAUTHORIZED, not that its parameters are wrong — validation messages
    // tell you which inputs are acceptable, and somebody with no grant
    // should not learn that.
    let mut w = world(FULL);
    let stranger = Keys::generate();
    let stub = Stub::default();
    let handler = Handler::Control(&stub);
    let e = nostr_ln::service::handle(
        &handler, &w.grants, &mut w.usage, &stranger.public_key(),
        None,
        &Method::OpenChannel, &json!({"nonsense": true}), 0,
    )
    .await
    .unwrap_err();
    assert_eq!(e.code, ErrorCode::Unauthorized, "not a params error");
}

#[tokio::test]
async fn a_restricted_method_never_reaches_the_handler() {
    let mut w = world(r#"{"control":{"list_channels":{}}}"#);
    let stub = Stub::default();
    let e = run(&mut w, &stub, Method::OpenChannel, json!({"pubkey":"02ab","amount_sats":1}), 0)
        .await
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::Restricted);
    assert_eq!(stub.executes(), 0);
}

#[tokio::test]
async fn malformed_parameters_are_refused_before_the_node_is_asked_to_prepare() {
    // A cost computed from unvalidated input is garbage, so validation
    // comes first — and the node is never asked to find a route for a
    // request that cannot be parsed.
    let mut w = world(FULL);
    let stub = Stub::default();
    let e = run(&mut w, &stub, Method::OpenChannel, json!({"amount_sats": "not a number"}), 0)
        .await
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::Other);
    assert_eq!(stub.prepares(), 0, "nothing was prepared");
    assert_eq!(stub.executes(), 0);
}

// ── limits ───────────────────────────────────────────────────────────

#[tokio::test]
async fn the_rate_limit_is_checked_before_the_node_prepares_anything() {
    // Preparing means asking the node to find a route or pick a feerate —
    // real work. A caller past its rate limit must not be able to provoke
    // it.
    let profile = r#"{"control":{"list_channels":{"rate":{"amount":0,"per_secs":1,"max_capacity":1}}}}"#;
    let mut w = world(profile);
    let stub = Stub::default();

    assert!(run(&mut w, &stub, Method::ListChannels, json!({}), 0).await.is_ok());
    assert_eq!(stub.prepares(), 1);

    let e = run(&mut w, &stub, Method::ListChannels, json!({}), 0).await.unwrap_err();
    assert_eq!(e.code, ErrorCode::RateLimited);
    assert_eq!(stub.prepares(), 1, "the second call prepared nothing");
    assert_eq!(stub.executes(), 1);
}

#[tokio::test]
async fn the_quota_is_checked_against_the_prepared_cost_not_the_request() {
    // The whole point of nostr-ln#1. The request says nothing about cost;
    // the node quotes 5000 sats, and a 1000-sat quota must refuse it.
    let profile = r#"{"control":{"OTHERS":{}},"quota":{"amount":0,"per_secs":1,"max_capacity":1000}}"#;
    let mut w = world(profile);
    let stub = Stub::with_cost(5000);

    let e = run(&mut w, &stub, Method::OpenChannel, json!({"pubkey":"02ab","amount_sats":1}), 0)
        .await
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::QuotaExceeded);
    assert_eq!(stub.prepares(), 1, "it had to prepare to learn the cost");
    assert_eq!(stub.executes(), 0, "and nothing moved");
}

#[tokio::test]
async fn a_spend_within_quota_proceeds_and_is_charged_the_quoted_amount() {
    let profile = r#"{"control":{"OTHERS":{}},"quota":{"amount":0,"per_secs":1,"max_capacity":1000}}"#;
    let mut w = world(profile);
    let stub = Stub::with_cost(600);

    assert!(run(&mut w, &stub, Method::OpenChannel, json!({"pubkey":"02ab","amount_sats":1}), 0)
        .await
        .is_ok());
    assert_eq!(stub.executes(), 1);

    // 600 charged, so a second 600 does not fit in 1000.
    let e = run(&mut w, &stub, Method::OpenChannel, json!({"pubkey":"02ab","amount_sats":1}), 0)
        .await
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::QuotaExceeded);
    assert_eq!(stub.executes(), 1, "the second never ran");
}

// ── commit ───────────────────────────────────────────────────────────

#[tokio::test]
async fn a_failed_execution_charges_nothing() {
    let profile = r#"{"control":{"OTHERS":{}},"quota":{"amount":0,"per_secs":1,"max_capacity":1000}}"#;
    let mut w = world(profile);
    let stub = Stub::with_cost(600);
    *stub.fail_execute.lock().unwrap() = Some(ErrorCode::Internal);

    let e = run(&mut w, &stub, Method::OpenChannel, json!({"pubkey":"02ab","amount_sats":1}), 0)
        .await
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::Internal);
    assert_eq!(stub.executes(), 1, "it did reach the handler");

    // Nothing charged: a second identical call still fits.
    *stub.fail_execute.lock().unwrap() = None;
    assert!(
        run(&mut w, &stub, Method::OpenChannel, json!({"pubkey":"02ab","amount_sats":1}), 0)
            .await
            .is_ok(),
        "a caller is never billed for something they did not receive"
    );
}

#[tokio::test]
async fn a_refused_request_charges_nothing() {
    let profile = r#"{"control":{"list_channels":{}},"quota":{"amount":0,"per_secs":1,"max_capacity":1000}}"#;
    let mut w = world(profile);
    let stub = Stub::with_cost(600);

    // Restricted before any limit is touched.
    assert!(run(&mut w, &stub, Method::OpenChannel, json!({"pubkey":"02ab","amount_sats":1}), 0)
        .await
        .is_err());
    // The quota is untouched, so a permitted spend of the whole thing works.
    assert!(run(&mut w, &stub, Method::ListChannels, json!({}), 0).await.is_ok());
}

// ── the two maps are separate ────────────────────────────────────────

#[tokio::test]
async fn a_wallet_grant_does_not_authorise_a_control_method() {
    let mut w = world(r#"{"methods":{"OTHERS":{}}}"#);
    let stub = Stub::default();
    let e = run(&mut w, &stub, Method::ListChannels, json!({}), 0).await.unwrap_err();
    assert_eq!(e.code, ErrorCode::Restricted, "spending does not make you an administrator");
    assert_eq!(stub.executes(), 0);
}
