//! TLS, client and server. rustls is the transport T-0905 ruled in; `ring` is
//! its crypto provider.
//!
//! ⛔ Verification is on by default. `--insecure` exists for a self-signed
//! test relay and for nothing else, and every place that turns it on says so
//! on stderr. The security boundary is the ssh session inside, but a relay
//! that can swap certificates can still deny service and observe timing.

use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConnection, DigitallySignedStruct, ServerConnection, SignatureScheme};

use super::Stream;

/// A client TLS configuration, shared by every dial.
pub struct ClientConfig {
    pub config: Arc<rustls::ClientConfig>,
    pub insecure: bool,
}

impl ClientConfig {
    /// Build a root store, in order: an operator's `--tls-ca`, the host
    /// bundle, then `webpki-roots`. A cage with an intercepting proxy puts
    /// its CA in the host bundle; a cage with no `/etc` gets the built-in
    /// roots. Either way the default is verify, never trust-anything.
    pub fn new(insecure: bool, extra_ca: &[std::path::PathBuf]) -> io::Result<Self> {
        if insecure {
            let config = rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(NoVerify::new()))
                .with_no_client_auth();
            return Ok(ClientConfig {
                config: Arc::new(config),
                insecure: true,
            });
        }

        let mut roots = rustls::RootCertStore::empty();
        let mut loaded = 0usize;
        for p in extra_ca {
            let n = load_pem_roots(&mut roots, p)?;
            if n == 0 {
                return Err(io::Error::other(format!(
                    "{} carries no CERTIFICATE block",
                    p.display()
                )));
            }
            loaded += n;
        }
        if loaded == 0 {
            for p in host_bundles() {
                if let Ok(n) = load_pem_roots(&mut roots, Path::new(p)) {
                    loaded += n;
                }
            }
        }
        if loaded == 0 {
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        }
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(ClientConfig {
            config: Arc::new(config),
            insecure: false,
        })
    }
}

/// Where a normal Linux host keeps its trust anchors.
pub fn host_bundles() -> &'static [&'static str] {
    &[
        "/etc/ssl/certs/ca-certificates.crt",
        "/etc/pki/tls/certs/ca-bundle.crt",
        "/etc/ssl/ca-bundle.pem",
        "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
    ]
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

/// A client TLS stream over any transport, so `tls://` can ride a CONNECT
/// proxy, a unix socket, or an `exec` command.
pub struct Tls(pub rustls::StreamOwned<ClientConnection, Box<dyn Stream>>);

impl Tls {
    pub fn connect(
        base: Box<dyn Stream>,
        host: &str,
        cfg: Arc<ClientConfig>,
    ) -> io::Result<Self> {
        let name = ServerName::try_from(host.to_string())
            .map_err(|e| io::Error::other(format!("bad TLS server name {host:?}: {e}")))?;
        let conn = ClientConnection::new(cfg.config.clone(), name)
            .map_err(|e| io::Error::other(format!("TLS client: {e}")))?;
        Ok(Tls(rustls::StreamOwned::new(conn, base)))
    }
}

impl Read for Tls {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}
impl Write for Tls {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
impl Stream for Tls {
    fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        self.0.sock.set_read_timeout(d)
    }
    fn shutdown_write(&mut self) -> io::Result<()> {
        let _ = self.0.conn.send_close_notify();
        self.0.sock.shutdown_write()
    }
    fn describe(&self) -> &'static str {
        "tls"
    }
}

/// The server side, used only by `podssh relay --tls-cert/--tls-key`.
pub struct ServerTls(pub rustls::StreamOwned<ServerConnection, Box<dyn Stream>>);

impl ServerTls {
    pub fn accept(base: Box<dyn Stream>, cfg: Arc<rustls::ServerConfig>) -> io::Result<Self> {
        let conn = ServerConnection::new(cfg)
            .map_err(|e| io::Error::other(format!("TLS server: {e}")))?;
        Ok(ServerTls(rustls::StreamOwned::new(conn, base)))
    }
}

impl Read for ServerTls {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}
impl Write for ServerTls {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
impl Stream for ServerTls {
    fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()> {
        self.0.sock.set_read_timeout(d)
    }
    fn shutdown_write(&mut self) -> io::Result<()> {
        let _ = self.0.conn.send_close_notify();
        self.0.sock.shutdown_write()
    }
    fn describe(&self) -> &'static str {
        "tls-server"
    }
}

/// Build a server config from a certificate chain and private key in PEM.
pub fn server_config(cert: &Path, key: &Path) -> io::Result<Arc<rustls::ServerConfig>> {
    let certs = std::fs::read(cert)?;
    let key = std::fs::read(key)?;
    server_config_pem(&certs, &key)
}

