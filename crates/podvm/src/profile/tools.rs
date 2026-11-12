//! The pinned programs of the runtime profiles (TODO/cli.md T-1661).
//!
//! A runtime with no OCI image of its party comes as an archive of its
//! release. podvm pins the SHA-256 of the archive and of the program in it
//! ([`crate::assets::Proof::Pinned`]), fetches the archive once, and keeps
//! the program in the store under that digest, beside the files of the
//! profile (a config). The run mounts the directory read-only at the
//! tool's guest directory.
//!
//! ⚠ `pod-services` does not hold these pins yet: COORD-053 asks for
//! workerd (its host waits in `WAITING` of `crates/podvm/src/hosts_tests.rs`)
//! and COORD-054 for wasmtime. The pins then move there.

use std::io::Read;
use std::path::{Path, PathBuf};

use podvm_image::net::{Limits, Net};

/// How the archive is packed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pack {
    TarGz,
    TarXz,
}

/// One pinned program.
#[derive(Debug)]
pub struct Tool {
    pub name: &'static str,
    pub version: &'static str,
    pub url: &'static str,
    pub pack: Pack,
    pub archive_sha256: &'static str,
    /// A larger answer is not the pin.
    pub max_archive: u64,
    pub member: &'static str,
    pub bin_sha256: &'static str,
    /// Where the run sees the program and the files.
    pub guest_dir: &'static str,
    /// Each file that podvm writes beside the program: (name, text).
    pub files: &'static [(&'static str, &'static str)],
    /// The release on GitHub, fetched through `gh` first (T-1667).
    pub release: Option<super::gh::Release>,
}

/// workerd: the npm package `@cloudflare/workerd-linux-64` of 2026-10-29.
/// The registry's `dist.integrity` of the archive is
/// `sha512-HkZx+YRRhsZ9HTZYqR7/Ivyhb52XtSngL5Dn208PyS9x05thTlIVxQiF3f2gRMSMoWqJ6l1EO1qQTnThQfeHKg==`,
/// and it agrees with the archive of the pin.
pub const WORKERD: Tool = Tool {
    name: "workerd",
    version: "1.20261029.1",
    url: "https://registry.npmjs.org/@cloudflare/workerd-linux-64/-/workerd-linux-64-1.20261029.1.tgz",
    pack: Pack::TarGz,
    archive_sha256: "771936b71cbc88d01e5cbb2779ebabc91a5b79070b8f41d3d203973afb779c85",
    max_archive: 128 << 20,
    member: "package/bin/workerd",
    bin_sha256: "685acc339ac78bf7b605928c1845fef8d8a89e919b079556b571a3404b95db3e",
    guest_dir: "/opt/podvm-workerd",
    files: &[("config.capnp", super::workerd::CONFIG)],
    release: None,
};

/// wasmtime: the release v50.0.1 of 2026-10-29. Its party publishes no
/// checksum (F25 of issue 176), so the pin is podvm's own measurement.
pub const WASMTIME: Tool = Tool {
    name: "wasmtime",
    version: "50.0.1",
    url: "https://github.com/bytecodealliance/wasmtime/releases/download/v50.0.1/wasmtime-v50.0.1-x86_64-linux.tar.xz",
    pack: Pack::TarXz,
    archive_sha256: "685466455dd42a14774a86c36c8f2e45cbb9bd6d92a850139315df60a75498a5",
    max_archive: 64 << 20,
    member: "wasmtime-v50.0.1-x86_64-linux/wasmtime",
    bin_sha256: "983e57573a320cf44c93ccaa571a25640cf38b15e8e14160aa7bb919336caae8",
    guest_dir: "/opt/podvm-wasmtime",
    files: &[],
    release: Some(super::gh::Release {
        repo: "bytecodealliance/wasmtime",
        tag: "v50.0.1",
        asset: "wasmtime-v50.0.1-x86_64-linux.tar.xz",
    }),
};

