//! Runs `vectors/nwc.json`, generated from the NWC specifications.
//!
//! The point is that these are the documents' own examples rather than ones
//! written to match the code. A type that only round-trips what it emits
//! proves nothing; these decode what the specifications say.
//!
//! This has earned itself twice on the NNC side. Most recently
//! `channel_opened` carried a `state` field in the type and none in the
//! specification's example: the hand-written round trip passed on invented
//! JSON and the generated vector failed.

use nostr_ln::nwc::methods::*;
use nostr_ln::nwc::WalletMethod;
use serde_json::Value;

fn vectors() -> Vec<Value> {
    let doc: Value =
        serde_json::from_str(include_str!("../vectors/nwc.json")).expect("vectors parse");
    doc["vectors"].as_array().expect("array").clone()
}

fn find(name: &str) -> Value {
    vectors()
        .into_iter()
        .find(|v| v["name"] == name)
        .unwrap_or_else(|| panic!("{name} has no vector"))
}

/// Decode `v[part]["params"]` or `v[part]["result"]` as `T`, then
/// re-encode and compare against what the document showed.
fn round_trip<T>(name: &str, part: &str, inner: &str)
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let v = find(name);
    let doc = v
        .get(part)
        .unwrap_or_else(|| panic!("{name}: no {part} in the vector"))
        .get(inner)
        .unwrap_or_else(|| panic!("{name}: {part} has no {inner}"))
        .clone();
    let typed: T = serde_json::from_value(doc.clone())
        .unwrap_or_else(|e| panic!("{name} {part}: does not decode: {e}\n{doc:#}"));
    let back = serde_json::to_value(&typed).expect("re-encode");

    // Every field the document shows must survive. The type may drop
    // nothing; it may not add anything either.
    let (a, b) = (doc.as_object().unwrap(), back.as_object().unwrap());
    for (k, want) in a {
        let got = b
            .get(k)
            .unwrap_or_else(|| panic!("{name} {part}: the type drops `{k}`"));
        assert_eq!(want, got, "{name} {part}: `{k}` changed in the round trip");
    }
}

#[test]
fn every_method_we_claim_has_a_vector() {
    let names: Vec<String> =
        vectors().iter().map(|v| v["name"].as_str().unwrap().to_string()).collect();
    for m in WalletMethod::ALL {
        assert!(
            names.contains(&m.as_str().to_string()),
            "{} has no vector — the specifications and the types disagree",
            m.as_str()
        );
    }
}

#[test]
fn every_vector_is_a_method_we_claim() {
    for v in vectors() {
        let name = v["name"].as_str().unwrap();
        let m: WalletMethod = name.parse().unwrap();
        assert!(
            !matches!(m, WalletMethod::Unknown(_)),
            "{name} is in a specification we implement and not in WalletMethod"
        );
    }
}

#[test]
fn published_core_is_covered() {
    // Distinct from the test above: an extension method must not be able
    // to satisfy a claim about core.
    let names: Vec<String> =
        vectors().iter().map(|v| v["name"].as_str().unwrap().to_string()).collect();
    for m in WalletMethod::CORE {
        assert!(m.is_core());
        assert!(names.contains(&m.as_str().to_string()), "core method {m} uncovered");
    }
}

#[test]
fn pay_invoice_round_trips() {
    round_trip::<PayInvoiceRequest>("pay_invoice", "request", "params");
    round_trip::<PayInvoiceResponse>("pay_invoice", "response", "result");
}

#[test]
fn make_invoice_round_trips() {
    round_trip::<MakeInvoiceRequest>("make_invoice", "request", "params");
    round_trip::<MakeInvoiceResponse>("make_invoice", "response", "result");
}

#[test]
fn lookup_invoice_round_trips() {
    round_trip::<LookupInvoiceRequest>("lookup_invoice", "request", "params");
    round_trip::<LookupInvoiceResponse>("lookup_invoice", "response", "result");
}

#[test]
fn get_balance_round_trips() {
    round_trip::<GetBalanceRequest>("get_balance", "request", "params");
    round_trip::<GetBalanceResponse>("get_balance", "response", "result");
}

#[test]
fn get_info_round_trips() {
    round_trip::<GetInfoRequest>("get_info", "request", "params");
    round_trip::<GetInfoResponse>("get_info", "response", "result");
}

#[test]
fn pay_onchain_round_trips() {
    round_trip::<PayOnchainRequest>("pay_onchain", "request", "params");
    round_trip::<PayOnchainResponse>("pay_onchain", "response", "result");
}

#[test]
fn get_balance_has_one_field_as_published_core_defines_it() {
    // Our forked 47.md adds lightning_balance and onchain_balance_sats.
    // That is divergence inside core; 18.2 decides whether it becomes an
    // extension. This asserts we did not carry it over by habit.
    let v = find("get_balance");
    let result = v["response"]["result"].as_object().unwrap();
    assert_eq!(
        result.keys().collect::<Vec<_>>(),
        vec!["balance"],
        "published core's get_balance has exactly one field"
    );
}

#[test]
fn the_error_codes_the_specifications_name_have_variants() {
    use nostr_ln::nnc::ErrorCode;
    // Published NIP-47 core's list, plus the two nwc-onchain.md adds.
    // A code spelled as `Unknown("...")` round-trips but cannot be matched
    // on, which is how UNSUPPORTED_ENCRYPTION was written until 18.1's
    // first consumer needed the others.
    for code in [
        "RATE_LIMITED",
        "NOT_IMPLEMENTED",
        "INSUFFICIENT_BALANCE",
        "QUOTA_EXCEEDED",
        "RESTRICTED",
        "UNAUTHORIZED",
        "INTERNAL",
        "UNSUPPORTED_ENCRYPTION",
        "OTHER",
        "PAYMENT_FAILED",
        "BAD_REQUEST",
        "UNSUPPORTED_NETWORK",
    ] {
        let parsed: ErrorCode =
            serde_json::from_value(serde_json::json!(code)).expect("decodes");
        assert!(
            !matches!(parsed, ErrorCode::Unknown(_)),
            "{code} is named by a specification we implement and has no variant"
        );
        assert_eq!(parsed.to_string(), code, "{code} does not round-trip");
    }
}
