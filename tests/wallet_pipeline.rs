//! The wallet half of the pipeline, against a stub. No node, no relay.
//!
//! [`tests/pipeline.rs`](pipeline.rs) proves the order for a controller.
//! This proves the claim mission 18.2 rests on: that a method from an
//! **extension** is not a special case. NWC-03's hold invoices are
//! resolved, authorised, limited and validated by the same steps as
//! `get_balance`, and nothing in the pipeline knows which specification a
//! method came from.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use nostr::event::{EventBuilder, Kind, Tag};
use nostr::key::Keys;
use nostr::types::Timestamp;
use nostr_ln::nnc::{ErrorCode, Method, NncError};
use nostr_ln::nwc::methods::*;
use nostr_ln::nwc::WalletMethod;
use nostr_ln::service::handler::Fut;
use nostr_ln::service::*;
use nostr_ln::*;
use serde_json::{json, Value};

/// A wallet that records what it was asked and can be told to fail.
#[derive(Default)]
struct Stub {
    executed: AtomicUsize,
    prepared: AtomicUsize,
    seen: Mutex<Vec<String>>,
}

impl Stub {
    fn executes(&self) -> usize {
        self.executed.load(Ordering::SeqCst)
    }
    fn prepares(&self) -> usize {
        self.prepared.load(Ordering::SeqCst)
    }
    fn ran(&self, what: &str) {
        self.executed.fetch_add(1, Ordering::SeqCst);
        self.seen.lock().unwrap().push(what.into());
    }
}

#[nostr_ln::service]
impl WalletService for Stub {
    fn get_balance<'a>(&'a self, _r: GetBalanceRequest, _c: Caller<'a>)
        -> Fut<'a, Result<GetBalanceResponse, NncError>>
    {
        Box::pin(async move {
            self.ran("get_balance");
            Ok(GetBalanceResponse { balance: 1 })
        })
    }

    fn make_hold_invoice<'a>(&'a self, r: MakeHoldInvoiceRequest, _c: Caller<'a>)
        -> Fut<'a, Result<MakeHoldInvoiceResponse, NncError>>
    {
        Box::pin(async move {
            self.ran("make_hold_invoice");
            Ok(MakeHoldInvoiceResponse {
                kind: "incoming".into(),
                invoice: Some("lnbcrt1...".into()),
                payment_hash: r.payment_hash,
                amount: r.amount,
                created_at: 1_700_000_000,
                expires_at: None,
                description: r.description,
                description_hash: r.description_hash,
            })
        })
    }

    fn settle_hold_invoice<'a>(&'a self, _r: SettleHoldInvoiceRequest, _c: Caller<'a>)
        -> Fut<'a, Result<SettleHoldInvoiceResponse, NncError>>
    {
        Box::pin(async move {
            self.ran("settle_hold_invoice");
            Ok(SettleHoldInvoiceResponse {})
        })
    }

    fn cancel_hold_invoice<'a>(&'a self, _r: CancelHoldInvoiceRequest, _c: Caller<'a>)
        -> Fut<'a, Result<CancelHoldInvoiceResponse, NncError>>
    {
        Box::pin(async move {
            self.ran("cancel_hold_invoice");
            Ok(CancelHoldInvoiceResponse {})
        })
    }

    fn quote_payment<'a>(&'a self, _r: QuotePaymentRequest, _c: Caller<'a>)
        -> Fut<'a, Result<QuotePaymentResponse, NncError>>
    {
        Box::pin(async move {
            self.ran("quote_payment");
            Ok(QuotePaymentResponse {
                amount: 1,
                fee_msat: 0,
                cltv_expiry_delta: 144,
                route_found: true,
            })
        })
    }

    fn prepare<'a>(&'a self, _m: &'a str, _p: &'a Value, _c: Caller<'a>)
        -> Fut<'a, Result<Prepared, NncError>>
    {
        Box::pin(async move {
            self.prepared.fetch_add(1, Ordering::SeqCst);
            Ok(Prepared::free())
        })
    }
}

/// A wallet that implements none of the extension.
struct CoreOnly;

