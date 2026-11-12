//! The relay legs of the multiplexed reverse protocol, `reverse-v1`, over
//! `pod-relay` (PodUtils/podcrates): its blocking facade runs the node and
//! the operator on a runtime of its own, so podvm's code stays synchronous.
//! TODO/deps.md T-1678.
//!
//! ## The wire
//!
//! `reverse-v1` is the relay's `hello` with `version` 1, which `pod-relay`
//! reads as its dialect 1; a relay of another dialect ends the node. The
//! relay's document is captured at
//! `docs/history/references/relay-index-2026-09-28-r12.md`:
//!
//! | role | address | token |
//! | --- | --- | --- |
//! | node | `wss://HOST/v1/node/{name}` | `node_token` in `X-Relay-Token` |
//! | operator | `wss://HOST/v1/connect/{name}` | `connect_token` in `X-Relay-Token` |
//!
//! The crate owns the framing, the control messages, the caps that the
//! `hello` states, the keepalive and the redial. This module owns podvm's
//! side: the flags and their checks, the TLS roots and the proxy of a leg,
//! the SSH server of each session, and the lines and the counts of the
//! binaries. Both legs carry the bytes of a session as they are: podvm runs
//! no end-to-end channel of the crate, as the operators of `reverse-v1` run
//! none.
//!
//! ⛔ Tokens never enter a log line or an error string. They travel in
//! `X-Relay-Token`, never in a URL, because a URL is written to every access
//! log on the way and a header is not.

use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use pod_relay::blocking::{
    Client, Config, Local, NodeChannel, NodeOptions, Operator, OperatorChannel, Stopper,
};
use pod_relay::pair::Pair;
use pod_relay::reverse::{Exit, OperatorLimits, Outcome, Wire};
use pod_relay::{Context, Relay};

use crate::error::{Error, Kind};
use crate::transport::WRITE_DEADLINE;

/// The relay protocol this module speaks: the `hello` of version 1.
pub const PROTOCOL_VERSION: &str = "reverse-v1";
/// The bound on each connection to the relay: the dial, the proxy, TLS and
/// the upgrade.
pub const DIAL_TIMEOUT: Duration = Duration::from_secs(10);
/// How long the operator waits for `ready` by default.
const DEFAULT_READY_WAIT: Duration = Duration::from_secs(20);
/// How long the operator waits for the relay's goodbye after stdin ends.
const GOODBYE_WAIT: Duration = Duration::from_secs(10);
/// How long a node that ended waits for its sessions' servers to end, so
/// that its counts are whole.
const SESSIONS_END_WAIT: Duration = Duration::from_secs(5);

/// Where a normal Linux host keeps its trust anchors, in load order. The
/// first one with a usable root is read, beside the compiled-in roots.
pub const HOST_BUNDLES: &[&str] = &[
    "/etc/ssl/certs/ca-certificates.crt",
    "/etc/pki/tls/certs/ca-bundle.crt",
    "/etc/ssl/ca-bundle.pem",
    "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
];

mod origin;
pub use origin::parse_origin;

/// A pair name travels as a URL path segment: the crate's rule, which
/// refuses a name that could change the path, before any dial.
pub fn validate_name(name: &str) -> Result<(), Error> {
    pod_relay::relay::check_node_name(name)
        .map_err(|why| Error::new(Kind::Usage, "relay name", why))
}

/// A token travels as an HTTP header value: 8 to 4096 visible ASCII
/// characters, the shape that the relay gives. The error never shows it.
pub fn validate_token(token: &str) -> Result<(), Error> {
    if pod_relay::cache::valid_token(token) {
        return Ok(());
    }
    Err(Error::new(
        Kind::Usage,
        "relay token",
        "the relay token is not 8 to 4096 visible ASCII characters",
    ))
}

/// The way out of each relay leg: the TLS of `pod-tls` with the first host
/// bundle beside the compiled-in roots, and the proxy of the environment
/// (`HTTPS_PROXY`, `ALL_PROXY`, `NO_PROXY`; a loopback relay goes directly).
/// No file is kept: the pair's tokens come from the user.
pub fn context() -> Result<Context, Error> {
    let mut words = pod_tls::Words::new("podvm");
    words.replace_roots = String::new();
    words.add_roots = Some(format!("the first host CA bundle ({})", HOST_BUNDLES[0]));
    let mut settings = pod_tls::Settings::new(words);
    settings.system_bundles = HOST_BUNDLES.iter().map(PathBuf::from).collect();
    let tls = pod_tls::Context::new(Arc::new(rustls::crypto::ring::default_provider()), settings);
    // The configuration is made here: a failure names the roots, not a dial.
    tls.config_for(&pod_tls::Trust::Default)
        .map_err(|e| Error::dial("relay tls roots", io::Error::other(e.to_string())))?;
    let env = pod_proxy::Environment::read(&|name| std::env::var_os(name));
    let proxy = pod_proxy::Settings::new("podvm", pod_proxy::ProxyChoice::FromEnvironment(env));
    let dial = pod_proxy::Dialer::new(proxy, Arc::new(pod_proxy::resolve::System::new()));
    let agent = concat!("podvm/", env!("CARGO_PKG_VERSION"));
    let way = pod_ws::Way::new(Arc::new(tls), Arc::new(dial), agent);
    Ok(Context::new(way, pod_relay::cache::Cache::none()))
}

/// The client of the relay at `origin`, and the relay: TLS for `wss://`,
/// plain WebSocket for `ws://` on the loopback.
fn client(origin: &str) -> Result<(Client, Relay), Error> {
    let (tls, host, port) = parse_origin(origin)?;
    let wire = if tls { Wire::Tls } else { Wire::PlainLoopback };
    let config = Config {
        wire,
        timeout: DIAL_TIMEOUT,
        ..Config::new(context()?)
    };
    let client = Client::new(config)
        .map_err(|e| Error::new(Kind::Dial, format!("relay {origin}"), e.to_string()))?;
    Ok((client, Relay { host, port }))
}

