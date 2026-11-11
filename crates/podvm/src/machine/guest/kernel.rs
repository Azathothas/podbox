//! The guest kernel of the machine tier, pinned, TODO/machine.md T-1656.
//!
//! podvm boots each guest of `run --podvm-tier=machine` with one kernel:
//! `vmlinuz-virt` of the Alpine netboot archive 3.22.6, the pin of
//! experiments 146, 147 and 390 (T-1687). The archive is fetched once,
//! checked against its SHA-256, and the kernel is kept in the store under
//! that digest. `PODVM_MACHINE_KERNEL` names another kernel file, for a
//! host with no network; the banner then names it.

use std::io::Read;
use std::path::{Path, PathBuf};

use podvm_image::net::{Limits, Net};

pub const URL: &str =
    "https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/alpine-netboot-3.22.6-x86_64.tar.gz";
/// The SHA-256 of the kernel file, as experiment 390 pins it.
pub const KERNEL_SHA256: &str = "6b58e5d779e44e57c9efa20232da18650415eceb9d4f544e5c165c1f392c5d51";
const MEMBER: &str = "boot/vmlinuz-virt";
/// The archive is about 90 MB; a larger answer is not the pin.
const MAX_ARCHIVE: u64 = 512 << 20;
const LIMITS: Limits = Limits {
    head: std::time::Duration::from_secs(60),
    idle: std::time::Duration::from_secs(120),
};

/// The kernel file of a guest: the variable's file, or the pinned kernel
/// in `store_root/machine/`, fetched where it is not there yet.
pub fn ensure(store_root: &Path) -> Result<PathBuf, String> {
    if let Some(p) = std::env::var_os("PODVM_MACHINE_KERNEL") {
        let p = PathBuf::from(p);
        return if p.is_file() {
            Ok(p)
        } else {
            Err(format!(
                "PODVM_MACHINE_KERNEL names {}, which is not a file",
                p.display()
            ))
        };
    }
    let dir = store_root.join("machine");
    let path = dir.join(format!("vmlinuz-{}", &KERNEL_SHA256[..16]));
    if path.is_file() && sha256_file(&path)? == KERNEL_SHA256 {
        return Ok(path);
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let (config, _roots) = podvm_image::tls::client_config();
    let mut net = Net::from_environment(config)?;
    let body = net
        .send("GET", URL, &[], true, LIMITS)
        .map_err(|e| format!("{URL}: {e}"))?;
    let gz = flate2::read::GzDecoder::new(body.take(MAX_ARCHIVE));
    let kernel = tar_member(gz, MEMBER).map_err(|e| format!("{URL}: {e}"))?;
    let got = sha256_hex(&kernel);
    if got != KERNEL_SHA256 {
        return Err(format!(
            "{URL}: {MEMBER} has SHA-256 {got}, not the pin {KERNEL_SHA256}"
        ));
    }
    let tmp = path.with_extension("partial");
    std::fs::write(&tmp, &kernel).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// The bytes of the member `name` of a tar stream (ustar or GNU headers).
pub fn tar_member<R: Read>(mut r: R, name: &str) -> Result<Vec<u8>, String> {
    let mut head = [0u8; 512];
    let mut long_name: Option<String> = None;
    loop {
        r.read_exact(&mut head)
            .map_err(|e| format!("the archive ends before {name}: {e}"))?;
        if head.iter().all(|&b| b == 0) {
            return Err(format!("the archive has no {name}"));
        }
        let field = |a: usize, n: usize| {
            let f = &head[a..a + n];
            String::from_utf8_lossy(&f[..f.iter().position(|&b| b == 0).unwrap_or(n)]).into_owned()
        };
        let size = u64::from_str_radix(field(124, 12).trim(), 8)
            .map_err(|_| "a tar header has no size".to_string())?;
        let kind = head[156];
        let prefix = field(345, 155);
        let short = field(0, 100);
        let member = long_name.take().unwrap_or_else(|| {
            if prefix.is_empty() {
                short
            } else {
                format!("{prefix}/{short}")
            }
        });
        let padded = size.div_ceil(512) * 512;
        let mut data = vec![0u8; padded as usize];
        r.read_exact(&mut data)
            .map_err(|e| format!("a member is cut: {e}"))?;
        data.truncate(size as usize);
        match kind {
            b'L' => {
                long_name = Some(
                    String::from_utf8_lossy(&data)
                        .trim_end_matches('\0')
                        .to_string(),
                )
            }
            b'0' | 0 if member.trim_start_matches("./") == name => return Ok(data),
            _ => {}
        }
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn sha256_file(p: &Path) -> Result<String, String> {
    std::fs::read(p)
        .map(|b| sha256_hex(&b))
        .map_err(|e| format!("{}: {e}", p.display()))
}