#[nostr_ln::service]
impl WalletService for CoreOnly {
    fn get_balance<'a>(&'a self, _r: GetBalanceRequest, _c: Caller<'a>)
        -> Fut<'a, Result<GetBalanceResponse, NncError>>
    {
        Box::pin(async move { Ok(GetBalanceResponse { balance: 0 }) })
    }
}

struct World {
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
    World { controller, grants, usage: Usage::new() }
}

/// A wallet request, as the transport builds one: the method arrives as a
/// name, which is why `Method::Unknown` is the right spelling here and not
/// a sign of anything being wrong.
async fn run(
    w: &mut World,
    wallet: &dyn WalletService,
    method: WalletMethod,
    params: Value,
) -> Result<Value, NncError> {
    let handler = Handler::Wallet(wallet);
    nostr_ln::service::handle(
        &handler,
        &w.grants,
        &mut w.usage,
        &w.controller.public_key(),
        None,
        &Method::Unknown(method.as_str().to_string()),
        &params,
        0,
    )
    .await
}

const FULL: &str = r#"{"methods":{"OTHERS":{}}}"#;

fn a_hold() -> Value {
    json!({"amount": 90_000, "payment_hash": "ab".repeat(32)})
}

// ── what the wallet declares ─────────────────────────────────────────

#[test]
fn an_adopted_extension_is_advertised_like_core() {
    let stub = Stub::default();
    let mut m = stub.methods().to_vec();
    m.sort();
    assert_eq!(
        m,
        vec![
            "cancel_hold_invoice",
            "get_balance",
            "make_hold_invoice",
            "quote_payment",
            "settle_hold_invoice",
        ],
        "kind 13194 is generated from the impl block, so implementing an \
         extension method is what advertises it"
    );
}

#[test]
fn a_wallet_that_does_not_implement_it_does_not_advertise_it() {
    assert_eq!(CoreOnly.methods(), &["get_balance"]);
}

#[test]
fn every_method_this_crate_knows_has_a_wire_spelling_that_round_trips() {
    for m in WalletMethod::ALL {
        let parsed: WalletMethod = m.as_str().parse().unwrap();
        assert_eq!(parsed, m, "{} does not survive a round trip", m.as_str());
        assert!(
            !matches!(parsed, WalletMethod::Unknown(_)),
            "{} parses as unknown, so a wallet could advertise what it \
             cannot dispatch",
            m.as_str()
        );
    }
}

// ── resolve ──────────────────────────────────────────────────────────

#[tokio::test]
async fn an_extension_method_a_wallet_lacks_is_not_implemented() {
    let mut w = world(FULL);
    let e = run(&mut w, &CoreOnly, WalletMethod::MakeHoldInvoice, a_hold()).await.unwrap_err();
    assert_eq!(
        e.code,
        ErrorCode::NotImplemented,
        "answered, not dropped — a client must be able to tell a wallet \
         that will not do this from one that is not there"
    );
}

// ── authorize ────────────────────────────────────────────────────────

