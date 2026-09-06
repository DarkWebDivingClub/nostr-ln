//! NIP-XX as types. Each test is a clause of the specification, and where
//! JSON appears it is the JSON `XX.md` shows.

use nostr_ln::nnc::*;
use std::str::FromStr;

// ── the envelope ─────────────────────────────────────────────────────

#[test]
fn every_method_round_trips_its_wire_spelling() {
    for m in Method::ALL {
        let wire = m.as_str().to_string();
        assert_eq!(Method::from_str(&wire).unwrap(), m, "{wire} parses back");
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, format!("\"{wire}\""), "{wire} serialises bare");
        assert_eq!(serde_json::from_str::<Method>(&json).unwrap(), m);
    }
}

#[test]
fn an_unknown_method_parses_rather_than_failing() {
    // Err = Infallible: a name invented tomorrow gets NOT_IMPLEMENTED at
    // dispatch, not a deserialisation failure with nowhere to put it.
    let m = Method::from_str("invented_tomorrow").unwrap();
    assert_eq!(m, Method::Unknown("invented_tomorrow".into()));
    assert_eq!(m.as_str(), "invented_tomorrow");
    assert_eq!(
        serde_json::from_str::<Method>("\"invented_tomorrow\"").unwrap(),
        m
    );
}

#[test]
fn method_is_exhaustive_over_the_specification() {
    // Sixteen, and the count is asserted so that adding a variant without
    // adding it to ALL is caught.
    assert_eq!(Method::ALL.len(), 16);
    let names: Vec<_> = Method::ALL.iter().map(|m| m.as_str()).collect();
    for expected in [
        "list_channels", "open_channel", "close_channel", "list_peers",
        "connect_peer", "disconnect_peer", "get_channel_fees", "set_channel_fees",
        "get_forwarding_history", "get_pending_htlcs", "query_routes",
        "list_network_nodes", "get_network_stats", "get_network_node",
        "get_network_channel", "sign_message",
    ] {
        assert!(names.contains(&expected), "{expected} is missing");
    }
}

#[test]
fn only_two_methods_are_asynchronous() {
    let async_ones: Vec<_> = Method::ALL
        .iter()
        .filter(|m| m.is_asynchronous())
        .map(|m| m.as_str())
        .collect();
    assert_eq!(async_ones, vec!["open_channel", "close_channel"]);

    assert_eq!(Method::OpenChannel.notification(), Some(NotificationType::ChannelOpened));
    assert_eq!(Method::CloseChannel.notification(), Some(NotificationType::ChannelClosed));
    assert_eq!(Method::ListChannels.notification(), None);
}

#[test]
fn a_notification_type_is_not_a_method() {
    // They are separate enums because Method is what a grant's `control`
    // map is keyed on, what dispatch matches over, and what the info event
    // advertises. This asserts the names do not collide into one space.
    let notif_names: Vec<_> = NotificationType::ALL.iter().map(|n| n.as_str()).collect();
    let method_names: Vec<_> = Method::ALL.iter().map(|m| m.as_str()).collect();
    for n in &notif_names {
        assert!(!method_names.contains(n), "{n} must not be a method");
    }
    assert_eq!(notif_names, vec!["channel_opened", "channel_closed"]);
}

#[test]
fn a_result_and_an_error_are_mutually_exclusive() {
    let ok = Response::ok(Method::ListChannels, ListChannelsResponse { channels: vec![] }).unwrap();
    assert!(ok.error.is_none() && ok.result.is_some());
    let json = serde_json::to_string(&ok).unwrap();
    assert!(!json.contains("error"), "a successful response omits error");

    let err = Response::err(
        Method::ListChannels,
        NncError::new(ErrorCode::Restricted, "no"),
    );
    assert!(err.result.is_none() && err.error.is_some());
    let json = serde_json::to_string(&err).unwrap();
    assert!(!json.contains("\"result\""), "a failed response omits result");

    assert!(matches!(
        err.result_as::<ListChannelsResponse>(),
        Err(ResultError::Failed(_))
    ));
}

#[test]
fn every_error_code_in_the_spec_is_representable() {
    for code in [
        "RATE_LIMITED", "NOT_IMPLEMENTED", "RESTRICTED", "UNAUTHORIZED",
        "QUOTA_EXCEEDED", "NOT_FOUND", "CHANNEL_FAILED", "CONNECTION_FAILED",
        "INTERNAL", "OTHER",
    ] {
        let c: ErrorCode = serde_json::from_str(&format!("\"{code}\"")).unwrap();
        assert_eq!(c.to_string(), code, "{code} round-trips");
        assert!(!matches!(c, ErrorCode::Unknown(_)), "{code} has a variant");
    }
    // And one that does not exist yet.
    let c: ErrorCode = serde_json::from_str("\"INVENTED\"").unwrap();
    assert_eq!(c, ErrorCode::Unknown("INVENTED".into()));
}

