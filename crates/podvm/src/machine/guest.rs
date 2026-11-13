//! `run --podvm-tier=machine`: a disposable guest from an OCI image,
//! TODO/machine.md T-1656.
//!
//! The rootfs of the image becomes a `newc` base archive, built once for
//! each image digest and kept in the store; each run appends a small
//! archive with `/podvm-init` and the console nodes. QEMU boots the pinned
//! kernel ([`kernel`]) with that initrd, under KVM where `/dev/kvm` opens
//! and under TCG elsewhere. `PODVM_MACHINE_ACCEL=kvm` asks for KVM and then
//! refuses by name where it does not open, with no fall back to TCG; `tcg`
//! asks for TCG. The command's exit code comes back through the markers of
//! [`init`]. The guest has user-mode networking.
//!
//! A `-v HOST:GUEST` is a 9p mount of the host directory ([`vol`],
//! TODO/machine.md T-1723), so a write in the guest lands on the host; with
//! a kernel of `PODVM_MACHINE_KERNEL` (no 9p module) it is a copy. `-t`
//! runs the command on a terminal of a virtio console ([`tty`]).

pub(crate) mod archive;
pub(crate) mod cpio;
mod drive;
pub(crate) mod init;
pub(crate) mod kernel;
pub(crate) mod net;
pub(crate) mod squashfs;
pub(crate) mod stderr;
pub(crate) mod tty;
pub(crate) mod vol;

use std::io::Write;
use std::path::{Path, PathBuf};

use std::time::Duration;

use podvm_image::error::EXIT_RUNTIME_ERROR;
use podvm_windows::plan::Accel;

pub use archive::run_archive;
use archive::tools;
use drive::drive;

/// The version of the base archive format; a change of [`cpio`] bumps it,
/// so an old cached archive is not reused. 2 keeps the hard links; 3 the
/// owners of the image (T-1776).
pub const BASE_FORMAT: u32 = 3;
const DEFAULT_MEM: u64 = 512 << 20;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);

/// One guest run.
pub struct Request<'a> {
    pub verb: &'a str,
    pub store_root: &'a Path,
    pub rootfs: &'a Path,
    pub digest: &'a str,
    pub argv: Vec<String>,
    pub env: Vec<String>,
    pub workdir: String,
    pub mem: Option<u64>,
    /// `--podvm-cpus`: the `-smp` of the guest; one where it is `None`.
    pub cpus: Option<u32>,
    /// `--network broker` (TODO/net.md T-1665): the port of the broker on
    /// the host's loopback. The guest network is restricted, and its one
    /// way out is a forward to the broker.
    pub broker: Option<u16>,
    pub timeout: Option<Duration>,
    pub qemu_args: Vec<String>,
    /// Host paths copied into the guest at GUEST (the probe of
    /// probe-inside). Writes in the guest do not return to the host.
    pub copies: Vec<(PathBuf, String)>,
    /// `-v HOST:GUEST[:ro]`: each a 9p mount of the host directory
    /// (TODO/machine.md T-1723); `true` is read-only.
    pub volumes: Vec<(PathBuf, String, bool)>,
    /// podvm's stdin where it is a pipe or a file, read to its end: the
    /// command's stdin in the guest (`/podvm/stdin`).
    pub stdin: Option<Vec<u8>>,
    /// `-t`: the command runs on a terminal of a virtio console, and stdin
    /// goes there live ([`tty`], TODO/machine.md T-1723).
    pub tty: bool,
}

/// The most bytes of stdin that a guest takes.
pub const MAX_STDIN: u64 = 64 << 20;

/// podvm's stdin for the guest: its bytes where it is a pipe or a regular
/// file; `None` for a terminal or anything else, which gives the guest
/// `/dev/null`. More than [`MAX_STDIN`] bytes is an error.
pub fn read_stdin() -> Result<Option<Vec<u8>>, String> {
    use std::io::Read;
    use std::os::unix::fs::FileTypeExt;
    let Ok(md) = std::fs::metadata("/proc/self/fd/0") else {
        return Ok(None);
    };
    if !(md.file_type().is_fifo() || md.is_file()) {
        return Ok(None);
    }
    let mut buf = Vec::new();
    std::io::stdin()
        .lock()
        .take(MAX_STDIN + 1)
        .read_to_end(&mut buf)
        .map_err(|e| format!("stdin: {e}"))?;
    if buf.len() as u64 > MAX_STDIN {
        return Err(format!(
            "stdin holds more than {MAX_STDIN} bytes, the most that the guest takes"
        ));
    }
    Ok(Some(buf))
}

/// The cache file of the base archive of `digest`.
pub fn base_path(store_root: &Path, digest: &str) -> PathBuf {
    let hex = digest.trim_start_matches("sha256:");
    store_root
        .join("machine")
        .join(format!("base-{hex}-v{BASE_FORMAT}.cpio"))
}

