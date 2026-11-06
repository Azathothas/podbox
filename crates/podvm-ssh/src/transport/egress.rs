//! The proxy of a TCP dial, by the rules of `pod-proxy` (PodUtils/podcrates):
//! `https_proxy`, `HTTPS_PROXY`, `all_proxy`, `ALL_PROXY` and the `NO_PROXY`
//! list. A loopback target never goes through a proxy.
//!
//! ⛔ `pod-proxy` decides, and it carries a dial through a proxy. A target
//! that goes directly keeps the direct dial of [`super::Dialer`]: its bound
//! on the resolution, and each address in turn with the whole timeout.

use std::io;
use std::sync::Arc;
use std::time::Duration;

use pod_proxy::blocking;
use pod_proxy::resolve::System;
use pod_proxy::{Environment, ProxyChoice, Settings};

use super::{Stream, Tcp, Unix};

/// The blocking dialer of `pod-proxy` with the proxy of `env` and the
/// resolver of the system. It has its own runtime of one thread.
pub(super) fn dialer(env: Environment) -> io::Result<blocking::Dialer> {
    blocking::Dialer::new(
        Settings::new("podvm", ProxyChoice::FromEnvironment(env)),
        Arc::new(System::new()),
    )
}

/// The stream to `host:port` through the proxy that the rules choose, or
/// `None` when the target goes directly. A proxy setting that cannot be
/// used is an error: a silent direct dial would go around the proxy.
pub(super) fn through_proxy(
    egress: &blocking::Dialer,
    host: &str,
    port: u16,
    timeout: Duration,
) -> io::Result<Option<Box<dyn Stream>>> {
    let proxy = egress
        .proxy_at(host, port)
        .map_err(|why| io::Error::other(format!("the proxy setting is not usable: {why}")))?;
    let Some(proxy) = proxy else {
        return Ok(None);
    };
    let (stream, _) = egress
        .stream(host, port, timeout)
        .map_err(|e| io::Error::other(e.to_string()))?;
    match stream {
        blocking::Stream::Tcp(s) => Ok(Some(Box::new(Tcp(s)))),
        blocking::Stream::Unix(s) => Ok(Some(Box::new(Unix(s)))),
        // ⛔ A variable cannot name a proxy at a command (`pod-proxy` takes
        // one only from a value of the user), and such a stream has no read
        // timeout, which the pump needs.
        blocking::Stream::Exec(_) => Err(io::Error::other(format!(
            "the proxy {} is a command, and a TCP dial takes a socket only",
            proxy.label()
        ))),
    }
}

#[cfg(test)]
mod tests;