// ── the methods, against the spec's own JSON ─────────────────────────

#[test]
fn list_channels_round_trips() {
    let req: Request = serde_json::from_str(r#"{"method":"list_channels","params":{}}"#).unwrap();
    assert_eq!(req.method, Method::ListChannels);
    req.params_as::<ListChannelsRequest>().unwrap();

    let res: Response = serde_json::from_str(
        r#"{"result_type":"list_channels","result":{"channels":[
            {"id":"abc123","short_channel_id":"800000x1x0","peer_pubkey":"02abc",
             "state":"active","is_private":false,"capacity":1000000,
             "local_balance":600000,"remote_balance":400000,"funding_txid":"def456"}]}}"#,
    )
    .unwrap();
    let r: ListChannelsResponse = res.result_as().unwrap();
    assert_eq!(r.channels.len(), 1);
    assert_eq!(r.channels[0].state, Some(ChannelState::Active));
    assert_eq!(r.channels[0].capacity, Some(1_000_000));
}

#[test]
fn open_channel_is_acknowledged_not_completed() {
    let req = Request::new(
        Method::OpenChannel,
        OpenChannelRequest {
            pubkey: "02abc".into(),
            amount_sats: 1_000_000,
            push_amount: Some(0),
            private: Some(false),
            host: None,
            close_address: None,
            notify: None,
        },
    )
    .unwrap();
    assert_eq!(req.method, Method::OpenChannel);

    let res: Response =
        serde_json::from_str(r#"{"result_type":"open_channel","result":{}}"#).unwrap();
    res.result_as::<OpenChannelResponse>().unwrap();

    // The distinction the spec spends a paragraph on, in the types.
    assert!(res.is_acknowledgement(), "an empty result here means accepted, not done");
    let sync: Response =
        serde_json::from_str(r#"{"result_type":"list_peers","result":{"peers":[]}}"#).unwrap();
    assert!(!sync.is_acknowledgement());
}

#[test]
fn notify_round_trips_and_is_absent_by_default() {
    let plain = OpenChannelRequest {
        pubkey: "02abc".into(), amount_sats: 1, push_amount: None, private: None,
        host: None, close_address: None, notify: None,
    };
    let json = serde_json::to_string(&plain).unwrap();
    assert!(!json.contains("notify"), "omitted when not set — the spec's default is true");

    let off = OpenChannelRequest { notify: Some(false), ..plain };
    assert!(serde_json::to_string(&off).unwrap().contains(r#""notify":false"#));
}

#[test]
fn the_remaining_methods_round_trip() {
    let cases: Vec<(&str, &str)> = vec![
        (r#"{"method":"close_channel","params":{"id":"abc","force":false,"notify":true}}"#, "close_channel"),
        (r#"{"method":"list_peers","params":{}}"#, "list_peers"),
        (r#"{"method":"connect_peer","params":{"pubkey":"02abc","host":"10.0.0.1:9735"}}"#, "connect_peer"),
        (r#"{"method":"disconnect_peer","params":{"pubkey":"02abc"}}"#, "disconnect_peer"),
        (r#"{"method":"get_channel_fees","params":{"id":"abc"}}"#, "get_channel_fees"),
        (r#"{"method":"set_channel_fees","params":{"id":"abc","base_fee":1000,"fee_rate":1}}"#, "set_channel_fees"),
        (r#"{"method":"get_forwarding_history","params":{"from":1,"until":2,"limit":50,"offset":0}}"#, "get_forwarding_history"),
        (r#"{"method":"get_pending_htlcs","params":{}}"#, "get_pending_htlcs"),
        (r#"{"method":"query_routes","params":{"destination":"02abc","amount":100000,"max_routes":3}}"#, "query_routes"),
        (r#"{"method":"list_network_nodes","params":{"limit":50,"offset":0}}"#, "list_network_nodes"),
        (r#"{"method":"get_network_stats","params":{}}"#, "get_network_stats"),
        (r#"{"method":"get_network_node","params":{"pubkey":"02abc"}}"#, "get_network_node"),
        (r#"{"method":"get_network_channel","params":{"short_channel_id":"800000x1x0"}}"#, "get_network_channel"),
        (r#"{"method":"sign_message","params":{"message":"I am node 02abc"}}"#, "sign_message"),
    ];
    for (json, name) in cases {
        let req: Request = serde_json::from_str(json).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(req.method.as_str(), name);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&serde_json::to_string(&req).unwrap())
                .unwrap()["method"],
            serde_json::Value::String(name.into())
        );
    }
}

#[test]
fn query_routes_response_carries_hops() {
    let res: Response = serde_json::from_str(
        r#"{"result_type":"query_routes","result":{"routes":[{"total_fee":150,
            "total_time_lock":40,"hops":[{"pubkey":"02abc","short_channel_id":"800000x1x0","fee":50}]}]}}"#,
    ).unwrap();
    let r: QueryRoutesResponse = res.result_as().unwrap();
    assert_eq!(r.routes[0].hops.len(), 1);
    assert_eq!(r.routes[0].total_fee, 150);
}

#[test]
fn get_network_channel_carries_both_policies() {
    let res: Response = serde_json::from_str(
        r#"{"result_type":"get_network_channel","result":{"short_channel_id":"800000x1x0",
            "capacity":1000000,"node1_pubkey":"02abc","node2_pubkey":"03def",
            "node1_policy":{"base_fee":1000,"fee_rate":1,"disabled":false},
            "node2_policy":{"base_fee":500,"fee_rate":2,"disabled":true}}}"#,
    ).unwrap();
    let r: GetNetworkChannelResponse = res.result_as().unwrap();
    assert_eq!(r.node1_policy.unwrap().base_fee, Some(1000));
    assert!(r.node2_policy.unwrap().disabled);
}

// ── the two notifications ────────────────────────────────────────────

#[test]
fn channel_opened_round_trips() {
    let n: Notification = serde_json::from_str(
        r#"{"notification_type":"channel_opened","notification":{
            "id":"abc123","short_channel_id":"800000x1x0","peer_pubkey":"02abc",
            "state":"active","capacity":1000000,"local_balance":1000000,
            "remote_balance":0,"funding_txid":"abc123","is_private":false}}"#,
    ).unwrap();
    assert_eq!(n.notification_type, NotificationType::ChannelOpened);
    let c: ChannelOpened = n.as_typed().unwrap();
    assert_eq!(c.channel.id, "abc123");
    assert_eq!(c.channel.capacity, Some(1_000_000));
}

#[test]
fn channel_closed_covers_closes_nobody_asked_for() {
    for (ct, expected) in [
        ("cooperative", CloseType::Cooperative),
        ("force_local", CloseType::ForceLocal),
        ("force_remote", CloseType::ForceRemote),
        ("breach", CloseType::Breach),
    ] {
        let n: Notification = serde_json::from_str(&format!(
            r#"{{"notification_type":"channel_closed","notification":{{
                "id":"abc","closing_txid":"def","close_type":"{ct}"}}}}"#
        ))
        .unwrap();
        let c: ChannelClosed = n.as_typed().unwrap();
        assert_eq!(c.close_type, expected);
    }
}

#[test]
fn a_channel_is_one_type_in_both_places() {
    // list_channels and channel_opened carry the same Channel, not two
    // near-identical structs that can drift apart.
    let from_list: Response = serde_json::from_str(
        r#"{"result_type":"list_channels","result":{"channels":[
            {"id":"x","peer_pubkey":"02a","state":"active"}]}}"#,
    ).unwrap();
    let listed: ListChannelsResponse = from_list.result_as().unwrap();

    // channel_opened carries no `state` — the notification means the
    // channel is active by definition. Asserted from the spec's shape
    // rather than from an invented one: the first version of this test
    // passed only because the JSON was written to match the code.
    let n: Notification = serde_json::from_str(
        r#"{"notification_type":"channel_opened","notification":{
            "id":"x","peer_pubkey":"02a"}}"#,
    ).unwrap();
    let opened: ChannelOpened = n.as_typed().unwrap();

    assert_eq!(opened.channel.state, None, "channel_opened omits state");
    assert_eq!(listed.channels[0].id, opened.channel.id);
    assert_eq!(listed.channels[0].peer_pubkey, opened.channel.peer_pubkey);
}

