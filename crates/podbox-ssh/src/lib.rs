//! podssh: native ssh reachability into a sealed sandbox, over whatever the
//! network still allows, with vanilla ssh on the operator end.
//!
//! The shape is two outbound dials and a blind relay. The agent registers a
//! name; the operator claims it and runs `ssh`. Nothing listens in the cage,
//! no port is forwarded, and no custom client is required at either end.
//!
//! ⛔ Every transport is a [`transport::Stream`], so the protocol, the relay,
//! the server, the client and the probe share one code path. A new transport
//! is a new arm in [`transport::Dialer::dial`], not a new mode.
#![forbid(unsafe_code)]

pub mod catalog;
pub mod chain;
pub mod cli;
pub mod connect;
pub mod error;
pub mod probe;
pub mod protocol;
pub mod relay;
pub mod resolver;
pub mod serve;
pub mod sshserver;
pub mod target;
pub mod transport;
pub mod url;
pub mod util;

// Re-exports, so a caller names one module.
pub use error::{Error, Exit, Kind, Result};
pub use target::Target;
pub use transport::{Dialer, Stream};

/// The version every user-agent and `--json` document carries.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
