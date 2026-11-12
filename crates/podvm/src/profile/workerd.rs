//! The workerd of the profile `workerd-test`, pinned (TODO/cli.md T-1661).
//!
//! workerd has no OCI image of its party. Its npm package
//! `@cloudflare/workerd-linux-64` holds the binary, and the registry
//! publishes the package's sha512 (`dist.integrity`). podvm pins the
//! SHA-256 of the archive and of the binary in it, fetches the archive
//! once, and keeps the binary in the store under that digest, beside a
//! config with no outbound service. The run mounts the directory read-only
//! at [`GUEST_DIR`].
//!
//! ⚠ The host is in `WAITING` of `crates/podvm/src/hosts_tests.rs` until
//! `pod-services` lists it (COORD-053); the pins then move there.

use std::io::Read;
use std::path::{Path, PathBuf};

use podvm_image::net::{Limits, Net};

/// The version of the package: the release of 2026-10-29.
pub const VERSION: &str = "1.20261029.1";
const URL: &str =
    "https://registry.npmjs.org/@cloudflare/workerd-linux-64/-/workerd-linux-64-1.20261029.1.tgz";
/// The SHA-256 of the archive (43,615,013 bytes); its sha512 is the
/// registry's `dist.integrity`
/// `sha512-HkZx+YRRhsZ9HTZYqR7/Ivyhb52XtSngL5Dn208PyS9x05thTlIVxQiF3f2gRMSMoWqJ6l1EO1qQTnThQfeHKg==`.
const ARCHIVE_SHA256: &str = "771936b71cbc88d01e5cbb2779ebabc91a5b79070b8f41d3d203973afb779c85";
const MEMBER: &str = "package/bin/workerd";
/// The SHA-256 of the binary (148,775,704 bytes).
const BIN_SHA256: &str = "685acc339ac78bf7b605928c1845fef8d8a89e919b079556b571a3404b95db3e";
/// A larger answer is not the pin.
const MAX_ARCHIVE: u64 = 128 << 20;
const LIMITS: Limits = Limits {
    head: std::time::Duration::from_secs(60),
    idle: std::time::Duration::from_secs(120),
};

/// Where the run sees the binary and the config.
pub const GUEST_DIR: &str = "/opt/podvm-workerd";

/// The config: the worker `/workspace/worker.js`, and no outbound service,
/// so each `fetch` of the worker fails. The date leaves `process`
/// undefined. An embed is relative to the config, so the worker is
/// `../../workspace/worker.js` from [`GUEST_DIR`].
pub const CONFIG: &str = r#"using Workerd = import "/workerd/workerd.capnp";
const config :Workerd.Config = (
  services = [
    (name = "main", worker = .worker),
    (name = "none", network = (allow = [])),
  ],
);
const worker :Workerd.Worker = (
  modules = [ (name = "worker.js", esModule = embed "../../workspace/worker.js") ],
  compatibilityDate = "2025-09-15",
  globalOutbound = "none",
);
"#;

/// The command prefix: `workerd test` of the config.
pub fn prefix() -> Vec<String> {
    vec![
        format!("{GUEST_DIR}/workerd"),
        "test".into(),
        format!("{GUEST_DIR}/config.capnp"),
    ]
}

/// The directory of the binary and the config in `store_root`, fetched
/// where it is not there yet.
pub fn ensure(store_root: &Path) -> Result<PathBuf, String> {
    let dir = store_root
        .join("tools")
        .join(format!("workerd-{}", &BIN_SHA256[..16]));
    let bin = dir.join("workerd");
    let config = dir.join("config.capnp");
    if bin.is_file() && std::fs::read_to_string(&config).is_ok_and(|c| c == CONFIG) {
        return Ok(dir);
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if !bin.is_file() {
        crate::assets::space_floor(&dir, "workerd", 512 << 20)?;
        let bytes = fetch().map_err(|e| format!("workerd {VERSION}: {e}"))?;
        let tmp = dir.join("workerd.partial");
        std::fs::write(&tmp, &bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .and_then(|()| std::fs::rename(&tmp, &bin))
            .map_err(|e| format!("{}: {e}", bin.display()))?;
    }
    std::fs::write(&config, CONFIG).map_err(|e| format!("{}: {e}", config.display()))?;
    Ok(dir)
}

/// The binary out of the pinned archive, each SHA-256 checked.
fn fetch() -> Result<Vec<u8>, String> {
    use crate::machine::guest::kernel::{sha256_hex, tar_members};
    let (config, _roots) = podvm_image::tls::client_config();
    let mut net = Net::from_environment(config)?;
    let body = net
        .send("GET", URL, &[], true, LIMITS)
        .map_err(|e| format!("{URL}: {e}"))?;
    let mut archive = Vec::new();
    body.take(MAX_ARCHIVE + 1)
        .read_to_end(&mut archive)
        .map_err(|e| format!("{URL}: {e}"))?;
    let got = sha256_hex(&archive);
    if got != ARCHIVE_SHA256 {
        return Err(format!(
            "{URL}: the archive has SHA-256 {got}, not the pin {ARCHIVE_SHA256}"
        ));
    }
    let gz = flate2::read::GzDecoder::new(&archive[..]);
    let bin = tar_members(gz, &[MEMBER])
        .map_err(|e| format!("{URL}: {e}"))?
        .pop()
        .flatten()
        .ok_or(format!("{URL}: no {MEMBER}"))?;
    let got = sha256_hex(&bin);
    if got != BIN_SHA256 {
        return Err(format!(
            "{URL}: {MEMBER} has SHA-256 {got}, not the pin {BIN_SHA256}"
        ));
    }
    Ok(bin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_has_no_outbound_service() {
        assert!(CONFIG.contains("globalOutbound = \"none\""));
        assert!(CONFIG.contains("network = (allow = [])"));
        assert!(!CONFIG.contains("public"));
        assert_eq!(prefix()[1], "test");
        assert!(URL.contains(VERSION));
    }
}
