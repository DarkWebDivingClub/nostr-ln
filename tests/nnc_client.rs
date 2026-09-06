//! The NNC client. What can be tested without a relay is tested here;
//! everything that needs one is mission 17.3.

#![cfg(feature = "client")]

use std::str::FromStr;
use std::time::Duration;

use nostr::key::Keys;
use nostr_ln::nnc::client::{Error, NostrNodeControl, Pending, DEFAULT_OUTCOME_TIMEOUT, DEFAULT_TIMEOUT};
use nostr_ln::nnc::*;

const SERVICE: &str = "b889ff5b1513b641e2a139f661a661364979c5beee91842f8f0ef42ab558e9d4";

fn uri() -> NodeControlUri {
    NodeControlUri::from_str(&format!(
        "nostr+nodecontrol://{SERVICE}?relay=wss%3A%2F%2Frelay.damus.io"
    ))
    .unwrap()
}

// ── the URI ──────────────────────────────────────────────────────────

#[test]
fn a_node_control_uri_round_trips() {
    let u = uri();
    assert_eq!(u.service.to_hex(), SERVICE);
    assert_eq!(u.relays, vec!["wss://relay.damus.io"]);
    assert_eq!(NodeControlUri::from_str(&u.to_string()).unwrap(), u);
}

#[test]
fn a_uri_carrying_a_secret_is_rejected_rather_than_ignored() {
    // That is an NWC URI. Accepting it would silently give the caller an
    // identity the owner never granted anything to.
    let e = NodeControlUri::from_str(&format!(
        "nostr+nodecontrol://{SERVICE}?relay=wss%3A%2F%2Fr.example&secret=deadbeef"
    ))
    .unwrap_err();
    assert_eq!(e, UriError::CarriesSecret);
    assert!(e.to_string().contains("signs with its own key"));
}

#[test]
fn a_uri_needs_a_relay_and_the_right_scheme() {
    assert_eq!(
        NodeControlUri::from_str(&format!("nostr+nodecontrol://{SERVICE}")).unwrap_err(),
        UriError::NoRelay
    );
    assert!(matches!(
        NodeControlUri::from_str(&format!("nostr+walletconnect://{SERVICE}?relay=wss%3A%2F%2Fx")),
        Err(UriError::WrongScheme(_))
    ));
    assert_eq!(
        NodeControlUri::from_str("nostr+nodecontrol://nothex?relay=wss%3A%2F%2Fx").unwrap_err(),
        UriError::BadServiceKey
    );
}

#[test]
fn multiple_relays_are_kept() {
    let u = NodeControlUri::from_str(&format!(
        "nostr+nodecontrol://{SERVICE}?relay=wss%3A%2F%2Fa.example&relay=wss%3A%2F%2Fb.example"
    ))
    .unwrap();
    assert_eq!(u.relays, vec!["wss://a.example", "wss://b.example"]);
}

// ── the client ───────────────────────────────────────────────────────

#[test]
fn a_client_is_built_from_a_uri_and_its_own_signer() {
    // Two arguments where NWC takes one, because NNC's URI has no secret.
    let nnc = NostrNodeControl::new(uri(), Keys::generate());
    assert_eq!(nnc.service().to_hex(), SERVICE);
}

#[test]
fn the_two_timeouts_differ_by_design() {
    // A response is a round trip; an outcome waits for a confirmation.
    assert_eq!(DEFAULT_TIMEOUT, Duration::from_secs(30));
    assert_eq!(DEFAULT_OUTCOME_TIMEOUT, Duration::from_secs(3600));
    assert!(
        DEFAULT_OUTCOME_TIMEOUT > DEFAULT_TIMEOUT * 10,
        "waiting for a channel to confirm is not a request timeout"
    );
}

#[tokio::test]
async fn a_pending_handle_is_send_and_static_so_it_can_be_spawned() {
    // The property the whole design rests on: a dashboard cannot block a
    // request thread for six blocks, so the wait must be movable.
    fn assert_spawnable<T: Send + 'static>() {}
    assert_spawnable::<Pending<ChannelOpened>>();
    assert_spawnable::<Pending<ChannelClosed>>();

    // And the future it becomes must be Send too, or spawn still refuses.
    fn assert_future_send<T>()
    where
        T: std::future::IntoFuture,
        T::IntoFuture: Send,
    {
    }
    assert_future_send::<Pending<ChannelOpened>>();
}

#[test]
fn unauthorized_says_what_is_actually_wrong() {
    // A client can connect perfectly and be refused every call, because in
    // NNC a URI is not a credential. A bare code would send somebody
    // debugging their relay.
    let e = Error::Refused(NncError::new(ErrorCode::Unauthorized, "no grant"));
    let msg = e.to_string();
    assert!(msg.contains("30198"), "names the kind the owner must publish");
    assert!(msg.contains("not a credential"), "says why connecting was not enough");

    let other = Error::Refused(NncError::new(ErrorCode::Restricted, "nope"));
    assert!(!other.to_string().contains("not a credential"));
}

#[test]
fn every_method_has_a_function() {
    // Asserted against Method::ALL so a method added to the types cannot be
    // forgotten here. The list is the function names, kept in step by hand
    // — this test is what makes that safe.
    let covered = [
        "list_channels", "open_channel", "close_channel", "list_peers",
        "connect_peer", "disconnect_peer", "get_channel_fees", "set_channel_fees",
        "get_forwarding_history", "get_pending_htlcs", "query_routes",
        "list_network_nodes", "get_network_stats", "get_network_node",
        "get_network_channel", "sign_message",
    ];
    for m in Method::ALL {
        assert!(
            covered.contains(&m.as_str()),
            "{} has no client function",
            m.as_str()
        );
    }
    assert_eq!(covered.len(), Method::ALL.len());
}

#[test]
fn the_asynchronous_methods_have_both_forms() {
    // Two functions rather than a flag: the caller says at the call site
    // whether it wants the outcome, and the return type follows.
    let src = include_str!("../src/nnc/client/methods.rs");
    for m in ["open_channel", "close_channel"] {
        assert!(
            src.contains(&format!("pub async fn {m}(")),
            "{m} is missing its awaiting form"
        );
        assert!(
            src.contains(&format!("pub async fn {m}_without_notification(")),
            "{m} is missing its fire-and-forget form"
        );
    }
    // And the awaiting one is must_use, so dropping the handle warns.
    let pending = include_str!("../src/nnc/client/pending.rs");
    assert!(pending.contains("#[must_use"), "a dropped handle must warn");
    assert!(
        pending.contains("_without_notification"),
        "and the warning should name the alternative"
    );
}
