//! The SSH transport and server: a byte stream under a real SSH server.
//! `TODO/podssh.md` T-1401.
//!
//! The relay legs live here too: [`mux`] speaks the multiplexed reverse
//! protocol (`reverse-v1`) on the node and operator sides, over the [`ws`]
//! framing and the [`tls`] client. The one-pair rendezvous for local paths
//! is not in this crate; TODO/podssh.md T-1403 records the split and the
//! remainder.
//!
//! ⭐ **This crate takes the mechanism of podbox pull request 67 and drops
//! what T-1401 defers.** The stream trait with its read timeout, the one
//! dispatch that turns a target into a stream, the single-threaded pump, the
//! probe that starts a server briefly, and the generated server config all
//! come from that source. The chain, the resolver, the probe page, the
//! catalog, the target URL grammar, and the main binary stay out. No dead
//! code for later.
//!
//! ⚠ **A probe that passes is not proof of SSH speech.** A server that is
//! still running after the probe window started. Whether it speaks SSH shows
//! at key exchange with a real client, which is the better place to find out
//! than a guess made here.
//!
//! ⚠ **This crate builds on Unix only.** The transport uses Unix sockets and
//! child stdio pipes. The Windows host cannot compile it; the Linux lane
//! builds and tests it.

#![forbid(unsafe_code)]

pub mod error;
pub mod mux;
pub mod pump;
pub mod server;
pub mod tls;
pub mod transport;
pub mod ws;

pub use error::{Error, Kind, EXIT_RUNTIME_ERROR};
pub use pump::{is_would_block, pump, READ_TIMEOUT};
pub use server::{probe_server, spawn_stdio, write_sshd_config, ServerChild, PROBE_WINDOW};
pub use transport::{parse_port, Dialer, Stdio, Stream, Target};
