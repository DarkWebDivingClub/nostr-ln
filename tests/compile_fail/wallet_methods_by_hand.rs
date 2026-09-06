//! A `WalletService` may not write its own `methods()`.
//!
//! It could until mission 18.1, and had to: the trait had one `call` entry
//! point, so `#[nostr_ln::service]` found nothing in the impl block and
//! emitted an empty list. A wallet therefore maintained the list by hand
//! and carried the weakness the macro exists to remove — a declaration that
//! can disagree with what it answers.
//!
//! Now the methods are typed, the macro works, and a hand-written list is
//! refused here exactly as it already was for `ControlService`.

use nostr_ln::nnc::NncError;
use nostr_ln::nwc::methods::*;
use nostr_ln::service::handler::Fut;
use nostr_ln::service::{Caller, WalletService};

struct Wallet;

#[nostr_ln::service]
impl WalletService for Wallet {
    fn methods(&self) -> &'static [&'static str] {
        &["get_balance", "pay_invoice", "and_whatever_else_i_fancy"]
    }

    fn get_balance<'a>(
        &'a self,
        _r: GetBalanceRequest,
        _c: Caller<'a>,
    ) -> Fut<'a, Result<GetBalanceResponse, NncError>> {
        Box::pin(async move { Ok(GetBalanceResponse { balance: 0 }) })
    }
}

fn main() {}
