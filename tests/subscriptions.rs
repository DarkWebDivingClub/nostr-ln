//! Subscriptions are wants; grants are permissions. Delivery is the
//! intersection.

use nostr::event::{Event, EventBuilder, Kind, Tag};
use nostr::key::{Keys, PublicKey};
use nostr::signer::NostrSigner;
use nostr::types::Timestamp;
use nostr_ln::subscription::Rejected;
use nostr_ln::*;

fn ev(author: &Keys, kind: u16, p: &PublicKey, d: &str, content: &str, at: u64) -> Event {
    EventBuilder::new(Kind::Custom(kind), content)
        .tags([
            Tag::identifier(d.to_string()),
            Tag::parse(vec!["p".to_string(), p.to_hex()]).unwrap(),
        ])
        .custom_created_at(Timestamp::from(at))
        .sign_with_keys(author)
        .unwrap()
}

fn grant(owner: &Keys, node: &PublicKey, controller: &str, content: &str) -> Event {
    ev(owner, GRANT_KIND, node, &format!("{}:{}", node.to_hex(), controller), content, 1)
}

/// A subscription, with its content NIP-44'd to the node as the spec now
/// requires. Encrypting here rather than in each test keeps the tests
/// about the intersection rule, which is what they are for.
async fn sub(controller: &Keys, node: &PublicKey, types: &str, at: u64) -> Event {
    let content = controller.nip44_encrypt(node, types).await.unwrap();
    ev(controller, SUBSCRIPTION_KIND, node, &node.to_hex(), &content, at)
}

/// A subscription left in plaintext, as this kind used to be.
fn plaintext_sub(controller: &Keys, node: &PublicKey, types: &str, at: u64) -> Event {
    ev(controller, SUBSCRIPTION_KIND, node, &node.to_hex(), types, at)
}

const MAY_HEAR_CLOSES: &str = r#"{"control":{"list_channels":{}},"notifications":{"channel_closed":{}}}"#;

struct W {
    node: Keys,
    owner: Keys,
    carol: Keys,
}
fn w() -> W {
    W { node: Keys::generate(), owner: Keys::generate(), carol: Keys::generate() }
}

fn granted(w: &W, content: &str) -> Grants {
    let mut g = Grants::new(w.node.public_key(), vec![w.owner.public_key()]);
    g.apply(&grant(&w.owner, &w.node.public_key(), &w.carol.public_key().to_hex(), content))
        .unwrap();
    g
}

