//! Conformance for the access layer. No chain, no node, no relay.
//!
//! Each test is a clause of NIP-XX, and several are a filed bug against a
//! real implementation.

use nostr::event::{Event, EventBuilder, Kind, Tag};
use nostr::key::{Keys, PublicKey};
use nostr::types::Timestamp;
use nostr_ln::grant::Rejected;
use nostr_ln::*;

fn grant_event(owner: &Keys, node: &PublicKey, d_target: &str, content: &str, at: u64) -> Event {
    event(owner, GRANT_KIND, node, &format!("{}:{}", node.to_hex(), d_target), content, at)
}

fn event(
    author: &Keys,
    kind: u16,
    p: &PublicKey,
    d: &str,
    content: &str,
    at: u64,
) -> Event {
    EventBuilder::new(Kind::Custom(kind), content)
        .tags([
            Tag::identifier(d.to_string()),
            Tag::parse(vec!["p".to_string(), p.to_hex()]).unwrap(),
        ])
        .custom_created_at(Timestamp::from(at))
        .sign_with_keys(author)
        .expect("sign")
}

const FULL: &str = r#"{"methods":{"get_balance":{}},"control":{"list_channels":{}}}"#;

struct World {
    node: Keys,
    owner: Keys,
    controller: Keys,
    stranger: Keys,
}

fn world() -> World {
    World {
        node: Keys::generate(),
        owner: Keys::generate(),
        controller: Keys::generate(),
        stranger: Keys::generate(),
    }
}

// ── who may issue a grant ────────────────────────────────────────────

#[test]
fn a_grant_signed_by_a_non_owner_is_refused() {
    let w = world();
    let mut g = Grants::new(w.node.public_key(), vec![w.owner.public_key()]);
    let e = grant_event(&w.stranger, &w.node.public_key(), &w.controller.public_key().to_hex(), FULL, 1);
    assert_eq!(g.apply(&e), Err(Rejected::NotAnOwner));
    assert!(g.resolve(&w.controller.public_key()).is_err());
}

#[test]
fn an_empty_owner_list_refuses_every_grant() {
    // dln-node#1: both guards fail *open* when unset. Absent configuration
    // must make the type unconstructable in practice, not permissive.
    let w = world();
    let mut g = Grants::new(w.node.public_key(), vec![]);
    let e = grant_event(&w.owner, &w.node.public_key(), &w.controller.public_key().to_hex(), FULL, 1);
    assert_eq!(g.apply(&e), Err(Rejected::NoOwnersConfigured));
}

#[test]
fn a_grant_of_the_old_kind_is_not_a_grant() {
    // 30078 carried grants until NIP-XX moved them to 30198.
    let w = world();
    let mut g = Grants::new(w.node.public_key(), vec![w.owner.public_key()]);
    let e = event(
        &w.owner,
        30078,
        &w.node.public_key(),
        &format!("{}:{}", w.node.public_key().to_hex(), w.controller.public_key().to_hex()),
        FULL,
        1,
    );
    assert_eq!(g.apply(&e), Err(Rejected::WrongKind));
}

#[test]
fn a_grant_with_no_p_tag_naming_this_node_is_refused() {
    let w = world();
    let mut g = Grants::new(w.node.public_key(), vec![w.owner.public_key()]);
    let elsewhere = Keys::generate().public_key();
    let e = EventBuilder::new(Kind::Custom(GRANT_KIND), FULL)
        .tags([
            Tag::identifier(format!(
                "{}:{}",
                w.node.public_key().to_hex(),
                w.controller.public_key().to_hex()
            )),
            Tag::parse(vec!["p".to_string(), elsewhere.to_hex()]).unwrap(),
        ])
        .sign_with_keys(&w.owner)
        .unwrap();
    assert_eq!(g.apply(&e), Err(Rejected::NotAddressedToThisNode));
}

#[test]
fn a_grant_whose_d_tag_names_another_node_is_refused() {
    let w = world();
    let other_node = Keys::generate().public_key();
    let mut g = Grants::new(w.node.public_key(), vec![w.owner.public_key()]);
    let e = EventBuilder::new(Kind::Custom(GRANT_KIND), FULL)
        .tags([
            Tag::identifier(format!(
                "{}:{}",
                other_node.to_hex(),
                w.controller.public_key().to_hex()
            )),
            Tag::parse(vec!["p".to_string(), w.node.public_key().to_hex()]).unwrap(),
        ])
        .sign_with_keys(&w.owner)
        .unwrap();
    assert_eq!(g.apply(&e), Err(Rejected::WrongTarget));
}

