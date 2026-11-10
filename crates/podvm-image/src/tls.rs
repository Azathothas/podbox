//! The trust root for every request podvm makes.
//!
//! ⭐ The host bundle first, `webpki-roots` as the fallback. `Cargo.toml`
//! records why both are pinned: a machine whose egress is intercepted has a
//! private CA in its own bundle and nowhere else, and a client that trusts only
//! the compiled-in roots fails there with an error naming a certificate rather
//! than a proxy. A machine with no bundle at all is the opposite case and is
//! why the compiled-in set is not simply dropped.
//!
//! ⭐ **There IS a "skip verification" path, and it is never taken by default.**
//! [`crate::transport::Policy`] decides, from a `--tls-verify=false`, an
//! `--insecure-registry`, `$PODVM_INSECURE_REGISTRIES` or the config file, and
//! every use of it is announced on stderr.
//! [`TODO/image.md`](../../../TODO/image.md) T-0213.
//!
//! ⚠ **This paragraph used to say the opposite, and it cited a rule that does
//! not exist.** It read: "There is no skip-verification path here, not behind a
//! flag and not behind an environment variable. `docs/security/remote-ops.md`
//! and the environment note in `AGENTS.md` both say never to disable
//! verification". Checked on 2026-09-09: `remote-ops.md` says nothing about TLS
//! at all, and `AGENTS.md`'s note is an instruction to the **agent** about
//! this development container's intercepting proxy, not a rule about what
//! podvm may offer. ⛔ "A switch that exists gets set" is a real argument and
//! is answered rather than dismissed: the switch is off by default, it is
//! per-registry rather than global, and it prints a line naming the registry
//! every time it is used, so a set switch is visible in the transcript of an
//! automated caller rather than inferred from a missing failure.

use std::sync::Arc;

use rustls::ClientConfig;
use rustls_pki_types::CertificateDer;

/// Which roots a config was built from, for the diagnostic. ⛔ Reported, never
/// assumed: "TLS failed" is unactionable when the reader cannot tell which
/// trust store was consulted.
pub struct Roots {
    pub source: String,
    pub count: usize,
}

/// This machine's CA bundle, and ⭐ **whether the machine ANNOUNCED it**
/// (`pod_oci::trust::HostBundle`).
pub use pod_oci::trust::HostBundle;

/// The host's CA bundle as the PEM bytes on disk, and where they came from:
/// `$SSL_CERT_FILE`, `$CURL_CA_BUNDLE`, `$REQUESTS_CA_BUNDLE`, then the
/// distributions' paths (`pod_oci::trust`).
///
/// ⭐ [`TODO/complete.md`](../../../TODO/complete.md) T-0407 installs this into
/// an image that ships none, so that an `https://` package source can verify.
/// ⛔ It is THIS list rather than a second one: podvm's own transport already
/// had to answer "where does this machine keep its roots", and two lists drift
/// so that the one a reader trusts is the wrong one.
///
/// ⚠ The bytes are returned only where they parse as at least one certificate,
/// so an empty or truncated file reads as "this machine has none" rather than
/// being copied into an image where it would fail later and further away.
pub fn host_bundle_bytes() -> Option<HostBundle> {
    pod_oci::trust::host_bundle(&|n| std::env::var_os(n))
}

/// A verifier that accepts anything, for a registry the caller named insecure.
///
/// ⛔ Reachable only through [`client_config_unverified`], which
/// [`crate::registry::Client`] calls only for an endpoint
/// [`crate::transport::Policy`] permits. It is not a global mode and there is
/// no environment variable that reaches it without naming a host.
#[derive(Debug)]
struct AcceptAny(Arc<rustls::crypto::CryptoProvider>);

impl rustls::client::danger::ServerCertVerifier for AcceptAny {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls_pki_types::ServerName<'_>,
        _ocsp: &[u8],
        _now: rustls_pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// The configuration for an endpoint whose certificate is not checked.
///
/// ⚠ The chain is still built and the handshake still happens; what is skipped
/// is deciding whether the chain reaches a trusted root. A caller who sees
/// `Roots.source` here reads exactly that, so a log cannot be mistaken for a
/// verified connection.
pub fn client_config_unverified() -> (Arc<ClientConfig>, Roots) {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .expect("ring supports the default protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAny(provider)))
        .with_no_client_auth();
    (
        Arc::new(config),
        Roots {
            source: "note: NONE: this registry was named insecure, so its \
                     certificate was not verified against any root"
                .to_string(),
            count: 0,
        },
    )
}

/// Build the client configuration and say where its trust came from.
///
/// ⭐ The host bundle is used alone, as `pod-tls` uses a CA file
/// ([`pod_tls::CaFile`]); the compiled-in roots are the fallback for a machine
/// with no bundle, or with a bundle that has no usable root
/// (`pod_oci::trust::roots`). `pod-tls` makes the configuration: its
/// verifier, the protocol versions and the store of sessions.
pub fn client_config() -> (Arc<ClientConfig>, Roots) {
    let roots = pod_oci::trust::roots(host_bundle_bytes().as_ref());
    // ⛔ The provider is named rather than taken from the process default.
    // `ClientConfig::builder()` resolves a default that depends on which
    // provider features anything else in the graph enabled, so naming `ring`
    // here is what makes the build's crypto the crypto TODO/deps.md T-0905
    // measured.
    let context = pod_tls::Context::new(
        Arc::new(rustls::crypto::ring::default_provider()),
        pod_tls::Settings::new(pod_tls::Words::new("podvm")),
    );
    let config = context
        .client_config(&roots)
        .expect("a store with at least one root builds its verifier");
    (
        config,
        Roots {
            source: roots.source,
            count: roots.count,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_config_is_built_and_names_where_its_roots_came_from() {
        // The production default of the seam, not an injected double:
        // docs/conventions/code.md requires the shipping branch be tested.
        let (_cfg, roots) = client_config();
        assert!(roots.count > 0, "no roots from {}", roots.source);
        assert!(!roots.source.is_empty());
    }
}
