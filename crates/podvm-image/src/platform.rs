//! The platform of a pull: `pod_oci::platform`, with podvm's variables.

pub use pod_oci::platform::*;

use crate::error::Result;

/// `PODVM_DEFAULT_PLATFORM` first, then docker's own variable.
///
/// ⚠ docker's is honoured deliberately: `AGENTS.md` says podvm answers to
/// `docker` on PATH and takes the same flags, and a caller who has set
/// `DOCKER_DEFAULT_PLATFORM` for their toolchain means it for podvm too.
pub const ENV_VARS: [&str; 2] = ["PODVM_DEFAULT_PLATFORM", "DOCKER_DEFAULT_PLATFORM"];

/// What podvm asks for, given an explicit `--platform` or none: the flag,
/// then [`ENV_VARS`], then the host, which is docker's order.
pub fn wanted(flag: Option<&str>) -> Result<Platform> {
    let values: Vec<(&str, Option<String>)> = ENV_VARS
        .iter()
        .map(|name| (*name, std::env::var(name).ok()))
        .collect();
    let defaults: Vec<(&str, Option<&str>)> = values
        .iter()
        .map(|(name, value)| (*name, value.as_deref()))
        .collect();
    Ok(Platform::wanted(flag, &defaults)?)
}