// ------------------------------------------------------------ the node

/// What the node process needs. Tokens ride environment or flags into here;
/// they never come back out in an error.
pub struct NodeConfig {
    pub relay: String,
    pub name: String,
    pub node_token: String,
    /// Server command for every session. `None` runs detection once at
    /// startup, so a machine with no runnable server fails before dialing.
    pub server_command: Option<String>,
    /// Exit after the first socket ends, or at the first try that gives no
    /// socket. The drive and the tests use this; service use redials.
    pub once: bool,
}

/// What the node carried.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NodeStats {
    /// Sockets that the relay took (redials included).
    pub sockets: u64,
    /// Sessions whose server started and then ended.
    pub sessions_completed: u64,
    /// Sessions whose server could not start. The crate refuses a session
    /// past the relay's `maxSessions` itself; such a refusal is not counted.
    pub sessions_refused: u64,
}

mod node;

/// Run the node: register, serve every session on the one socket, redial on
/// loss. Each session runs the server command on a socketpair.
pub fn run_node(cfg: &NodeConfig) -> Result<NodeStats, Error> {
    validate_name(&cfg.name)?;
    validate_token(&cfg.node_token)?;
    let server_command = match &cfg.server_command {
        Some(c) if !c.trim().is_empty() => c.clone(),
        _ => crate::server::detect()?,
    };
    let (client, relay) = client(&cfg.relay)?;
    let mut pair = Pair::for_node(relay, &cfg.name, &cfg.node_token)
        .map_err(|e| Error::new(Kind::Usage, "relay pair", e.to_string()))?;
    let tally = Arc::new(node::Tally::default());
    let stopper = Stopper::new();
    let options = NodeOptions {
        channel: NodeChannel::Off,
        say: Some(node::say(cfg, tally.clone(), stopper.clone())),
        ..NodeOptions::default()
    };
    let handler = {
        let tally = tally.clone();
        move |_id: &str| node::open_server(&server_command, &tally)
    };
    let op = format!("relay node {}", cfg.relay);
    let exit = client
        .run_node(&mut pair, handler, &options, &stopper)
        .map_err(|e| Error::new(Kind::Dial, op.clone(), e.to_string()));
    // ⛔ The facade's runtime runs only inside its calls: the last steps of
    // the sessions (the drain after the relay's close) end when the client
    // is dropped, and only then does each server end and count.
    drop(client);
    tally.wait_for_sessions(SESSIONS_END_WAIT);
    let stats = tally.stats();
    match (exit?, tally.failure()) {
        // `--once` stopped the node at a try that gave no socket.
        (Exit::Stopped, Some(why)) => Err(Error::new(Kind::Dial, op, why)),
        (Exit::Stopped, None) => Ok(stats),
        // With `--once`, the relay's Close that ended the pair ended the
        // first socket too.
        (Exit::PairStopped | Exit::PairExpired, None) if cfg.once && stats.sockets > 0 => Ok(stats),
        (exit, _) => Err(Error::new(Kind::Dial, op, node::exit_words(&exit))),
    }
}

// ------------------------------------------------------------ the operator

/// What the operator process needs. Standard input and output are the SSH
/// client's; this process is a valid `ProxyCommand`.
pub struct OperatorConfig {
    pub relay: String,
    pub name: String,
    pub connect_token: String,
    /// How long to wait for the node's `ready` before failing.
    pub ready_wait: Duration,
}

impl OperatorConfig {
    pub fn new(relay: String, name: String, connect_token: String) -> Self {
        OperatorConfig {
            relay,
            name,
            connect_token,
            ready_wait: DEFAULT_READY_WAIT,
        }
    }
}

/// Run one operator session: what `input` gives goes to the node's server,
/// and what comes back goes to `output`. Input before `ready` is kept, up to
/// 1 MiB; the session ends at the end of `input`, after the relay's goodbye
/// or `GOODBYE_WAIT`.
pub fn run_operator<R, W>(cfg: &OperatorConfig, input: R, output: W) -> Result<(), Error>
where
    R: Read + Send + 'static,
    W: Write + Send + 'static,
{
    validate_name(&cfg.name)?;
    validate_token(&cfg.connect_token)?;
    let (client, relay) = client(&cfg.relay)?;
    let to = Operator {
        relay: &relay,
        name: &cfg.name,
        connect_token: &cfg.connect_token,
        limits: OperatorLimits {
            ready_limit: cfg.ready_wait,
            close_limit: GOODBYE_WAIT,
            ..OperatorLimits::default()
        },
        channel: OperatorChannel::Off,
    };
    let session = "mux operator session";
    match client.run_operator(&to, input, output) {
        Ok(outcome) if outcome.is_success() => Ok(()),
        Ok(Outcome::NeverReady { code, reason }) => {
            let code = code
                .map(|c| format!(" (relay close {c})"))
                .unwrap_or_default();
            Err(Error::new(
                Kind::Pump,
                session,
                format!("the session never started: {reason}{code}"),
            ))
        }
        Ok(Outcome::Ended { code, reason }) => Err(Error::new(
            Kind::Pump,
            session,
            format!("relay closed the session: {code} {reason}"),
        )),
        Ok(Outcome::LocalEnd) => Ok(()),
        Err(e) => Err(Error::new(
            Kind::Dial,
            format!("relay operator {}", cfg.relay),
            e.to_string(),
        )),
    }
}

#[cfg(test)]
mod tests;
