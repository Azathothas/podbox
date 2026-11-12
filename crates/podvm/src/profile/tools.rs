//! The pinned programs of the runtime profiles (TODO/cli.md T-1661).
//!
//! A runtime with no OCI image of its party comes as an archive of its
//! release. podvm pins the SHA-256 of the archive and of the program in it
//! ([`crate::assets::Proof::Pinned`]), fetches the archive once, and keeps
//! the program in the store under that digest, beside the files of the
//! profile (a config). The run mounts the directory read-only at the
//! tool's guest directory.
//!
//! The URL, the version and the sums are facts of `pod-services`
//! (`cloudflare-workerd-npm`, `wasmtime-release`; podcrates `v0.28.0`,
//! COORD-053 and COORD-054): podvm keeps no second copy of a host of
//! another party.

use std::io::Read;
use std::path::{Path, PathBuf};

use podvm_image::net::{Limits, Net};

/// How the archive is packed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pack {
    TarGz,
    TarXz,
}

/// One pinned program: podvm's own facts of it, and the service of
/// `pod-services` that holds its pins.
#[derive(Debug)]
pub struct Tool {
    pub name: &'static str,
    /// The id of the service in `pod-services`.
    pub service: &'static str,
    pub pack: Pack,
    /// A larger answer is not the pin.
    pub max_archive: u64,
    /// Where the run sees the program and the files.
    pub guest_dir: &'static str,
    /// Each file that podvm writes beside the program: (name, text).
    pub files: &'static [(&'static str, &'static str)],
    /// The repository of a release on GitHub, fetched through `gh` first
    /// (T-1667).
    pub gh_repo: Option<&'static str>,
}

/// workerd: the npm package `@cloudflare/workerd-linux-64`.
pub const WORKERD: Tool = Tool {
    name: "workerd",
    service: "cloudflare-workerd-npm",
    pack: Pack::TarGz,
    max_archive: 128 << 20,
    guest_dir: "/opt/podvm-workerd",
    files: &[("config.capnp", super::workerd::CONFIG)],
    gh_repo: None,
};

/// wasmtime: the release archive. Its party publishes no checksum (F25 of
/// issue 176), so the pin is podvm's own measurement.
pub const WASMTIME: Tool = Tool {
    name: "wasmtime",
    service: "wasmtime-release",
    pack: Pack::TarXz,
    max_archive: 64 << 20,
    guest_dir: "/opt/podvm-wasmtime",
    files: &[],
    gh_repo: Some("bytecodealliance/wasmtime"),
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

    fn svc(&self) -> &'static pod_services::Service {
        pod_services::get(self.service)
            .unwrap_or_else(|| panic!("pod-services holds {} (TODO/cli.md T-1661)", self.service))
    }

    fn fact(&self, name: &str) -> &'static str {
        self.svc()
            .text(name)
            .unwrap_or_else(|| panic!("{} has the fact {name}", self.service))
    }

    /// The URL of the archive.
    pub fn url(&self) -> String {
        self.svc()
            .endpoint("archive")
            .unwrap_or_else(|| panic!("{} has the endpoint archive", self.service))
            .url()
    }

    pub fn version(&self) -> &'static str {
        self.fact("version")
    }

    pub fn archive_sha256(&self) -> &'static str {
        self.fact("archive-sha256")
    }

    pub fn member(&self) -> &'static str {
        self.fact("bin-member")
    }

    pub fn bin_sha256(&self) -> &'static str {
        self.fact("bin-sha256")
    }

    /// The asset of the release on GitHub: the tag `v` and the version,
    /// and the last part of the URL.
    pub fn release(&self) -> Option<super::gh::Release> {
        let url = self.url();
        self.gh_repo.map(|repo| super::gh::Release {
            repo,
            tag: format!("v{}", self.version()),
            asset: url.rsplit('/').next().unwrap_or_default().to_string(),
        })
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
        .join(format!("{}-{}", t.name, &t.bin_sha256()[..16]));
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
        let bytes = fetch(t).map_err(|e| format!("{} {}: {e}", t.name, t.version()))?;
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
    let url = t.url();
    let url = url.as_str();
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
    let archive = match &t.release() {
        Some(r) => super::gh::fetch(r, "gh", t.max_archive, public)?.0,
        None => public()?,
    };
    let got = sha256_hex(&archive);
    if got != t.archive_sha256() {
        return Err(format!(
            "{url}: the archive has SHA-256 {got}, not the pin {}",
            t.archive_sha256()
        ));
    }
    let members = match t.pack {
        Pack::TarGz => tar_members(flate2::read::GzDecoder::new(&archive[..]), &[t.member()]),
        Pack::TarXz => tar_members(
            xz4rust::XzReader::new(std::io::Cursor::new(archive)),
            &[t.member()],
        ),
    };
    let bin = members
        .map_err(|e| format!("{url}: {e}"))?
        .pop()
        .flatten()
        .ok_or(format!("{url}: no {}", t.member()))?;
    let got = sha256_hex(&bin);
    if got != t.bin_sha256() {
        return Err(format!(
            "{url}: {} has SHA-256 {got}, not the pin {}",
            t.member(),
            t.bin_sha256()
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
            assert!(t.url().contains(t.version()), "{t:?}");
            assert_eq!(t.archive_sha256().len(), 64);
            assert_eq!(t.bin_sha256().len(), 64);
            assert!(t.guest_dir.starts_with("/opt/podvm-"));
            assert!(t.member().ends_with(t.name));
        }
        // The asset of the release names the version of the service.
        let r = WASMTIME.release().unwrap();
        assert_eq!(r.tag, "v50.0.1");
        assert_eq!(r.asset, "wasmtime-v50.0.1-x86_64-linux.tar.xz");
        assert!(WORKERD.release().is_none());
        assert_eq!(of_runtime("node").map(|t| t.name), None);
        assert_eq!(WASMTIME.guest_program(), "/opt/podvm-wasmtime/wasmtime");
    }
}