// ── sign_message and the two confusable keys ─────────────────────────

#[test]
fn a_node_id_cannot_be_a_nostr_pubkey() {
    let node_id = "02".to_string() + &"a".repeat(64);
    assert!(NodeId::parse(&node_id).is_ok());
    assert!(NodeId::parse(&("03".to_string() + &"b".repeat(64))).is_ok());

    // A Nostr pubkey: 32 bytes x-only, 64 hex characters, no prefix.
    let nostr_pubkey = "a".repeat(64);
    assert!(
        NodeId::parse(&nostr_pubkey).is_err(),
        "the spec warns these are confusable; the type must refuse"
    );
    assert!(NodeId::parse("04".to_string().repeat(33).as_str()).is_err());
    assert!(NodeId::parse("not hex at all").is_err());
}

#[test]
fn sign_message_round_trips() {
    let res: Response = serde_json::from_str(&format!(
        r#"{{"result_type":"sign_message","result":{{"message":"I am node 02abc",
            "signature":"d9tibmnic9t5","pubkey":"{}"}}}}"#,
        "02".to_string() + &"a".repeat(64)
    )).unwrap();
    let r: SignMessageResponse = res.result_as().unwrap();
    assert!(r.pubkey.as_str().starts_with("02"));
    assert_eq!(r.pubkey.as_str().len(), 66);
}
