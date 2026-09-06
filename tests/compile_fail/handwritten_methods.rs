//! A hand-written `methods()` can disagree with what the impl provides,
//! which is the lie the macro exists to prevent.
use nostr_ln::service::ControlService;

struct Node;

#[nostr_ln::service]
impl ControlService for Node {
    fn methods(&self) -> &'static [&'static str] {
        &["open_channel", "everything_really"]
    }
}

fn main() {}
