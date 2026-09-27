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
pub mod cli;
pub mod connect;
pub mod probe;
pub mod protocol;
pub mod relay;
pub mod serve;
pub mod sshserver;
pub mod transport;
pub mod url;
pub mod util;
