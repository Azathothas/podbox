//! TLS for the relay legs, client side only.
//!
//! rustls is the transport TODO/deps.md T-0905 ruled in; `ring` is its
//! crypto provider and already builds in this workspace. Verification is
//! always on: there is no `--insecure` flag, because a relay that can swap
//! certificates can deny service and observe timing, and a test relay that
//! needs no verification speaks plain `ws` on loopback instead.
//!
//! The root store loads the host bundles first, because a cage with an
//! intercepting proxy puts its CA in the host bundle, and falls back to
//! `webpki-roots` for a machine with no bundle at all. Either way the
//! default is verify, never trust-anything.

use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::ServerName;

use crate::transport::Stream;

/// Where a normal Linux host keeps its trust anchors, in load order.
pub const HOST_BUNDLES: &[&str] = &[
    "/etc/ssl/certs/ca-certificates.crt",
    "/etc/pki/tls/certs/ca-bundle.crt",
    "/etc/ssl/ca-bundle.pem",
    "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
];

/// A client TLS configuration, shared by every dial on one process.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    config: Arc<rustls::ClientConfig>,
}

impl TlsConfig {
    /// Build the root store: host bundles first, `webpki-roots` when none
    /// loaded. An unreadable bundle is skipped and an empty one adds
    /// nothing; there is no refusal when no host bundle exists, because
    /// the fallback is the design for a machine with none at all, and a
    /// client with the fallback still verifies everything.
    pub fn load() -> io::Result<Self> {
        let mut roots = rustls::RootCertStore::empty();
        let mut loaded = 0usize;
        for p in HOST_BUNDLES {
            match load_pem_roots(&mut roots, Path::new(p)) {
                Ok(n) => loaded += n,
                Err(_) => continue,
            }
        }
        if loaded == 0 {
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        }
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(TlsConfig {
            config: Arc::new(config),
        })
    }
}

fn load_pem_roots(roots: &mut rustls::RootCertStore, path: &Path) -> io::Result<usize> {
    let data = std::fs::read(path)?;
    let mut cursor = std::io::Cursor::new(data);
    let mut n = 0;
    for cert in rustls_pemfile::certs(&mut cursor) {
        roots
            .add(cert?)
            .map_err(|e| io::Error::other(format!("{}: {e}", path.display())))?;
        n += 1;
    }
    Ok(n)
}

/// A client TLS stream over any byte transport, so `wss://` rides the same
/// TCP dial every other target uses.
pub struct Tls {
    inner: rustls::StreamOwned<rustls::ClientConnection, Box<dyn Stream>>,
}

impl Tls {
    pub fn connect(base: Box<dyn Stream>, host: &str, cfg: &TlsConfig) -> io::Result<Self> {
        // ⛔ The name is validated before the handshake, not during it: a
        // dial to something that is not a DNS name fails here, naming the
        // value, rather than mid-handshake.
        let name = ServerName::try_from(host.to_string())
            .map_err(|e| io::Error::other(format!("bad TLS server name {host:?}: {e}")))?;
        let conn = rustls::ClientConnection::new(cfg.config.clone(), name)
            .map_err(|e| io::Error::other(format!("TLS client: {e}")))?;
        Ok(Tls {
            inner: rustls::StreamOwned::new(conn, base),
        })
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
        assert!(e.to_string().contains("bad TLS server name"), "got {e}");
    }
}