#[tokio::test]
async fn a_subscription_plus_a_permitting_grant_delivers() {
    let w = w();
    let g = granted(&w, MAY_HEAR_CLOSES);
    let mut s = Subscriptions::new(w.node.public_key());
    assert_eq!(s.apply(&sub(&w.carol, &w.node.public_key(), r#"["channel_closed"]"#, 5).await, &g, &w.node).await, Ok(1));
    assert!(s.should_receive(&w.carol.public_key(), "channel_closed", &g));
    assert_eq!(s.recipients("channel_closed", &g), vec![w.carol.public_key()]);
}

#[tokio::test]
async fn subscribing_to_a_type_the_grant_omits_yields_nothing() {
    let w = w();
    let g = granted(&w, MAY_HEAR_CLOSES);
    let mut s = Subscriptions::new(w.node.public_key());
    s.apply(&sub(&w.carol, &w.node.public_key(), r#"["channel_opened"]"#, 5).await, &g, &w.node).await.unwrap();
    assert!(!s.should_receive(&w.carol.public_key(), "channel_opened", &g));
    assert!(s.recipients("channel_opened", &g).is_empty());
}

#[tokio::test]
async fn being_permitted_a_type_never_subscribed_to_yields_nothing() {
    let w = w();
    let g = granted(&w, MAY_HEAR_CLOSES);
    let s = Subscriptions::new(w.node.public_key());
    assert!(!s.should_receive(&w.carol.public_key(), "channel_closed", &g));
}

#[tokio::test]
async fn narrowing_the_grant_stops_delivery_without_the_subscription_changing() {
    // "A subscription that outlived its grant would be a revocation that
    // does not revoke."
    let w = w();
    let mut g = granted(&w, MAY_HEAR_CLOSES);
    let mut s = Subscriptions::new(w.node.public_key());
    s.apply(&sub(&w.carol, &w.node.public_key(), r#"["channel_closed"]"#, 5).await, &g, &w.node).await.unwrap();
    assert!(s.should_receive(&w.carol.public_key(), "channel_closed", &g));

    // The owner narrows it. The subscription event is untouched.
    let narrowed = ev(
        &w.owner,
        GRANT_KIND,
        &w.node.public_key(),
        &format!("{}:{}", w.node.public_key().to_hex(), w.carol.public_key().to_hex()),
        r#"{"control":{"list_channels":{}}}"#,
        9,
    );
    g.apply(&narrowed).unwrap();

    assert!(!s.should_receive(&w.carol.public_key(), "channel_closed", &g));
    assert_eq!(s.len(), 1, "the subscription is inert, not deleted — the node does not own it");
}

#[tokio::test]
async fn a_controller_with_no_grant_gets_no_state_allocated() {
    // Anyone may publish a 30199 naming any node, so a registry keyed by
    // publishers is unbounded.
    let w = w();
    let g = Grants::new(w.node.public_key(), vec![w.owner.public_key()]);
    let mut s = Subscriptions::new(w.node.public_key());
    for _ in 0..50 {
        let stranger = Keys::generate();
        assert_eq!(
            s.apply(&sub(&stranger, &w.node.public_key(), r#"["channel_closed"]"#, 5).await, &g, &w.node).await,
            Err(Rejected::NoGrant)
        );
    }
    assert_eq!(s.len(), 0, "fifty strangers, no state");
}

#[tokio::test]
async fn a_subscription_whose_d_names_another_node_is_ignored() {
    let w = w();
    let g = granted(&w, MAY_HEAR_CLOSES);
    let elsewhere = Keys::generate().public_key();
    let mut s = Subscriptions::new(w.node.public_key());
    let e = ev(
        &w.carol,
        SUBSCRIPTION_KIND,
        &w.node.public_key(),
        &elsewhere.to_hex(),
        r#"["channel_closed"]"#,
        5,
    );
    assert_eq!(s.apply(&e, &g, &w.node).await, Err(Rejected::WrongTarget));
}

#[tokio::test]
async fn the_author_is_the_subscriber_by_construction() {
    // There is nothing else it could mean, so a subscription cannot name
    // somebody else: what it changes is keyed by who signed it.
    let w = w();
    let g = granted(&w, MAY_HEAR_CLOSES);
    let impostor = Keys::generate();
    let mut s = Subscriptions::new(w.node.public_key());
    // The impostor holds no grant, so their attempt is refused outright...
    assert_eq!(
        s.apply(&sub(&impostor, &w.node.public_key(), r#"["channel_closed"]"#, 5).await, &g, &w.node).await,
        Err(Rejected::NoGrant)
    );
    // ...and Carol's subscription is unaffected by anything they published.
    assert!(!s.should_receive(&w.carol.public_key(), "channel_closed", &g));
}

#[tokio::test]
async fn an_empty_array_unsubscribes() {
    let w = w();
    let g = granted(&w, MAY_HEAR_CLOSES);
    let mut s = Subscriptions::new(w.node.public_key());
    s.apply(&sub(&w.carol, &w.node.public_key(), r#"["channel_closed"]"#, 5).await, &g, &w.node).await.unwrap();
    assert_eq!(s.apply(&sub(&w.carol, &w.node.public_key(), "[]", 6).await, &g, &w.node).await, Ok(0));
    assert_eq!(s.len(), 0);
    assert!(!s.should_receive(&w.carol.public_key(), "channel_closed", &g));
}

#[tokio::test]
async fn publishing_again_replaces_rather_than_merges() {
    let w = w();
    let g = granted(
        &w,
        r#"{"notifications":{"channel_closed":{},"channel_opened":{}}}"#,
    );
    let mut s = Subscriptions::new(w.node.public_key());
    s.apply(&sub(&w.carol, &w.node.public_key(), r#"["channel_closed"]"#, 5).await, &g, &w.node).await.unwrap();
    s.apply(&sub(&w.carol, &w.node.public_key(), r#"["channel_opened"]"#, 6).await, &g, &w.node).await.unwrap();
    assert!(s.should_receive(&w.carol.public_key(), "channel_opened", &g));
    assert!(
        !s.should_receive(&w.carol.public_key(), "channel_closed", &g),
        "replaced, not merged"
    );
}

#[tokio::test]
async fn an_older_subscription_does_not_displace_a_newer_one() {
    let w = w();
    let g = granted(&w, r#"{"notifications":{"OTHERS":{}}}"#);
    let mut s = Subscriptions::new(w.node.public_key());
    s.apply(&sub(&w.carol, &w.node.public_key(), r#"["channel_opened"]"#, 100).await, &g, &w.node).await.unwrap();
    assert_eq!(
        s.apply(&sub(&w.carol, &w.node.public_key(), r#"["channel_closed"]"#, 50).await, &g, &w.node).await,
        Err(Rejected::Superseded)
    );
    assert!(s.should_receive(&w.carol.public_key(), "channel_opened", &g));
}

#[tokio::test]
async fn a_subscription_of_the_wrong_kind_is_not_a_subscription() {
    let w = w();
    let g = granted(&w, MAY_HEAR_CLOSES);
    let mut s = Subscriptions::new(w.node.public_key());
    let e = ev(
        &w.carol,
        30078,
        &w.node.public_key(),
        &w.node.public_key().to_hex(),
        r#"["channel_closed"]"#,
        5,
    );
    assert_eq!(s.apply(&e, &g, &w.node).await, Err(Rejected::WrongKind));
}

#[tokio::test]
async fn others_in_the_notifications_map_covers_types_not_named() {
    let w = w();
    let g = granted(&w, r#"{"notifications":{"OTHERS":{}}}"#);
    let mut s = Subscriptions::new(w.node.public_key());
    s.apply(&sub(&w.carol, &w.node.public_key(), r#"["something_new"]"#, 5).await, &g, &w.node).await.unwrap();
    assert!(s.should_receive(&w.carol.public_key(), "something_new", &g));
}

#[tokio::test]
async fn a_plaintext_subscription_is_not_applied() {
    // The shape this kind carried before it was encrypted. Refused as
    // undecryptable rather than accepted, so a stale publisher fails
    // loudly instead of subscribing to something nobody can audit.
    let w = w();
    let g = granted(&w, MAY_HEAR_CLOSES);
    let mut s = Subscriptions::new(w.node.public_key());
    let e = plaintext_sub(&w.carol, &w.node.public_key(), r#"["channel_closed"]"#, 5);
    assert_eq!(s.apply(&e, &g, &w.node).await, Err(Rejected::Undecryptable));
    assert!(s.recipients("channel_closed", &g).is_empty());
}

#[tokio::test]
async fn a_stranger_is_refused_before_anything_is_decrypted() {
    // Anyone may publish a kind 30199 naming any node. If decryption came
    // first, an unsolicited event would cost an ECDH — a bounded registry
    // turned into an amplifier. The grant check runs first, so a stranger
    // is refused as NoGrant, never as Undecryptable, whatever its content.
    let w = w();
    let g = granted(&w, MAY_HEAR_CLOSES);
    let mut s = Subscriptions::new(w.node.public_key());
    let stranger = Keys::generate();

    // Content this node genuinely cannot read: encrypted to somebody else.
    let junk = stranger.nip44_encrypt(&Keys::generate().public_key(), "[]").await.unwrap();
    let e = ev(
        &stranger,
        SUBSCRIPTION_KIND,
        &w.node.public_key(),
        &w.node.public_key().to_hex(),
        &junk,
        5,
    );

    // NoGrant, not Undecryptable: the ordering is observable in the error.
    assert_eq!(s.apply(&e, &g, &w.node).await, Err(Rejected::NoGrant));
}
