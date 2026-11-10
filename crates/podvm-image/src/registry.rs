//! `TODO/image.md` T-0201: the registry client is `pod_oci::registry`;
//! podvm gives it its transport ([`crate::net`]), its trust
//! ([`crate::tls`]), its logins ([`crate::credentials`]), its policy and the
//! file-size ceiling of the process.

use std::sync::Arc;

pub use pod_oci::registry::*;

use crate::error::{Error, Result};
use crate::transport::Policy;

/// The client of one pull, or of one fetch worker, over the proxy of the
/// process environment.
///
/// ⛔ Each request verifies with podvm's own configuration
/// ([`crate::tls::client_config`]); the one that does not is reached only
/// for a registry that the caller named insecure ([`Policy`]). A move to
/// plain HTTP is a line on stderr, `podvm: ...`.
pub fn client(policy: Policy) -> Result<Client> {
    let (config, _roots) = crate::tls::client_config();
    let (unverified, _) = crate::tls::client_config_unverified();
    let refused = |e: String| Error::Http {
        what: "the transport".into(),
        detail: e,
    };
    let env = crate::net::environment(&|n| std::env::var_os(n));
    let dialer = crate::net::dialer(env)
        .map_err(|e| refused(format!("the runtime of the HTTP client: {e}")))?;
    let context = Arc::new(pod_tls::Context::new(
        Arc::new(rustls::crypto::ring::default_provider()),
        pod_tls::Settings::new(pod_tls::Words::new("podvm")),
    ));
    let http =
        pod_proxy::blocking::Client::new(context, &dialer).with_user_agent(crate::net::USER_AGENT);
    let verified = pod_tls::Trust::caller(config).map_err(|e| refused(e.to_string()))?;
    let unverified = pod_tls::Trust::caller(unverified).map_err(|e| refused(e.to_string()))?;
    Ok(Client::new(http, verified)
        .with_policy(policy)
        .with_insecure(unverified)
        .with_logins(Arc::new(crate::credentials::files()))
        .with_events(Arc::new(|e: pod_core::Event| {
            use std::io::Write;
            let _ = writeln!(std::io::stderr(), "podvm: {e}");
        }))
        .with_file_ceiling(crate::space::file_ceiling()))
}
