//! [`TODO/image.md`](../../../TODO/image.md) T-0213: the insecure-registry
//! policy is `pod_oci::transport`; podvm resolves it from its flags,
//! `$PODVM_INSECURE_REGISTRIES` and its config file.

use std::path::PathBuf;

pub use pod_oci::transport::*;

use crate::error::{Error, Result};

/// `docker --insecure-registry`'s own environment spelling, plus podvm's.
const ENV_INSECURE: &str = "PODVM_INSECURE_REGISTRIES";

/// One host per line, `#` comments ([`pod_oci::transport::parse_hosts`]).
const CONFIG_ENV: &str = "PODVM_CONFIG";
const CONFIG_LEAF: &str = "podvm/registries.conf";

/// Build the policy from the flags, then the environment, then the config
/// file. ⚠ A flag never *loses* to a file: the sets are unioned, and the
/// only scalar, `tls_verify`, takes the flag when there is one.
pub fn resolve(insecure_flags: &[String], tls_verify_flag: Option<bool>) -> Result<Policy> {
    let mut hosts: Vec<String> = insecure_flags.to_vec();
    if let Ok(v) = std::env::var(ENV_INSECURE) {
        for h in v.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            normalise_host(h)
                .map_err(|e| Error::Usage(format!("${ENV_INSECURE} lists a bad host: {e}")))?;
            hosts.push(h.to_string());
        }
    }
    hosts.extend(read_config()?);
    Ok(Policy::resolve(
        hosts.iter().map(String::as_str),
        tls_verify_flag,
    )?)
}

/// `$PODVM_CONFIG`, else `$XDG_CONFIG_HOME/podvm/registries.conf`, else
/// `$HOME/.config/podvm/registries.conf`. ⚠ A missing file is not an error:
/// most machines have none and that is the default.
fn read_config() -> Result<Vec<String>> {
    let path = match std::env::var_os(CONFIG_ENV) {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => {
            let base = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
            match base {
                Some(b) => b.join(CONFIG_LEAF),
                None => return Ok(Vec::new()),
            }
        }
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(path.display().to_string(), e)),
    };
    Ok(parse_hosts(&text, &path.display().to_string())?)
}