#[tokio::test]
async fn a_grant_that_does_not_name_it_restricts_it() {
    let mut w = world(r#"{"methods":{"get_balance":{}}}"#);
    let stub = Stub::default();
    let e = run(&mut w, &stub, WalletMethod::MakeHoldInvoice, a_hold()).await.unwrap_err();
    assert_eq!(e.code, ErrorCode::Restricted);
    assert_eq!(stub.executes(), 0);
    assert_eq!(stub.prepares(), 0, "and nothing was prepared for it");
}

#[tokio::test]
async fn a_grant_may_name_one_hold_method_without_naming_the_others() {
    // Worth asserting because the three travel together in every
    // description of the protocol, and an owner must still be able to
    // grant issuing without granting settling.
    let mut w = world(r#"{"methods":{"make_hold_invoice":{}}}"#);
    let stub = Stub::default();
    assert!(run(&mut w, &stub, WalletMethod::MakeHoldInvoice, a_hold()).await.is_ok());
    let e = run(&mut w, &stub, WalletMethod::SettleHoldInvoice, json!({"preimage": "cd".repeat(32)}))
        .await
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::Restricted);
    assert_eq!(stub.executes(), 1);
}

#[tokio::test]
async fn a_control_grant_does_not_authorise_a_wallet_extension() {
    let mut w = world(r#"{"control":{"OTHERS":{}}}"#);
    let stub = Stub::default();
    let e = run(&mut w, &stub, WalletMethod::MakeHoldInvoice, a_hold()).await.unwrap_err();
    assert_eq!(e.code, ErrorCode::Restricted, "administering does not make you a spender");
}

// ── validate ─────────────────────────────────────────────────────────

#[tokio::test]
async fn a_hold_invoice_without_a_hash_never_reaches_the_wallet() {
    // The one parameter that cannot be defaulted: the hash is what the
    // invoice locks to, and it comes from somebody else. A wallet asked to
    // make one without it would have to invent a secret, which is exactly
    // what a hold invoice must not do.
    let mut w = world(FULL);
    let stub = Stub::default();
    let e = run(&mut w, &stub, WalletMethod::MakeHoldInvoice, json!({"amount": 90_000}))
        .await
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::Other);
    assert_eq!(stub.executes(), 0);
    assert_eq!(stub.prepares(), 0);
}

#[tokio::test]
async fn settling_without_a_preimage_never_reaches_the_wallet() {
    let mut w = world(FULL);
    let stub = Stub::default();
    let e = run(&mut w, &stub, WalletMethod::SettleHoldInvoice, json!({})).await.unwrap_err();
    assert_eq!(e.code, ErrorCode::Other);
    assert_eq!(stub.executes(), 0);
}

#[tokio::test]
async fn quoting_without_an_invoice_never_reaches_the_wallet() {
    let mut w = world(FULL);
    let stub = Stub::default();
    let e = run(&mut w, &stub, WalletMethod::QuotePayment, json!({"amount": 1})).await.unwrap_err();
    assert_eq!(e.code, ErrorCode::Other);
    assert_eq!(stub.executes(), 0);
}

// ── limit ────────────────────────────────────────────────────────────

#[tokio::test]
async fn an_extension_method_is_rate_limited_like_any_other() {
    let profile = r#"{"methods":{"make_hold_invoice":{"rate":{"amount":0,"per_secs":1,"max_capacity":1}}}}"#;
    let mut w = world(profile);
    let stub = Stub::default();

    assert!(run(&mut w, &stub, WalletMethod::MakeHoldInvoice, a_hold()).await.is_ok());
    let e = run(&mut w, &stub, WalletMethod::MakeHoldInvoice, a_hold()).await.unwrap_err();
    assert_eq!(e.code, ErrorCode::RateLimited);
    assert_eq!(stub.executes(), 1, "the second never ran");
}

// ── execute ──────────────────────────────────────────────────────────

#[tokio::test]
async fn the_whole_extension_dispatches_and_decodes() {
    let mut w = world(FULL);
    let stub = Stub::default();

    let made: MakeHoldInvoiceResponse =
        serde_json::from_value(run(&mut w, &stub, WalletMethod::MakeHoldInvoice, a_hold()).await.unwrap())
            .unwrap();
    assert_eq!(made.payment_hash, "ab".repeat(32), "it locks to the hash it was given");

    run(&mut w, &stub, WalletMethod::SettleHoldInvoice, json!({"preimage": "cd".repeat(32)}))
        .await
        .unwrap();
    run(&mut w, &stub, WalletMethod::CancelHoldInvoice, json!({"payment_hash": "ab".repeat(32)}))
        .await
        .unwrap();

    let quoted: QuotePaymentResponse = serde_json::from_value(
        run(&mut w, &stub, WalletMethod::QuotePayment, json!({"invoice": "lnbcrt1..."}))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(quoted.cltv_expiry_delta, 144);

    assert_eq!(
        *stub.seen.lock().unwrap(),
        vec!["make_hold_invoice", "settle_hold_invoice", "cancel_hold_invoice", "quote_payment"],
        "each reached its own method, not a shared entry point"
    );
}
