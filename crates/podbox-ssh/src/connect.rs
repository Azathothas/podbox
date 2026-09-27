//! Operator side. `podssh connect` is an ssh `ProxyCommand`: ssh runs it,
//! writes the ssh protocol to its stdin, and reads the server's answers from
//! its stdout. podssh moves those bytes to the relay and back.
//!
//! ⛔ Nothing custom is required at the operator end. The result is a normal
//! `ssh <name>` line, so `scp`, `rsync`, `git`, `sshfs` and every editor that
//! speaks ssh keep working.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use crate::protocol::{self, Protocol, Role};
use crate::relay::PUMP_TIMEOUT;
use crate::transport::{Dialer, Stdio, Stream};

pub struct Config {
    pub relays: Vec<String>,
    pub name: String,
    pub auth: String,
    pub protocol: Protocol,
    pub dialer: Dialer,
    /// Keep retrying a relay that has no node yet for this long. A
    /// `ProxyCommand` is started the moment ssh needs it, which can be a
    /// second before the node registers after a redial.
    pub wait: Duration,
    pub handshake_timeout: Duration,
}

pub fn run(cfg: &Config) -> io::Result<()> {
    if cfg.relays.is_empty() {
        return Err(io::Error::other(
            "no relay. Pass --relay <url> (or set PODSSH_RELAY).",
        ));
    }
    let deadline = Instant::now() + cfg.wait;
    let mut last = io::Error::other("no relay tried");
    loop {
        for relay in &cfg.relays {
            match session(cfg, relay) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    eprintln!("podssh connect: {relay}: {e}");
                    last = e;
                }
            }
        }
        if Instant::now() >= deadline {
            return Err(io::Error::other(format!(
                "no relay paired before --wait {}s; last: {last}",
                cfg.wait.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn session(cfg: &Config, relay: &str) -> io::Result<()> {
    let mut stream = cfg.dialer.dial(relay)?;
    stream.set_read_timeout(Some(cfg.handshake_timeout))?;
    protocol::greet(
        &mut *stream,
        cfg.protocol,
        Role::Client,
        &cfg.name,
        &cfg.auth,
        cfg.handshake_timeout,
    )?;
    eprintln!("podssh connect: paired via {relay}");
    let mut io = Stdio::new()?;
    stream.set_read_timeout(Some(PUMP_TIMEOUT))?;
    crate::util::pump(&mut io, &mut *stream)
}

/// `podssh forward`: reach a fixed `host:port` through the dialer with no
/// rendezvous. This is the shape for a public egress relay that forwards to a
/// concrete ssh server, and for a plain jump host.
pub struct ForwardConfig {
    pub target: String,
    pub expect_banner: Option<String>,
    pub dialer: Dialer,
}

pub fn forward(cfg: &ForwardConfig) -> io::Result<()> {
    let (host, port) = split(&cfg.target)?;
    let mut stream = cfg.dialer.dial_addr(&host, port)?;
    if let Some(prefix) = &cfg.expect_banner {
        let banner = read_banner(&mut *stream, Duration::from_secs(10))?;
        if !banner.starts_with(prefix.as_bytes()) {
            return Err(io::Error::other(format!(
                "target banner {:?} does not start with {:?}; refusing to splice",
                String::from_utf8_lossy(&banner),
                prefix
            )));
        }
        // The banner is the first thing the ssh client reads; it was consumed
        // to check the target, so it has to be handed back before the splice.
        let mut out = Stdio::new()?;
        out.write_all(&banner)?;
        out.flush()?;
        stream.set_read_timeout(Some(PUMP_TIMEOUT))?;
        return crate::util::pump(&mut out, &mut *stream);
    }
    let mut io = Stdio::new()?;
    stream.set_read_timeout(Some(PUMP_TIMEOUT))?;
    crate::util::pump(&mut io, &mut *stream)
}

fn split(addr: &str) -> io::Result<(String, u16)> {
    let (host, port) = addr
        .rsplit_once(':')
        .ok_or_else(|| io::Error::other(format!("{addr:?} is not host:port")))?;
    Ok((
        host.to_string(),
        port.parse::<u16>()
            .map_err(|_| io::Error::other(format!("{addr:?} has a bad port")))?,
    ))
}

fn read_banner(s: &mut dyn Stream, timeout: Duration) -> io::Result<Vec<u8>> {
    s.set_read_timeout(Some(timeout))?;
    let mut out = Vec::new();
    let mut b = [0u8; 1];
    while out.len() < 255 {
        match s.read(&mut b) {
            Ok(0) => break,
            Ok(_) => {
                out.push(b[0]);
                if out.ends_with(b"\r\n") || out.ends_with(b"\n") {
                    return Ok(out);
                }
            }
            Err(e) if crate::util::is_would_block(&e) => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "no ssh banner from target",
                ))
            }
            Err(e) => return Err(e),
        }
    }
    Ok(out)
}