#[test]
fn an_older_grant_does_not_displace_a_newer_one() {
    let w = world();
    let c = w.controller.public_key().to_hex();
    let mut g = Grants::new(w.node.public_key(), vec![w.owner.public_key()]);
    g.apply(&grant_event(&w.owner, &w.node.public_key(), &c, FULL, 100)).unwrap();
    let older = grant_event(&w.owner, &w.node.public_key(), &c, r#"{"methods":{}}"#, 50);
    assert_eq!(g.apply(&older), Err(Rejected::Superseded));
    assert!(g
        .resolve(&w.controller.public_key())
        .unwrap()
        .profile()
        .allows_wallet("get_balance")
        .is_ok());
}

// ── precedence and OTHERS ────────────────────────────────────────────

#[test]
fn a_key_with_no_grant_and_no_others_is_unauthorized() {
    let w = world();
    let g = Grants::new(w.node.public_key(), vec![w.owner.public_key()]);
    assert!(matches!(g.resolve(&w.controller.public_key()), Err(Denied::Unauthorized)));
}

#[test]
fn others_applies_to_a_key_with_no_grant_of_its_own() {
    let w = world();
    let mut g = Grants::new(w.node.public_key(), vec![w.owner.public_key()]);
    g.apply(&grant_event(&w.owner, &w.node.public_key(), OTHERS, FULL, 1)).unwrap();
    let p = g.resolve(&w.stranger.public_key()).unwrap();
    assert!(p.profile().allows_wallet("get_balance").is_ok());
}

#[test]
fn an_explicit_grant_takes_precedence_over_others() {
    let w = world();
    let c = w.controller.public_key().to_hex();
    let mut g = Grants::new(w.node.public_key(), vec![w.owner.public_key()]);
    g.apply(&grant_event(&w.owner, &w.node.public_key(), OTHERS, FULL, 1)).unwrap();
    g.apply(&grant_event(
        &w.owner,
        &w.node.public_key(),
        &c,
        r#"{"control":{"list_peers":{}}}"#,
        1,
    ))
    .unwrap();

    let mine = g.resolve(&w.controller.public_key()).unwrap();
    assert!(mine.profile().allows_control("list_peers").is_ok());
    assert_eq!(mine.profile().allows_wallet("get_balance"), Err(Denied::Restricted));
}

#[test]
fn an_explicit_empty_grant_denies_a_key_that_others_would_allow() {
    // Revocation of one controller while OTHERS stays in force.
    let w = world();
    let c = w.controller.public_key().to_hex();
    let mut g = Grants::new(w.node.public_key(), vec![w.owner.public_key()]);
    g.apply(&grant_event(&w.owner, &w.node.public_key(), OTHERS, FULL, 1)).unwrap();
    g.apply(&grant_event(&w.owner, &w.node.public_key(), &c, "{}", 1)).unwrap();

    assert!(g.resolve(&w.stranger.public_key()).unwrap().profile().allows_wallet("get_balance").is_ok());
    let revoked = g.resolve(&w.controller.public_key()).unwrap();
    assert!(revoked.profile().is_empty());
    assert_eq!(revoked.profile().allows_wallet("get_balance"), Err(Denied::Restricted));
}

// ── what a profile permits ───────────────────────────────────────────

#[test]
fn the_field_is_rate_not_access_rate() {
    // dln-node#2: reading it as `access_rate` silently yields no limit.
    let p: UsageProfile = serde_json::from_str(
        r#"{"methods":{"pay_invoice":{"rate":{"amount":5,"per_secs":604800,"max_capacity":5}}}}"#,
    )
    .unwrap();
    let rule = p.allows_wallet("pay_invoice").unwrap().expect("a limit");
    assert_eq!(rule.amount, 5);
    assert_eq!(rule.per_secs, 604_800);
}

#[test]
fn others_as_a_method_key_covers_methods_not_named() {
    let p: UsageProfile =
        serde_json::from_str(r#"{"methods":{"OTHERS":{},"get_info":{}}}"#).unwrap();
    assert!(p.allows_wallet("get_info").is_ok());
    assert!(p.allows_wallet("anything_at_all").is_ok());
}

#[test]
fn a_method_absent_with_no_others_is_restricted() {
    let p: UsageProfile = serde_json::from_str(r#"{"methods":{"get_info":{}}}"#).unwrap();
    assert_eq!(p.allows_wallet("pay_invoice"), Err(Denied::Restricted));
    assert_eq!(
        serde_json::from_str::<UsageProfile>("{}").unwrap().allows_wallet("get_info"),
        Err(Denied::Restricted)
    );
}

#[test]
fn the_three_maps_are_separate() {
    let p: UsageProfile = serde_json::from_str(r#"{"methods":{"OTHERS":{}}}"#).unwrap();
    assert!(p.allows_wallet("pay_invoice").is_ok());
    // Being allowed to spend does not make you an administrator...
    assert_eq!(p.allows_control("close_channel"), Err(Denied::Restricted));
    // ...nor entitle you to be told anything.
    assert!(!p.allows_notification("channel_closed"));

    let c: UsageProfile = serde_json::from_str(r#"{"control":{"OTHERS":{}}}"#).unwrap();
    assert_eq!(c.allows_wallet("pay_invoice"), Err(Denied::Restricted));
    assert!(!c.allows_notification("channel_closed"));
}

#[test]
fn per_secs_zero_denies_unauthorized_and_does_not_leave_the_old_grant_standing() {
    let w = world();
    let c = w.controller.public_key().to_hex();
    let mut g = Grants::new(w.node.public_key(), vec![w.owner.public_key()]);
    g.apply(&grant_event(&w.owner, &w.node.public_key(), &c, FULL, 1)).unwrap();

    let bad = r#"{"methods":{"get_balance":{"rate":{"amount":1,"per_secs":0}}}}"#;
    g.apply(&grant_event(&w.owner, &w.node.public_key(), &c, bad, 2)).unwrap();

    let p = g.resolve(&w.controller.public_key()).unwrap();
    assert!(!p.profile().is_valid());
    // UNAUTHORIZED, not RESTRICTED: the grant does not apply at all.
    assert_eq!(p.profile().allows_wallet("get_balance"), Err(Denied::Unauthorized));
    // And the older, valid grant is gone rather than still in force.
    assert_eq!(p.profile().allows_control("list_channels"), Err(Denied::Unauthorized));
}
