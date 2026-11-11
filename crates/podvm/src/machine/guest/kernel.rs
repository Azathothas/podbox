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

/// The archive and the SHA-256 of its kernel file come from `pod-services`
/// (`alpine-netboot`, TODO/deps.md T-1679): podvm keeps no second copy of a
/// host of another party.
fn netboot() -> &'static pod_services::Service {
    pod_services::get("alpine-netboot")
        .expect("pod-services holds alpine-netboot (TODO/deps.md T-1679)")
}

/// The URL of the netboot archive.
pub fn url() -> String {
    netboot()
        .endpoint("archive")
        .expect("alpine-netboot has the endpoint archive")
        .url()
}

/// The SHA-256 of the kernel file, as experiment 390 pins it.
pub fn kernel_sha256() -> &'static str {
    netboot()
        .text("vmlinuz-virt-sha256")
        .expect("alpine-netboot has the fact vmlinuz-virt-sha256")
}
const MEMBER: &str = "boot/vmlinuz-virt";
/// The archive is about 90 MB; a larger answer is not the pin.
const MAX_ARCHIVE: u64 = 512 << 20;
const LIMITS: Limits = Limits {
    head: std::time::Duration::from_secs(60),
    idle: std::time::Duration::from_secs(120),
};

/// The modules of the network, out of `boot/initramfs-virt` of the same
/// archive, each with its SHA-256 (T-1656). The NIC driver of the pinned
/// kernel is a module. The sums are facts of `alpine-netboot` in
/// `pod-services` (TODO/deps.md T-1679).
pub fn modules() -> [(&'static str, &'static str); 3] {
    let fact = |name: &str| {
        netboot()
            .text(name)
            .unwrap_or_else(|| panic!("alpine-netboot has the fact {name}"))
    };
    [
        ("failover", fact("module-failover-sha256")),
        ("net_failover", fact("module-net-failover-sha256")),
        ("virtio_net", fact("module-virtio-net-sha256")),
    ]
}
const INITRAMFS: &str = "boot/initramfs-virt";

/// What a guest boots with: the kernel, and the module files of the
/// network (none where `PODVM_MACHINE_KERNEL` names the kernel).
pub struct Assets {
    pub kernel: PathBuf,
    pub modules: Vec<(&'static str, PathBuf)>,
}

fn cached(dir: &Path) -> Option<Assets> {
    let kernel = dir.join(format!("vmlinuz-{}", &kernel_sha256()[..16]));
    let modules: Vec<_> = modules()
        .iter()
        .map(|(n, sha)| (*n, dir.join(format!("{n}-{}.ko", &sha[..16]))))
        .collect();
    let all = kernel.is_file() && modules.iter().all(|(_, p)| p.is_file());
    all.then_some(Assets { kernel, modules })
}

/// The guest's kernel and modules: the variable's kernel, or the pin in
/// `store_root/machine/`, fetched where it is not there yet.
pub fn ensure(store_root: &Path) -> Result<Assets, String> {
    if let Some(p) = std::env::var_os("PODVM_MACHINE_KERNEL") {
        let p = PathBuf::from(p);
        return if p.is_file() {
            // TODO/cli.md T-1667: a kernel of the caller has no pin, so its
            // first use is recorded and a change refuses.
            crate::assets::verify(store_root, "machine-kernel", &p, None)
                .map_err(|e| format!("PODVM_MACHINE_KERNEL: {e}"))?;
            Ok(Assets {
                kernel: p,
                modules: Vec::new(),
            })
        } else {
            Err(format!(
                "PODVM_MACHINE_KERNEL names {}, which is not a file",
                p.display()
            ))
        };
    }
    let dir = store_root.join("machine");
    if let Some(a) = cached(&dir) {
        return Ok(a);
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    // TODO/cli.md T-1667: the floor before the download (the kernel and its
    // modules are some 15 MB; the floor leaves room for them twice over).
    crate::assets::space_floor(&dir, "the guest kernel of the machine tier", 64 << 20)?;
    let (config, _roots) = podvm_image::tls::client_config();
    let mut net = Net::from_environment(config)?;
    let url = url();
    let body = net
        .send("GET", &url, &[], true, LIMITS)
        .map_err(|e| format!("{url}: {e}"))?;
    let gz = flate2::read::GzDecoder::new(body.take(MAX_ARCHIVE));
    let mut got = tar_members(gz, &[MEMBER, INITRAMFS]).map_err(|e| format!("{url}: {e}"))?;
    let initramfs = got
        .pop()
        .flatten()
        .ok_or(format!("{url}: no {INITRAMFS}"))?;
    let kernel = got.pop().flatten().ok_or(format!("{url}: no {MEMBER}"))?;
    check(&kernel, kernel_sha256(), MEMBER)?;
    let mut cpio = Vec::new();
    flate2::read::GzDecoder::new(&initramfs[..])
        .read_to_end(&mut cpio)
        .map_err(|e| format!("{INITRAMFS}: {e}"))?;
    let put = |name: String, bytes: &[u8]| {
        let path = dir.join(name);
        let tmp = path.with_extension("partial");
        std::fs::write(&tmp, bytes)
            .and_then(|()| std::fs::rename(&tmp, &path))
            .map_err(|e| format!("{}: {e}", path.display()))
    };
    for (n, sha) in modules() {
        let want = format!("/{n}.ko");
        let bytes = super::cpio::member(&cpio, &want).ok_or(format!("{INITRAMFS}: no {n}.ko"))?;
        check(bytes, sha, &format!("{n}.ko"))?;
        put(format!("{n}-{}.ko", &sha[..16]), bytes)?;
    }
    put(format!("vmlinuz-{}", &kernel_sha256()[..16]), &kernel)?;
    cached(&dir).ok_or_else(|| format!("{}: the assets did not stay", dir.display()))
}

fn check(bytes: &[u8], want: &str, what: &str) -> Result<(), String> {
    let got = sha256_hex(bytes);
    if got == want {
        Ok(())
    } else {
        Err(format!(
            "{}: {what} has SHA-256 {got}, not the pin {want}",
            url()
        ))
    }
}

/// The bytes of each member of `names` of a tar stream, in that order.
pub fn tar_members<R: Read>(mut r: R, names: &[&str]) -> Result<Vec<Option<Vec<u8>>>, String> {
    let mut out: Vec<Option<Vec<u8>>> = vec![None; names.len()];
    loop {
        if out.iter().all(Option::is_some) {
            return Ok(out);
        }
        match next_member(&mut r)? {
            None => return Ok(out),
            Some((name, data)) => {
                if let Some(i) = names.iter().position(|n| *n == name) {
                    out[i] = Some(data);
                }
            }
        }
    }
}

/// The next regular member of a tar stream, `./` taken off its name;
/// `None` at the end.
fn next_member<R: Read>(r: &mut R) -> Result<Option<(String, Vec<u8>)>, String> {
    let mut head = [0u8; 512];
    let mut long_name: Option<String> = None;
    loop {
        if r.read_exact(&mut head).is_err() || head.iter().all(|&b| b == 0) {
            return Ok(None);
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
            b'0' | 0 => return Ok(Some((member.trim_start_matches("./").to_string(), data))),
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
