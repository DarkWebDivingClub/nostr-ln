//! Forgetting `#[nostr_ln::service]` must be a compile error, not a node
//! that advertises nothing and denies everything at runtime.
use nostr_ln::service::ControlService;

struct Node;

// No #[nostr_ln::service], so `methods()` is never generated — and the
// trait gives it no default, deliberately.
impl ControlService for Node {}

fn main() {}
