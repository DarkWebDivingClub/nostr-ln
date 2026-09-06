//! The transport, against what can be checked without a relay.
//!
//! The scenarios needing one — a live grant, a revocation mid-session, a
//! reconnection — are mission 17.3, which has a relay harness. What is
//! testable here is the shape: which kinds a service subscribes to, what
//! it publishes, and that presence of a handler is the declaration.

#![cfg(feature = "transport")]

use std::sync::Arc;

use nostr::key::Keys;
use nostr_ln::nnc::{methods::*, NncError};
use nostr_ln::service::handler::Fut;
use nostr_ln::service::transport::*;
use nostr_ln::service::*;
use serde_json::Value;

struct Node;

#[nostr_ln::service]
impl ControlService for Node {
    fn list_channels<'a>(
        &'a self,
        _r: ListChannelsRequest,
        _c: Caller<'a>,
    ) -> Fut<'a, Result<ListChannelsResponse, NncError>> {
        Box::pin(async move { Ok(ListChannelsResponse { channels: vec![] }) })
    }
}

struct Wallet;

// **No `#[nostr_ln::service]` here**, and that is the finding rather than
// an oversight. `WalletService` has one `call` entry point rather than a
// function per method, so there is nothing in the impl block for the macro
// to read — it would emit an empty list. An NWC handler therefore writes
// `methods()` by hand, and carries the weaker guarantee that comes with a
// list which can disagree with what it answers. Mission 18 removes the
// difference by bringing NIP-47's types here.
impl WalletService for Wallet {
    fn methods(&self) -> &'static [&'static str] {
        &["pay_bip321", "get_balance"]
    }

    fn call<'a>(
        &'a self,
        _m: &'a str,
        _p: &'a Value,
        _c: Caller<'a>,
    ) -> Fut<'a, Result<Value, NncError>> {
        Box::pin(async move { Ok(Value::Null) })
    }
}

#[test]
fn the_kinds_are_the_ones_the_spec_names() {
    assert_eq!(WALLET_INFO_KIND, 13194);
    assert_eq!(WALLET_REQUEST_KIND, 23194);
    assert_eq!(WALLET_RESPONSE_KIND, 23195);
    assert_eq!(CONTROL_INFO_KIND, 13198);
    assert_eq!(CONTROL_REQUEST_KIND, 23198);
    assert_eq!(CONTROL_RESPONSE_KIND, 23199);
}

#[test]
fn a_service_can_be_built_with_either_handler_or_both() {
    let signer = Keys::generate();
    let relays = vec!["ws://localhost:7777".to_string()];
    let owners = vec![Keys::generate().public_key()];

    let _control_only = Service::new(signer.clone(), relays.clone(), owners.clone())
        .control(Arc::new(Node));
    let _wallet_only = Service::new(signer.clone(), relays.clone(), owners.clone())
        .wallet(Arc::new(Wallet));
    let _both = Service::new(signer, relays, owners)
        .control(Arc::new(Node))
        .wallet(Arc::new(Wallet));
}

#[test]
fn an_info_event_lists_exactly_what_the_handler_implements() {
    // Generated from the impl block, so a node cannot advertise a method it
    // does not have — and adding one to the impl adds it here with no
    // second list to update.
    assert_eq!(Node.methods(), &["list_channels"]);

    // The NWC side is hand-written, so this asserts only that it is what
    // the handler declared — not that the handler can answer it. That gap
    // is the deviation recorded in 13.2 and closed by mission 18.
    assert_eq!(Wallet.methods(), &["pay_bip321", "get_balance"]);
}

#[tokio::test]
async fn a_service_with_no_owners_is_constructable_and_accepts_nothing() {
    // Absent configuration fails closed. Treating "no owners" as "any
    // owner" is dln-node#1, and it is the direction that matters.
    let signer = Keys::generate();
    let me = nostr::key::Keys::generate().public_key();
    let mut grants = nostr_ln::Grants::new(me, vec![]);

    let owner = Keys::generate();
    let d = format!("{}:{}", me.to_hex(), Keys::generate().public_key().to_hex());
    let event = nostr::event::EventBuilder::new(
        nostr::event::Kind::Custom(nostr_ln::GRANT_KIND),
        r#"{"control":{"OTHERS":{}}}"#,
    )
    .tags([
        nostr::event::Tag::identifier(d),
        nostr::event::Tag::parse(vec!["p".to_string(), me.to_hex()]).unwrap(),
    ])
    .sign_with_keys(&owner)
    .unwrap();

    assert!(grants.apply(&event).is_err(), "no owners means no grant is trusted");
    let _ = Service::new(signer, vec!["ws://localhost:7777".into()], vec![]);
}

#[test]
fn reconnection_is_polled_because_the_sdk_does_not_report_it() {
    // ClientNotification carries Event, Message and Shutdown — no relay
    // status. If a future SDK adds one, this constant should disappear and
    // the loop should watch it instead.
    assert!(RECONNECT_POLL.as_secs() > 0);
    assert!(
        RECONNECT_POLL.as_secs() <= 30,
        "a revocation should take effect promptly once the relay is back"
    );
}