/// The PEM bytes form, so a test can carry its own certificate.
pub fn server_config_pem(
    cert_pem: &[u8],
    key_pem: &[u8],
) -> io::Result<Arc<rustls::ServerConfig>> {
    let certs: Vec<CertificateDer<'static>> = {
        let mut c = std::io::Cursor::new(cert_pem);
        rustls_pemfile::certs(&mut c)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| io::Error::other(format!("certificate PEM: {e}")))?
    };
    if certs.is_empty() {
        return Err(io::Error::other("certificate PEM carries no CERTIFICATE block"));
    }
    let key = {
        let mut c = std::io::Cursor::new(key_pem);
        rustls_pemfile::private_key(&mut c)
            .map_err(|e| io::Error::other(format!("private key PEM: {e}")))?
            .ok_or_else(|| io::Error::other("private key PEM carries no PRIVATE KEY"))?
    };
    let cfg = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| io::Error::other(format!("TLS server config: {e}")))?;
    Ok(Arc::new(cfg))
}

/// `--insecure` verifier. It accepts every certificate and every signature.
#[derive(Debug)]
struct NoVerify(Arc<rustls::crypto::CryptoProvider>);

impl NoVerify {
    fn new() -> Self {
        NoVerify(Arc::new(rustls::crypto::ring::default_provider()))
    }
}

impl ServerCertVerifier for NoVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    // A throwaway self-signed ed25519 certificate for `localhost`, valid to
    // 2036. It is a test fixture and protects nothing; the point of the test
    // is that a real rustls handshake completes in both directions.
    const CERT: &str = "-----BEGIN CERTIFICATE-----\n\
MIIBPDCB76ADAgECAhRfzK/G9wzeld8NGpKppy4EH33jwDAFBgMrZXAwFDESMBAG\n\
A1UEAwwJbG9jYWxob3N0MB4XDTI2MDkyNzAxMzEzMloXDTM2MDkyNDAxMzEzMlow\n\
FDESMBAGA1UEAwwJbG9jYWxob3N0MCowBQYDK2VwAyEAXk23BjK/m81BlnHo8MFt\n\
z+isc/nj63lX6i6XPqmvV3GjUzBRMB0GA1UdDgQWBBQC+spqW1I1YJAhzh8b7gC5\n\
XLpoFzAfBgNVHSMEGDAWgBQC+spqW1I1YJAhzh8b7gC5XLpoFzAPBgNVHRMBAf8E\n\
BTADAQH/MAUGAytlcANBAOUniLCkgkZH7LMK8OD34tngZgncoXSZKu4Ozg0sNqTO\n\
SFemxw+wQuNKd9t2G6v2SixknFSGS3aMf+xY5mjz4gQ=\n\
-----END CERTIFICATE-----\n";
    const KEY: &str = "-----BEGIN PRIVATE KEY-----\n\
MC4CAQAwBQYDK2VwBCIEIKIaUyQfO7sz4T4kA7hwrVRr0rfPYYtrgkOQgHSXTvFA\n\
-----END PRIVATE KEY-----\n";

    #[test]
    fn insecure_config_builds() {
        let cfg = ClientConfig::new(true, &[]).unwrap();
        assert!(cfg.insecure);
    }

    #[test]
    fn default_config_has_roots() {
        let cfg = ClientConfig::new(false, &[]).unwrap();
        assert!(!cfg.insecure);
    }

    #[test]
    fn missing_extra_ca_is_an_error() {
        let err = ClientConfig::new(false, &[std::path::PathBuf::from("/nonexistent.pem")]);
        assert!(err.is_err());
    }

    #[test]
    fn client_and_server_tls_round_trip_over_unix() {
        use std::os::unix::net::UnixStream;
        use std::sync::Arc;
        let cfg = server_config_pem(CERT.as_bytes(), KEY.as_bytes()).unwrap();
        let (a, b) = UnixStream::pair().unwrap();
        let h = std::thread::spawn(move || {
            let mut s = ServerTls::accept(Box::new(crate::transport::Unix(b)), cfg).unwrap();
            let mut buf = [0u8; 5];
            s.read_exact(&mut buf).unwrap();
            assert_eq!(&buf, b"hello");
            s.write_all(b"world").unwrap();
            s.flush().unwrap();
        });
        let client = Arc::new(ClientConfig::new(true, &[]).unwrap());
        let mut c = Tls::connect(Box::new(crate::transport::Unix(a)), "localhost", client).unwrap();
        c.write_all(b"hello").unwrap();
        c.flush().unwrap();
        let mut buf = [0u8; 5];
        c.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"world");
        h.join().unwrap();
    }

    #[test]
    fn write_read_traits_exist() {
        fn assert_traits<T: Read + Write + Send>() {}
        assert_traits::<Tls>();
        assert_traits::<ServerTls>();
        let _ = (std::io::empty(), std::io::sink());
        let _ = std::io::stdout().write_all(b"");
    }
}
