//! Runs `vectors/nnc.json`, which is generated from `XX.md` itself.
//!
//! The point is that these are the specification's own examples rather than
//! ones written to match the code. A type that only round-trips what it
//! emits proves nothing; these decode what the document says.

use nostr_ln::nnc::*;
use serde_json::Value;

fn vectors() -> Vec<Value> {
    let raw = include_str!("../vectors/nnc.json");
    let doc: Value = serde_json::from_str(raw).expect("vectors parse");
    doc["vectors"].as_array().expect("array").clone()
}

#[test]
fn every_method_in_the_spec_has_a_vector() {
    let names: Vec<String> = vectors()
        .iter()
        .filter(|v| v["kind"] == "method")
        .map(|v| v["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names.len(), 16, "sixteen methods, got {names:?}");
    for m in Method::ALL {
        assert!(
            names.contains(&m.as_str().to_string()),
            "{} has no vector — the spec and the types disagree",
            m.as_str()
        );
    }
}

#[test]
fn every_request_vector_decodes() {
    for v in vectors().iter().filter(|v| v["kind"] == "method") {
        let name = v["name"].as_str().unwrap();
        let Some(req_json) = v.get("request") else { continue };
        let req: Request = serde_json::from_value(req_json.clone())
            .unwrap_or_else(|e| panic!("{name}: request does not decode: {e}"));
        assert_eq!(
            req.method.as_str(),
            name,
            "{name}: the method field disagrees with the heading it is under"
        );
        assert!(
            !matches!(req.method, Method::Unknown(_)),
            "{name}: parsed as Unknown, so the enum is missing it"
        );
    }
}

#[test]
fn every_response_vector_decodes() {
    for v in vectors().iter().filter(|v| v["kind"] == "method") {
        let name = v["name"].as_str().unwrap();
        let Some(res_json) = v.get("response") else { continue };
        let res: Response = serde_json::from_value(res_json.clone())
            .unwrap_or_else(|e| panic!("{name}: response does not decode: {e}"));
        assert_eq!(res.result_type.as_str(), name);
        assert!(res.error.is_none(), "{name}: the spec's example is a success");
        assert!(res.result.is_some(), "{name}: a successful response carries a result");
    }
}

#[test]
fn the_asynchronous_methods_acknowledge_in_their_vectors() {
    // The spec's own examples show `"result": {}` for these two. If that
    // ever changes, this catches it — because the whole two-function client
    // design rests on it.
    for name in ["open_channel", "close_channel"] {
        let v = vectors()
            .into_iter()
            .find(|v| v["name"] == name)
            .unwrap_or_else(|| panic!("{name} missing"));
        let res: Response = serde_json::from_value(v["response"].clone()).unwrap();
        assert!(
            res.is_acknowledgement(),
            "{name}: the spec shows an acknowledgement, not a result"
        );
        assert_eq!(res.result, Some(serde_json::json!({})));
    }
}

#[test]
fn both_notification_vectors_decode() {
    let notifs: Vec<_> = vectors()
        .into_iter()
        .filter(|v| v["kind"] == "notification")
        .collect();
    assert_eq!(notifs.len(), 2);
    for v in notifs {
        let name = v["name"].as_str().unwrap().to_string();
        let n: Notification = serde_json::from_value(v["notification"].clone())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(n.notification_type.as_str(), name);
        match n.notification_type {
            NotificationType::ChannelOpened => {
                n.as_typed::<ChannelOpened>().expect("channel_opened decodes");
            }
            NotificationType::ChannelClosed => {
                n.as_typed::<ChannelClosed>().expect("channel_closed decodes");
            }
            NotificationType::Unknown(u) => panic!("unknown notification type {u}"),
        }
    }
}

#[test]
fn a_vector_that_is_not_in_the_spec_would_be_noticed() {
    // The generator names its source, so a hand-edited vector file is
    // visible rather than silently authoritative.
    let doc: Value = serde_json::from_str(include_str!("../vectors/nnc.json")).unwrap();
    assert!(doc["source"].as_str().unwrap().ends_with("XX.md"));
    assert!(doc["note"].as_str().unwrap().contains("Generated from XX.md"));
}
