//! Transports. A transport turns a target into a bidirectional byte stream.
//!
//! ⛔ Nothing above this module knows which target it is using. The proxy
//! binary and the pump both take a `Box<dyn Stream>`, so adding a transport
//! is a new arm in [`Dialer::dial`] and one type, not a change to the pump.
//!
//! ⛔ One read path and one write path. Every byte in comes through `Read`
//! and every byte out through `Write`. The drain thread below exists for one
//! reason: a pipe carries no read timeout, so a reader that handed a pipe to
//! the pump would stall it. The thread moves those bytes into a channel and
//! `read` pulls whatever has arrived, or reports `WouldBlock` when nothing
//! has. The pump itself stays single-threaded.

use std::fs::File;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio as ProcStdio};
use std::time::Duration;

use crate::error::{Error, Kind};

/// How long one DNS resolution may take. The dial timeout bounds the
/// connects; without this the lookup before them is unbounded, and a
/// stalled resolver wedges the whole dial.
pub const DNS_DEADLINE: Duration = Duration::from_secs(10);
/// How long one relay-leg write waits for the kernel to take it. A peer
/// that stops reading ends the op inside this bound instead of wedging it.
pub const WRITE_DEADLINE: Duration = Duration::from_secs(10);

/// A bidirectional byte stream with a read timeout. The timeout is what lets
/// the single-threaded [`crate::pump`] poll one direction, try the other,
/// and never deadlock.
pub trait Stream: Read + Write + Send {
    /// Bound how long `read` waits for a byte. The pump sets this itself.
    fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()>;
    /// Bound how long `write` waits for the kernel to take a byte. The
    /// relay legs set this at dial; a peer that stops reading ends the op
    /// inside the bound instead of wedging it.
    fn set_write_timeout(&self, d: Option<Duration>) -> io::Result<()>;
    /// Half-close the write side so the peer sees EOF and can finish. A
    /// blind pipe that never does this deadlocks the moment one side stops
    /// talking.
    fn shutdown_write(&mut self) -> io::Result<()>;
    /// The transport name for logs and error operations.
    fn describe(&self) -> &'static str {
        "stream"
    }
}

impl std::fmt::Debug for dyn Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stream({})", self.describe())
    }
}

/// What to dial. Three shapes, and no fourth hiding in a string: TCP for a
/// network hop, a Unix socket for a local server, a child process for
/// anything that carries a byte pipe on its standard input and output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Tcp { host: String, port: u16 },
    Unix { path: PathBuf },
    Exec { program: String, args: Vec<String> },
}

impl Target {
    /// Parse `host:port`, `[v6]:port`, or a bare `host` with `default_port`.
    ///
    /// ⛔ A bare host gets `default_port` and never a guessed port. A name
    /// with no dot in it is a name: `localhost` reaches the loopback
    /// resolver, and a parser that demands a dot sends it nowhere.
    pub fn parse_tcp(spec: &str, default_port: u16) -> Result<Target, Error> {
        let spec = spec.trim();
        if spec.is_empty() {
            return Err(Error::new(
                Kind::Usage,
                "target",
                "the destination is empty",
            ));
        }
        // Bracketed IPv6, with or without a port: [::1] or [::1]:22.
        if let Some(rest) = spec.strip_prefix('[') {
            let (host, tail) = rest.split_once(']').ok_or_else(|| {
                Error::new(
                    Kind::Usage,
                    "target",
                    format!("{spec:?} opens [ and never closes it"),
                )
            })?;
            if host.is_empty() {
                return Err(Error::new(
                    Kind::Usage,
                    "target",
                    format!("{spec:?} has an empty host"),
                ));
            }
            // The tail after `]` is `""` or `":port"`, so the leading colon
            // belongs to the separator, not to the port.
            let port = match tail.strip_prefix(':') {
                None if tail.is_empty() => default_port,
                None => {
                    return Err(Error::new(
                        Kind::Usage,
                        "target",
                        format!("{spec:?} has {tail:?} after ] where :port or nothing was meant"),
                    ));
                }
                Some(p) => parse_port(p)?,
            };
            return Ok(Target::Tcp {
                host: host.to_string(),
                port,
            });
        }
        // More than one colon and no brackets is a bare IPv6 literal, which
        // carries no port. `::1` is an address, not `host:port`.
        if spec.matches(':').count() > 1 {
            return Ok(Target::Tcp {
                host: spec.to_string(),
                port: default_port,
            });
        }
        match spec.split_once(':') {
            Some((host, p)) => {
                if host.is_empty() {
                    return Err(Error::new(
                        Kind::Usage,
                        "target",
                        format!("{spec:?} has no host"),
                    ));
                }
                Ok(Target::Tcp {
                    host: host.to_string(),
                    port: parse_port(p)?,
                })
            }
            None => Ok(Target::Tcp {
                host: spec.to_string(),
                port: default_port,
            }),
        }
    }

    /// The `host:port` spelling for logs and error operations.
    pub fn plain(&self) -> String {
        match self {
            Target::Tcp { host, port } => format!("{host}:{port}"),
            Target::Unix { path } => format!("unix {}", path.display()),
            Target::Exec { program, args } => {
                let mut s = program.clone();
                for a in args {
                    s.push(' ');
                    s.push_str(a);
                }
                s
            }
        }
    }
}

