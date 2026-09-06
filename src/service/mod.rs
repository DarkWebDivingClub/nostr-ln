//! The service side: what a node implements, and what runs around it.
//!
//! A node writes a handler. This crate owns everything else — grants,
//! limits, dispatch, and the pipeline that orders them. **Upstream has no
//! service side at all**, for NWC or NNC, which is why five hand-rolled
//! ones exist in this project and why their bugs differ.

pub mod dispatch;
pub mod handler;
pub mod pipeline;
pub mod state;

#[cfg(feature = "transport")]
pub mod transport;

pub use dispatch::{dispatch_control, dispatch_wallet};
pub use handler::{Caller, ControlService, Prepared, WalletService};
pub use pipeline::{handle, Handler, Protocol};
pub use state::Usage;

#[cfg(feature = "transport")]
pub use transport::{
    Notifier, Service, CONTROL_INFO_KIND, CONTROL_REQUEST_KIND, CONTROL_RESPONSE_KIND,
    NOTIFICATION_KIND, WALLET_INFO_KIND, WALLET_REQUEST_KIND, WALLET_RESPONSE_KIND,
};