/// The pinned program of `runtime`, where podvm brings one.
pub fn of_runtime(runtime: &str) -> Option<&'static Tool> {
    match runtime {
        "workerd" => Some(&WORKERD),
        "wasmtime" => Some(&WASMTIME),
        _ => None,
    }
}

impl Tool {
    /// The program, as the run sees it.
    pub fn guest_program(&self) -> String {
        format!("{}/{}", self.guest_dir, self.name)
    }
}

const LIMITS: Limits = Limits {
    head: std::time::Duration::from_secs(60),
    idle: std::time::Duration::from_secs(120),
};

/// The directory of the program and its files in `store_root`, fetched
/// where it is not there yet.
pub fn ensure(store_root: &Path, t: &Tool) -> Result<PathBuf, String> {
    let dir = store_root
        .join("tools")
        .join(format!("{}-{}", t.name, &t.bin_sha256[..16]));
    let bin = dir.join(t.name);
    let fresh = t
        .files
        .iter()
        .all(|(n, text)| std::fs::read_to_string(dir.join(n)).is_ok_and(|c| c == *text));
    if bin.is_file() && fresh {
        return Ok(dir);
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if !bin.is_file() {
        crate::assets::space_floor(&dir, t.name, 512 << 20)?;
        let bytes = fetch(t).map_err(|e| format!("{} {}: {e}", t.name, t.version))?;
        let tmp = dir.join(format!("{}.partial", t.name));
        std::fs::write(&tmp, &bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .and_then(|()| std::fs::rename(&tmp, &bin))
            .map_err(|e| format!("{}: {e}", bin.display()))?;
    }
    for (n, text) in t.files {
        let p = dir.join(n);
        std::fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    Ok(dir)
}

/// The program out of the pinned archive, each SHA-256 checked.
fn fetch(t: &Tool) -> Result<Vec<u8>, String> {
    use crate::machine::guest::kernel::{sha256_hex, tar_members};
    let url = t.url;
    let public = || -> Result<Vec<u8>, String> {
        let (config, _roots) = podvm_image::tls::client_config();
        let mut net = Net::from_environment(config)?;
        let body = net
            .send("GET", url, &[], true, LIMITS)
            .map_err(|e| format!("{url}: {e}"))?;
        let mut archive = Vec::new();
        body.take(t.max_archive + 1)
            .read_to_end(&mut archive)
            .map_err(|e| format!("{url}: {e}"))?;
        Ok(archive)
    };
    let archive = match &t.release {
        Some(r) => super::gh::fetch(r, "gh", t.max_archive, public)?.0,
        None => public()?,
    };
    let got = sha256_hex(&archive);
    if got != t.archive_sha256 {
        return Err(format!(
            "{url}: the archive has SHA-256 {got}, not the pin {}",
            t.archive_sha256
        ));
    }
    let members = match t.pack {
        Pack::TarGz => tar_members(flate2::read::GzDecoder::new(&archive[..]), &[t.member]),
        Pack::TarXz => tar_members(
            xz4rust::XzReader::new(std::io::Cursor::new(archive)),
            &[t.member],
        ),
    };
    let bin = members
        .map_err(|e| format!("{url}: {e}"))?
        .pop()
        .flatten()
        .ok_or(format!("{url}: no {}", t.member))?;
    let got = sha256_hex(&bin);
    if got != t.bin_sha256 {
        return Err(format!(
            "{url}: {} has SHA-256 {got}, not the pin {}",
            t.member, t.bin_sha256
        ));
    }
    Ok(bin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_pin_names_its_version_and_its_guest_directory() {
        for t in [&WORKERD, &WASMTIME] {
            assert!(t.url.contains(t.version), "{t:?}");
            assert_eq!(t.archive_sha256.len(), 64);
            assert_eq!(t.bin_sha256.len(), 64);
            assert!(t.guest_dir.starts_with("/opt/podvm-"));
            assert!(t.member.ends_with(t.name));
        }
        assert_eq!(of_runtime("node").map(|t| t.name), None);
        assert_eq!(WASMTIME.guest_program(), "/opt/podvm-wasmtime/wasmtime");
    }
}