/// Parse a port string. Refused values are usage failures, not panics:
/// a caller that passes a non-numeric port must get an error, not an abort.
/// Port 0 is refused: a destination on port 0 is never what a caller meant.
pub fn parse_port(s: &str) -> Result<u16, Error> {
    let n: u32 = s
        .parse()
        .map_err(|_| Error::new(Kind::Usage, "target", format!("{s:?} is not a port")))?;
    if n == 0 {
        return Err(Error::new(
            Kind::Usage,
            "target",
            format!("{s:?}: port 0 names no service"),
        ));
    }
    if n > u16::MAX as u32 {
        return Err(Error::new(
            Kind::Usage,
            "target",
            format!("{s:?}: port {n} is over 65535"),
        ));
    }
    Ok(n as u16)
}

/// Dials targets. One dispatch, per-dial timeouts.
pub struct Dialer {
    /// How long one dial waits. Every address a name resolves to gets this
    /// long, so a dial is always bounded.
    pub timeout: Duration,
    /// The proxy of the environment for a TCP dial (`egress.rs`), or `None`:
    /// each TCP dial goes directly.
    egress: Option<pod_proxy::blocking::Dialer>,
}

impl Dialer {
    /// A dialer that dials each target directly. The `proxy` binary uses it:
    /// SSH names the target of a `ProxyCommand`, and SSH's own configuration
    /// chooses a proxy.
    pub fn new(timeout: Duration) -> Self {
        Dialer {
            timeout,
            egress: None,
        }
    }

    /// A dialer whose TCP dials take the proxy of the process environment
    /// (`https_proxy`, `HTTPS_PROXY`, `all_proxy`, `ALL_PROXY`, and the
    /// `NO_PROXY` list). The relay legs use it.
    pub fn from_environment(timeout: Duration) -> io::Result<Self> {
        Dialer::with_environment(
            timeout,
            pod_proxy::Environment::read(&|name| std::env::var_os(name)),
        )
    }

    /// A dialer whose TCP dials take the proxy of `env`. With no proxy in
    /// `env`, it is [`Dialer::new`], and no runtime of `pod-proxy` starts.
    pub fn with_environment(timeout: Duration, env: pod_proxy::Environment) -> io::Result<Self> {
        let egress = match env.proxy {
            Some(_) => Some(egress::dialer(env)?),
            None => None,
        };
        Ok(Dialer { timeout, egress })
    }

    /// Dial one target. This is the one dispatch: every caller reaches every
    /// transport through here.
    pub fn dial(&self, target: &Target) -> Result<Box<dyn Stream>, Error> {
        match target {
            Target::Tcp { host, port } => self.dial_tcp(host, *port),
            Target::Unix { path } => self.dial_unix(path),
            Target::Exec { program, args } => self.dial_exec(program, args),
        }
    }

    fn dial_tcp(&self, host: &str, port: u16) -> Result<Box<dyn Stream>, Error> {
        let op = format!("dial tcp {host}:{port}");
        // ⛔ The proxy of the environment first. A target that the rules send
        // directly (loopback, `NO_PROXY`) takes the direct dial below.
        if let Some(egress) = &self.egress
            && let Some(s) = egress::through_proxy(egress, host, port, self.timeout)
                .map_err(|e| Error::dial(op.clone(), e))?
        {
            return Ok(s);
        }
        let mut last = io::Error::new(
            io::ErrorKind::NotFound,
            format!("no address for {host}:{port}"),
        );
        // ⛔ Every resolved address is tried, in order. The first address is
        // not always the reachable one.
        // ⛔ The lookup itself is bounded: `to_socket_addrs` carries no
        // deadline, so it runs on one worker and `recv_timeout` ends the
        // dial when the resolver stalls. The bound is the tighter of the
        // dial timeout and `DNS_DEADLINE`, and the failure names the host.
        let deadline = std::cmp::min(self.timeout, DNS_DEADLINE);
        let addrs =
            resolve_bounded(host, port, deadline).map_err(|e| Error::dial(op.clone(), e))?;
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, self.timeout) {
                Ok(s) => {
                    // ⛔ Best-effort latency hint. A stream that cannot set
                    // it still carries bytes, so this must not fail the dial.
                    let _ = s.set_nodelay(true);
                    return Ok(Box::new(Tcp(s)));
                }
                Err(e) => last = e,
            }
        }
        Err(Error::dial(op, last))
    }

    fn dial_unix(&self, path: &Path) -> Result<Box<dyn Stream>, Error> {
        let op = format!("dial unix {}", path.display());
        match UnixStream::connect(path) {
            Ok(s) => Ok(Box::new(Unix(s))),
            Err(e) => Err(Error::dial(op, e)),
        }
    }

    fn dial_exec(&self, program: &str, args: &[String]) -> Result<Box<dyn Stream>, Error> {
        let op = format!("dial exec {program}");
        match Exec::spawn(program, args) {
            Ok(s) => Ok(s),
            Err(e) => Err(Error::dial(op, e)),
        }
    }
}

mod bounded;
pub use bounded::*;
mod egress;

#[cfg(test)]
mod tests;
