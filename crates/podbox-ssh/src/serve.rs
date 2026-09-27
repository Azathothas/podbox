//! Agent side. The sandbox dials out, registers a name, waits, and when a
//! client pairs it runs one ssh server on the relay's bytes.
//!
//! ⛔ No listening socket is ever created here. The cage denies `bind(2)`, so
//! the only shape that works is register-and-wait, which is also the shape
//! that needs no port forwarding and no firewall change anywhere.

use std::io;
use std::time::Duration;

use crate::protocol::{self, Protocol, Role};
use crate::relay::PUMP_TIMEOUT;
use crate::sshserver::{self, ServerSpec};
use crate::transport::Dialer;

pub struct Config {
    pub relays: Vec<String>,
    pub name: String,
    pub auth: String,
    pub protocol: Protocol,
    pub dialer: Dialer,
    pub server: ServerSpec,
    pub once: bool,
    /// How long a registered node waits for a client before redialing.
    pub handshake_timeout: Duration,
}

impl Config {
    pub fn new(relays: Vec<String>, name: String, dialer: Dialer, server: ServerSpec) -> Self {
        Config {
            relays,
            name,
            auth: String::new(),
            protocol: Protocol::Podssh1,
            dialer,
            server,
            once: false,
            handshake_timeout: Duration::from_secs(900),
        }
    }
}

pub fn run(cfg: &Config) -> io::Result<()> {
    if cfg.relays.is_empty() {
        return Err(io::Error::other(
            "no relay. Pass --relay <url> (or set PODSSH_RELAY). \
             Run `podssh relay` on any host with a public address to create one.",
        ));
    }
    let mut backoff = 1u64;
    loop {
        for relay in &cfg.relays {
            match session(cfg, relay) {
                Ok(()) => {
                    eprintln!("podssh serve: session ended; redialing");
                    backoff = 1;
                }
                Err(e) => {
                    eprintln!("podssh serve: {relay}: {e}; retry in {backoff}s");
                }
            }
            if cfg.once {
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_secs(backoff));
        backoff = (backoff * 2).min(30);
    }
}

fn session(cfg: &Config, relay: &str) -> io::Result<()> {
    let mut stream = cfg.dialer.dial(relay)?;
    stream.set_read_timeout(Some(cfg.handshake_timeout))?;
    protocol::greet(
        &mut *stream,
        cfg.protocol,
        Role::Node,
        &cfg.name,
        &cfg.auth,
        cfg.handshake_timeout,
    )?;
    eprintln!(
        "podssh serve: paired via {relay}; starting {}",
        cfg.server_name()
    );
    let mut handle = sshserver::start(&cfg.server)?;
    stream.set_read_timeout(Some(PUMP_TIMEOUT))?;
    handle.stream.set_read_timeout(Some(PUMP_TIMEOUT))?;
    crate::util::pump(&mut *stream, &mut *handle.stream)
}

impl Config {
    pub fn server_name(&self) -> String {
        match &self.server {
            ServerSpec::Auto => "an auto-detected ssh server".to_string(),
            ServerSpec::Command(c) => format!("`{c}`"),
            ServerSpec::Forward(a) => format!("the ssh server at {a}"),
        }
    }
}
