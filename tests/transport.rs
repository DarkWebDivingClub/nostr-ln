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
use nostr_ln::nwc::methods::*;
use nostr_ln::nwc::WalletMethod;
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

// The macro now works here, which is what mission 18.1 was for. Until it
// landed, `WalletService` had one `call` entry point, so there was nothing
// in the impl block for the macro to read and it emitted an **empty** list
// with no error — a wallet that compiled and advertised nothing.
#[nostr_ln::service]
impl WalletService for Wallet {
    fn get_balance<'a>(
        &'a self,
        _r: GetBalanceRequest,
        _c: Caller<'a>,
    ) -> Fut<'a, Result<GetBalanceResponse, NncError>> {
        Box::pin(async move { Ok(GetBalanceResponse { balance: 0 }) })
    }

    fn pay_onchain<'a>(
        &'a self,
        _r: PayOnchainRequest,
        _c: Caller<'a>,
    ) -> Fut<'a, Result<PayOnchainResponse, NncError>> {
        Box::pin(async move {
            Ok(PayOnchainResponse { txid: "abc".into(), fee_sats: None })
        })
    }
}

/// A wallet that implements nothing at all.
///
/// The case that used to fail silently: every `WalletService` produced an
/// empty list, so this and `Wallet` were indistinguishable on the wire.
struct EmptyWallet;

#[nostr_ln::service]
impl WalletService for EmptyWallet {}

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

    // And now the NWC side is generated too, from the same impl block it
    // answers from. A wallet cannot advertise what it does not implement.
    assert_eq!(Wallet.methods(), &["get_balance", "pay_onchain"]);

    // A wallet that implements nothing advertises nothing. This is the
    // case that used to pass silently for *every* wallet: the macro found
    // only `call`, excluded it, and emitted an empty list either way.
    assert_eq!(EmptyWallet.methods(), &[] as &[&str]);
}

#[tokio::test]
async fn a_wallet_answers_what_it_implements_and_refuses_the_rest() {
    // The declaration is only worth having if it matches behaviour. This
    // is the half a generated list cannot prove on its own.
    let caller = Keys::generate().public_key();
    let c = Caller { controller: &caller, request_id: None };

    let ok = dispatch_wallet(
        &Wallet,
        &WalletMethod::GetBalance,
        &serde_json::json!({}),
        c,
    )
    .await;
    assert!(ok.is_ok(), "declared and answered");

    // Core method it did not implement.
    let no = dispatch_wallet(
        &Wallet,
        &WalletMethod::PayInvoice,
        &serde_json::json!({"invoice": "lnbc1"}),
        c,
    )
    .await;
    assert!(no.is_err(), "not declared, so NOT_IMPLEMENTED");

    // A method from an extension we have not adopted.
    let unknown = dispatch_wallet(
        &Wallet,
        &WalletMethod::Unknown("make_hold_invoice".into()),
        &serde_json::json!({}),
        c,
    )
    .await;
    assert!(unknown.is_err(), "NWC-03 is not adopted; NOT_IMPLEMENTED, not unroutable");
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
