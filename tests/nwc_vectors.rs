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
use nostr_ln::nwc::{WalletMethod, WalletNotificationType};
use serde_json::Value;

fn vectors() -> Vec<Value> {
    let doc: Value =
        serde_json::from_str(include_str!("../vectors/nwc.json")).expect("vectors parse");
    doc["vectors"].as_array().expect("array").clone()
}

fn find(name: &str) -> Value {
    let all: Vec<Value> =
        vectors().into_iter().filter(|v| v["name"] == name).collect();
    assert!(!all.is_empty(), "{name} has no vector");
    assert_eq!(
        all.len(),
        1,
        "{name} has {} vectors — name it by source with find_from",
        all.len()
    );
    all.into_iter().next().unwrap()
}

/// One vector for a method that more than one document defines.
///
/// `lookup_payment` is the case: NWC-09 defines it and NWC-12 extends it
/// with the `bolt12` payment type, so both documents carry an example and
/// both are worth decoding. That is not a duplicate definition, and a
/// `find` that silently took the first would test one of them and quietly
/// never look at the other.
fn find_from(name: &str, source_ends_with: &str) -> Value {
    vectors()
        .into_iter()
        .find(|v| {
            v["name"] == name
                && v["source"].as_str().is_some_and(|s| s.ends_with(source_ends_with))
        })
        .unwrap_or_else(|| panic!("{name} has no vector from {source_ends_with}"))
}

/// As `round_trip`, for a vector selected by source.
fn round_trip_from<T>(name: &str, source: &str, part: &str, inner: &str)
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let v = find_from(name, source);
    check_round_trip::<T>(&v, name, part, inner);
}

/// Decode `v[part]["params"]` or `v[part]["result"]` as `T`, then
/// re-encode and compare against what the document showed.
fn round_trip<T>(name: &str, part: &str, inner: &str)
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let v = find(name);
    check_round_trip::<T>(&v, name, part, inner);
}

fn check_round_trip<T>(v: &Value, name: &str, part: &str, inner: &str)
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
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
        // Notifications are vectors too since 25.2, and are not methods.
        if v["kind"] == "notification" {
            continue;
        }
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

// ── the adopted extensions ───────────────────────────────────────────

#[test]
fn the_offer_methods_round_trip() {
    round_trip::<MakeOfferRequest>("make_offer", "request", "params");
    round_trip::<MakeOfferResponse>("make_offer", "response", "result");
    round_trip::<PayOfferRequest>("pay_offer", "request", "params");
    round_trip::<PayOfferResponse>("pay_offer", "response", "result");
    round_trip::<ListOffersRequest>("list_offers", "request", "params");
    round_trip::<ListOffersResponse>("list_offers", "response", "result");
    round_trip::<DisableOfferRequest>("disable_offer", "request", "params");
    round_trip::<DisableOfferResponse>("disable_offer", "response", "result");
}

#[test]
fn lookup_payment_round_trips_for_both_documents_that_define_it() {
    // NWC-09 defines the method and the bolt11 type; NWC-12 adds bolt12.
    // One envelope, two payment types, and the envelope has to decode both
    // — which is the whole reason `details` is untyped.
    round_trip_from::<LookupPaymentRequest>("lookup_payment", "09.md", "request", "params");
    round_trip_from::<LookupPaymentResponse>("lookup_payment", "09.md", "response", "result");
    round_trip_from::<LookupPaymentRequest>("lookup_payment", "12.md", "request", "params");
    round_trip_from::<LookupPaymentResponse>("lookup_payment", "12.md", "response", "result");
}

#[test]
fn pay_keysend_round_trips() {
    round_trip::<PayKeysendRequest>("pay_keysend", "request", "params");
    round_trip::<PayKeysendResponse>("pay_keysend", "response", "result");
}

#[test]
fn list_transactions_round_trips() {
    round_trip::<ListTransactionsRequest>("list_transactions", "request", "params");
    round_trip::<ListTransactionsResponse>("list_transactions", "response", "result");
}

#[test]
fn the_hold_invoice_methods_round_trip() {
    round_trip::<MakeHoldInvoiceRequest>("make_hold_invoice", "request", "params");
    round_trip::<MakeHoldInvoiceResponse>("make_hold_invoice", "response", "result");
    round_trip::<SettleHoldInvoiceRequest>("settle_hold_invoice", "request", "params");
    round_trip::<CancelHoldInvoiceRequest>("cancel_hold_invoice", "request", "params");
}

// ── ours ─────────────────────────────────────────────────────────────

#[test]
fn the_onchain_methods_round_trip() {
    round_trip::<MakeNewAddressRequest>("make_new_address", "request", "params");
    round_trip::<MakeNewAddressResponse>("make_new_address", "response", "result");
    round_trip::<LookupAddressRequest>("lookup_address", "request", "params");
    round_trip::<LookupAddressResponse>("lookup_address", "response", "result");
    round_trip::<ListAddressesRequest>("list_addresses", "request", "params");
    round_trip::<ListAddressesResponse>("list_addresses", "response", "result");
    round_trip::<EstimateOnchainFeesRequest>("estimate_onchain_fees", "request", "params");
    round_trip::<EstimateOnchainFeesResponse>("estimate_onchain_fees", "response", "result");
}

#[test]
fn list_invoices_round_trips() {
    round_trip::<ListInvoicesRequest>("list_invoices", "request", "params");
    round_trip::<ListInvoicesResponse>("list_invoices", "response", "result");
}

#[test]
fn the_bip321_methods_round_trip() {
    round_trip::<PayBip321Request>("pay_bip321", "request", "params");
    round_trip::<PayBip321Response>("pay_bip321", "response", "result");
    round_trip::<MakeBip321Request>("make_bip321", "request", "params");
    round_trip::<MakeBip321Response>("make_bip321", "response", "result");
}

// ── notifications ────────────────────────────────────────────────────

#[test]
fn every_notification_we_claim_has_a_vector() {
    // Until 25.2 none of them did. The generator read `Request:` and
    // `Response:` blocks and a notification has neither, so the payload
    // types were checked against nothing — and `hold_invoice_accepted`
    // was missing `metadata` for as long as it existed.
    let names: Vec<String> = vectors()
        .iter()
        .filter(|v| v["kind"] == "notification")
        .map(|v| v["name"].as_str().unwrap().to_string())
        .collect();
    for n in WalletNotificationType::ALL {
        assert!(
            names.contains(&n.as_str().to_string()),
            "{} has no vector",
            n.as_str()
        );
    }
}

#[test]
fn the_notifications_round_trip() {
    round_trip::<HoldInvoiceAccepted>("hold_invoice_accepted", "notification", "notification");
    round_trip::<PaymentReceived>("payment_received", "notification", "notification");
    round_trip::<PaymentSent>("payment_sent", "notification", "notification");
}

#[test]
fn get_balance_has_one_field_as_published_core_defines_it() {
    // Our forked 47.md added lightning_balance and onchain_balance_sats.
    // 25.1 settled it: they are an extension, defined by `nwc-onchain.md`
    // and optional on the type. So the *vector* — which comes from
    // published core — still has exactly one field, and this asserts the
    // extension did not leak back into core by habit.
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