/// The base archive of the rootfs, from the cache or built now. `true`
/// where the cache answered.
pub fn base_archive(
    store_root: &Path,
    digest: &str,
    rootfs: &Path,
) -> Result<(PathBuf, bool), String> {
    let path = base_path(store_root, digest);
    if path.is_file() {
        return Ok((path, true));
    }
    let dir = path.parent().expect("a file under machine/");
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tmp = path.with_extension(format!("partial-{}", std::process::id()));
    let file = std::fs::File::create(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    let mut w = cpio::Newc::new(std::io::BufWriter::new(file));
    // TODO/machine.md T-1776: the owners of the image, from the sidecar
    // beside the rootfs; with no sidecar each file stays root's.
    if let Some(dir) = rootfs.parent() {
        let _ = w.owners_from(&dir.join(".meta.jsonl"));
    }
    let built = w
        .tree(rootfs)
        .and_then(|()| w.finish())
        .and_then(|mut b| b.flush());
    if let Err(e) = built {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("the base archive of {}: {e}", rootfs.display()));
    }
    std::fs::rename(&tmp, &path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((path, false))
}

/// The accelerator: `Err` where KVM was asked for and does not open.
pub fn accel(want: Option<&str>, kvm_opens: bool) -> Result<Accel, String> {
    match want {
        Some("kvm") if kvm_opens => Ok(Accel::Kvm),
        Some("kvm") => Err("PODVM_MACHINE_ACCEL=kvm asks for KVM, and /dev/kvm does not open \
                            for this user: podvm does not fall back to TCG (TODO/machine.md T-1656)"
            .to_string()),
        Some("tcg") => Ok(Accel::Tcg),
        Some(other) => Err(format!("PODVM_MACHINE_ACCEL={other}: the values are kvm and tcg")),
        None if kvm_opens => Ok(Accel::Kvm),
        None => Ok(Accel::Tcg),
    }
}

/// The emulator's arguments.
pub fn qemu_argv(
    accel: Accel,
    mem: u64,
    kernel: &Path,
    initrd: &Path,
    extra: &[String],
) -> Vec<String> {
    let mut v: Vec<String> = [
        "-M",
        "pc,acpi=off",
        "-nodefaults",
        "-no-user-config",
        "-display",
        "none",
        "-serial",
        "stdio",
        "-no-reboot",
        "-smp",
        "1",
        "-nic",
        "user,model=virtio-net-pci",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    v.push("-accel".to_string());
    v.push(match accel {
        Accel::Kvm => "kvm".to_string(),
        Accel::Tcg => "tcg,thread=multi".to_string(),
    });
    v.push("-cpu".to_string());
    v.push(accel.cpu().to_string());
    v.push("-m".to_string());
    v.push(format!("{}M", mem.div_ceil(1 << 20)));
    v.push("-kernel".to_string());
    v.push(kernel.display().to_string());
    v.push("-initrd".to_string());
    v.push(initrd.display().to_string());
    v.push("-append".to_string());
    v.push("console=ttyS0 quiet loglevel=0 panic=-1 rdinit=/podvm-init".to_string());
    v.extend(extra.iter().cloned());
    v
}

/// Boot the guest, run the command, and give its exit code.
pub fn boot(r: &Request) -> i32 {
    let fail = |what: String| {
        eprintln!("podvm {}: machine tier: {what}", r.verb);
        EXIT_RUNTIME_ERROR
    };
    let want = std::env::var("PODVM_MACHINE_ACCEL").ok();
    let accel = match accel(want.as_deref(), crate::machine::ssh::kvm_opens()) {
        Ok(a) => a,
        Err(e) => return fail(e),
    };
    let Some(qemu) = crate::tier::which_qemu("qemu-system-x86_64") else {
        return fail("qemu-system-x86_64 is not on PATH".to_string());
    };
    if std::fs::symlink_metadata(r.rootfs.join("bin/sh")).is_err() {
        return fail("the image has no /bin/sh, which runs /podvm-init in the guest".to_string());
    }
    let assets = match kernel::ensure(r.store_root) {
        Ok(k) => k,
        Err(e) => return fail(format!("the guest kernel: {e}")),
    };
    // TODO/machine.md T-1723: a volume is a 9p mount where the kernel has
    // the modules (the pin); a kernel of PODVM_MACHINE_KERNEL has none, and
    // a volume is then a copy, as before.
    let nine = !r.volumes.is_empty() && !assets.volume_modules.is_empty();
    let mut copies = r.copies.clone();
    let vols = if nine {
        let g: Vec<_> = r
            .volumes
            .iter()
            .map(|(_, g, ro)| (g.clone(), *ro))
            .collect();
        vol::vols(&g)
    } else {
        copies.extend(r.volumes.iter().map(|(h, g, _)| (h.clone(), g.clone())));
        Vec::new()
    };
    let mut modules = assets.modules.clone();
    if nine {
        modules.extend(assets.volume_modules.iter().cloned());
    }
    let tools = match tools(&modules) {
        Ok(t) => t,
        Err(e) => return fail(e),
    };
    let kernel = assets.kernel;
    let (base, cached) = match base_archive(r.store_root, r.digest, r.rootfs) {
        Ok(b) => b,
        Err(e) => return fail(e),
    };
    if r.tty && tools.is_none() {
        return fail(
            "-t needs podvm in the guest, and the kernel of PODVM_MACHINE_KERNEL brings no \
             module set for it (TODO/machine.md T-1723)"
                .to_string(),
        );
    }
    let size = tty::size()
        .map(|(rows, cols)| format!("{rows}x{cols}"))
        .unwrap_or_default();
    let nonce = match init::nonce() {
        Ok(n) => n,
        Err(e) => return fail(e),
    };
    let dir =
        r.store_root
            .join("machine")
            .join(format!("run-{}-{}", std::process::id(), &nonce[..8]));
    let initrd = dir.join("initrd.cpio");
    let made = std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::copy(&base, &initrd))
        .and_then(|_| {
            let script = init::script(
                &nonce,
                &r.argv,
                &r.env,
                &r.workdir,
                tools.is_some(),
                &vols,
                r.tty.then_some(size.as_str()),
            );
            run_archive(&script, tools.as_ref(), &copies, r.stdin.as_deref())
        })
        .and_then(|extra| {
            let mut f = std::fs::OpenOptions::new().append(true).open(&initrd)?;
            f.write_all(&extra)
        });
    if let Err(e) = made {
        let _ = std::fs::remove_dir_all(&dir);
        return fail(format!("the initrd of the run: {e}"));
    }
    eprintln!(
        "podvm {}: machine tier: a guest under {} ({}), base archive {} (TODO/machine.md T-1656)",
        r.verb,
        match accel {
            Accel::Kvm => "KVM",
            Accel::Tcg => "TCG",
        },
        match accel {
            Accel::Kvm => "hardware virtualization",
            Accel::Tcg => "the emulator process is the boundary, not hardware isolation",
        },
        if cached {
            "from the cache"
        } else {
            "built now"
        }
    );
    for (host, guest, ro) in &r.volumes {
        if nine && *ro {
            eprintln!(
                "podvm {}: machine tier: -v {}:{guest}:ro is a read-only 9p mount of the \
                 host directory (TODO/machine.md T-1723)",
                r.verb,
                host.display()
            );
        } else if nine {
            eprintln!(
                "podvm {}: machine tier: -v {}:{guest} is a 9p mount of the host directory: \
                 a write in the guest lands there, with the host user's ownership \
                 (TODO/machine.md T-1723)",
                r.verb,
                host.display()
            );
        } else {
            eprintln!(
                "podvm {}: machine tier: -v {}:{guest} is a copy into the guest: the kernel \
                 of PODVM_MACHINE_KERNEL has no 9p module, so writes in the guest do not \
                 return to the host (TODO/machine.md T-1723)",
                r.verb,
                host.display()
            );
        }
    }
    let sock = stderr::socket_path(&nonce);
    let _ = std::fs::remove_file(&sock);
    let mut extra = stderr::argv(&sock);
    if r.tty {
        let _ = std::fs::remove_file(tty::socket_path(&sock));
        extra.extend(tty::argv(&tty::socket_path(&sock)));
    }
    for (v, (host, _, _)) in vols.iter().zip(&r.volumes) {
        extra.push("-virtfs".to_string());
        extra.push(vol::virtfs(host, v));
    }
    extra.extend(r.qemu_args.iter().cloned());
    let mut argv = qemu_argv(
        accel,
        r.mem.unwrap_or(DEFAULT_MEM),
        &kernel,
        &initrd,
        &extra,
    );
    if let Some(port) = r.broker {
        let (h, p) = crate::broker::GUEST_FORWARD;
        if let Some(nic) = argv
            .iter_mut()
            .find(|a| a.starts_with("user,model=virtio-net-pci"))
        {
            // One relay of podvm for each connection of the guest: a
            // `tcp:` chardev would be one connection for the whole run.
            let me = std::env::current_exe().unwrap_or_default();
            *nic = format!(
                "{nic},restrict=on,guestfwd=tcp:{h}:{p}-cmd:{} {} 127.0.0.1:{port}",
                me.display(),
                crate::broker::RELAY_ARG
            );
        }
    }
    // TODO/cli.md T-1654 (`limits.cpus`). With `acpi=off` the kernel reads
    // the MP table, which names one CPU, so a guest of more CPUs keeps ACPI
    // (measured on 2026-11-11: `nproc` 1 with `acpi=off`, 2 without it).
    if let Some(n) = r.cpus.filter(|n| *n > 1) {
        if let Some(i) = argv.iter().position(|a| a == "-smp") {
            argv[i + 1] = n.to_string();
        }
        if let Some(m) = argv.iter_mut().find(|a| *a == "pc,acpi=off") {
            *m = "pc".to_string();
        }
    }
    let code = drive(r, &qemu, &argv, &nonce, &dir, &sock);
    let _ = std::fs::remove_file(&sock);
    let _ = std::fs::remove_file(tty::socket_path(&sock));
    let _ = std::fs::remove_dir_all(&dir);
    code
}

#[cfg(test)]
mod tests;
