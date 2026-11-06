//! TLS for the relay legs, client side only.
//!
//! rustls is the transport TODO/deps.md T-0905 ruled in; `ring` is its
//! crypto provider and already builds in this workspace. Verification is
//! always on: there is no `--insecure` flag, because a relay that can swap
//! certificates can deny service and observe timing, and a test relay that
//! needs no verification speaks plain `ws` on loopback instead.
//!
//! The trust anchors and the handshake are the ones of `pod-tls`
//! (PodUtils/podcrates), with `ring` passed in: the first readable host
//! bundle, because a cage with an intercepting proxy puts its CA in the host
//! bundle, and the compiled-in `webpki-roots`, for a machine with no bundle at
//! all. Either way the default is verify, never trust-anything.

use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use pod_tls::{Context, Settings, Trust, Words};

use crate::transport::Stream;

/// Where a normal Linux host keeps its trust anchors, in load order. The
/// first one with a usable root is read.
pub const HOST_BUNDLES: &[&str] = &[
    "/etc/ssl/certs/ca-certificates.crt",
    "/etc/pki/tls/certs/ca-bundle.crt",
    "/etc/ssl/ca-bundle.pem",
    "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
];

/// A client TLS configuration, shared by every dial on one process.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    context: Arc<Context>,
}

impl TlsConfig {
    /// Build the root store and the configuration over it. An unreadable
    /// bundle is skipped and an empty one adds nothing; there is no refusal
    /// when no host bundle exists, because the compiled-in roots are the
    /// design for a machine with none at all, and a client with them still
    /// verifies everything.
    pub fn load() -> io::Result<Self> {
        // The words of an unknown issuer name the one place that podvm-ssh
        // reads a CA from.
        let mut words = Words::new("podvm");
        words.replace_roots = format!("the first host CA bundle ({})", HOST_BUNDLES[0]);
        let mut settings = Settings::new(words);
        settings.system_bundles = HOST_BUNDLES.iter().map(PathBuf::from).collect();
        let context = Context::new(Arc::new(rustls::crypto::ring::default_provider()), settings);
        // The configuration is made here: a failure names the roots, not a dial.
        context
            .config_for(&Trust::Default)
            .map_err(io::Error::other)?;
        Ok(TlsConfig {
            context: Arc::new(context),
        })
    }
}

/// A client TLS stream over any byte transport, so `wss://` rides the same
/// TCP dial every other target uses.
pub struct Tls {
    inner: rustls::StreamOwned<rustls::ClientConnection, Box<dyn Stream>>,
}

impl Tls {
    /// The verified handshake runs here, before the first frame. A
    /// refusal names the server and says what to do about an unknown
    /// issuer or a host clock that refuses the certificate.
    pub fn connect(base: Box<dyn Stream>, host: &str, cfg: &TlsConfig) -> io::Result<Self> {
        // ⛔ The name is validated before the handshake, not during it: a
        // dial to something that is not a DNS name fails here, naming the
        // value, rather than mid-handshake.
        let inner = cfg
            .context
            .tls_over_blocking(base, host, &Trust::Default)
            .map_err(io::Error::other)?;
        Ok(Tls { inner })
    }
}

impl Read for Tls {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
impl Write for Tls {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
impl Stream for Tls {
    fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        self.inner.sock.set_read_timeout(d)
    }
    fn set_write_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        self.inner.sock.set_write_timeout(d)
    }
    fn shutdown_write(&mut self) -> io::Result<()> {
        // ⛔ Best-effort goodbye. The websocket close frame above is the
        // real goodbye; a failure here carries no bytes to report.
        self.inner.conn.send_close_notify();
        self.inner.sock.shutdown_write()
    }
    fn describe(&self) -> &'static str {
        "tls"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roots_load_on_this_machine() {
        // ⛔ The guarantee this pins: the production default (verify on)
        // always has somewhere to verify against. A store with neither a
        // host bundle nor the fallback is a client that trusts nothing by
        // verifying nothing.
        TlsConfig::load().expect("no trust anchors on this machine");
    }

    #[test]
    fn garbage_server_name_is_refused_before_the_handshake() {
        let cfg = TlsConfig::load().unwrap();
        let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
        let base: Box<dyn Stream> = Box::new(crate::transport::Unix(a));
        let e = match Tls::connect(base, "not a host name!", &cfg) {
            Ok(_) => panic!("a garbage server name must not connect"),
            Err(e) => e,
        };
        assert!(
            e.to_string().contains("is not a valid server name"),
            "got {e}"
        );
    }
}
