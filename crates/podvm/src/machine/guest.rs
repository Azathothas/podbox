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
//! Limits of this slice (named in the banner and in `docs/limits.md`): no
//! volume and no `/workspace` back to the host; stdout and stderr share one
//! serial line; no stdin.

pub(crate) mod cpio;
pub(crate) mod init;
pub(crate) mod kernel;
pub(crate) mod net;

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use podvm_image::error::EXIT_RUNTIME_ERROR;
use podvm_windows::plan::Accel;

/// The version of the base archive format; a change of [`cpio`] bumps it,
/// so an old cached archive is not reused.
pub const BASE_FORMAT: u32 = 1;
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
    pub timeout: Option<Duration>,
    pub qemu_args: Vec<String>,
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

/// podvm's own bytes and the modules of the network, for a guest
/// ([`net`]).
pub struct Tools {
    pub podvm: Vec<u8>,
    pub modules: Vec<(&'static str, Vec<u8>)>,
}

/// The archive of one run: `/podvm-init`, the console nodes, and, where
/// `tools` gives them, podvm itself and the modules of the network.
pub fn run_archive(script: &str, tools: Option<&Tools>) -> std::io::Result<Vec<u8>> {
    let mut w = cpio::Newc::new(Vec::new());
    w.entry("dev", cpio::DIR | 0o755, &[], (0, 0))?;
    w.entry("dev/console", cpio::CHR | 0o600, &[], (5, 1))?;
    w.entry("dev/ttyS0", cpio::CHR | 0o660, &[], (4, 64))?;
    if let Some(t) = tools {
        w.entry("podvm", cpio::DIR | 0o755, &[], (0, 0))?;
        w.entry("podvm/podvm", cpio::REG | 0o755, &t.podvm, (0, 0))?;
        w.entry("podvm/modules", cpio::DIR | 0o755, &[], (0, 0))?;
        for (n, bytes) in &t.modules {
            w.entry(
                &format!("podvm/modules/{n}.ko"),
                cpio::REG | 0o644,
                bytes,
                (0, 0),
            )?;
        }
    }
    w.entry("podvm-init", cpio::REG | 0o755, script.as_bytes(), (0, 0))?;
    w.finish()
}

/// podvm's own bytes and the module files, for the run archive.
fn tools(modules: &[(&'static str, PathBuf)]) -> Result<Option<Tools>, String> {
    if modules.is_empty() {
        return Ok(None);
    }
    let podvm = std::fs::read("/proc/self/exe").map_err(|e| format!("/proc/self/exe: {e}"))?;
    let mut m = Vec::new();
    for (n, p) in modules {
        m.push((
            *n,
            std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?,
        ));
    }
    Ok(Some(Tools { podvm, modules: m }))
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
    let tools = match tools(&assets.modules) {
        Ok(t) => t,
        Err(e) => return fail(e),
    };
    let kernel = assets.kernel;
    let (base, cached) = match base_archive(r.store_root, r.digest, r.rootfs) {
        Ok(b) => b,
        Err(e) => return fail(e),
    };
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
            let script = init::script(&nonce, &r.argv, &r.env, &r.workdir, tools.is_some());
            run_archive(&script, tools.as_ref())
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
    let argv = qemu_argv(
        accel,
        r.mem.unwrap_or(DEFAULT_MEM),
        &kernel,
        &initrd,
        &r.qemu_args,
    );
    let code = drive(r, &qemu, &argv, &nonce, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    code
}

/// Spawn the emulator and read its console to the exit marker.
fn drive(r: &Request, qemu: &Path, argv: &[String], nonce: &str, dir: &Path) -> i32 {
    use std::os::unix::process::CommandExt;
    let errlog = dir.join("qemu.err");
    let mut child = match Command::new(qemu)
        .args(argv)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(
            std::fs::File::create(&errlog)
                .map(Stdio::from)
                .unwrap_or(Stdio::null()),
        )
        .process_group(0)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("podvm {}: machine tier: {}: {e}", r.verb, qemu.display());
            return EXIT_RUNTIME_ERROR;
        }
    };
    let pid = child.id() as i32;
    let out = child.stdout.take().expect("piped");
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        for line in BufReader::new(out).split(b'\n').map_while(Result::ok) {
            if tx
                .send(String::from_utf8_lossy(&line).into_owned())
                .is_err()
            {
                break;
            }
        }
    });
    let deadline = Instant::now() + r.timeout.unwrap_or(DEFAULT_TIMEOUT);
    let mut console = init::Console::new(nonce);
    let mut blanks = 0usize;
    let mut code: Option<i32> = None;
    let stdout = std::io::stdout();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left.max(Duration::from_millis(1))) {
            Ok(line) => match console.line(&line) {
                init::Line::Before | init::Line::Ready => {}
                init::Line::Out("") => blanks += 1,
                init::Line::Out(s) => {
                    let mut o = stdout.lock();
                    for _ in 0..blanks {
                        let _ = writeln!(o);
                    }
                    blanks = 0;
                    let _ = writeln!(o, "{s}");
                }
                init::Line::Exit(c, before) => {
                    // The script's `echo` before the marker ends a last line
                    // without a newline; that one blank line is not output.
                    let mut o = stdout.lock();
                    for _ in 0..blanks.saturating_sub(1) {
                        let _ = writeln!(o);
                    }
                    if !before.is_empty() {
                        let _ = writeln!(o, "{before}");
                    }
                    code = Some(c);
                    break;
                }
            },
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
        }
        if Instant::now() >= deadline {
            break;
        }
    }
    // The guest resets after the marker and the emulator exits; a guest
    // that does not, and each child of the emulator, goes with SIGKILL.
    let ended = wait_bounded(&mut child, Duration::from_secs(10));
    if !ended {
        // SAFETY: a plain system call on the process group that `spawn`
        // made for the emulator.
        unsafe { kill(-pid, 9) };
        let _ = child.wait();
    }
    match code {
        Some(c) => c,
        None => {
            let tail = std::fs::read_to_string(dir.join("qemu.err")).unwrap_or_default();
            let tail = tail.lines().last().unwrap_or("");
            eprintln!(
                "podvm {}: machine tier: the guest gave no exit marker{}{}",
                r.verb,
                if Instant::now() >= deadline {
                    " before the timeout"
                } else {
                    ""
                },
                if tail.is_empty() {
                    String::new()
                } else {
                    format!("; qemu: {tail}")
                }
            );
            EXIT_RUNTIME_ERROR
        }
    }
}

unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

fn wait_bounded(child: &mut std::process::Child, d: Duration) -> bool {
    let end = Instant::now() + d;
    while Instant::now() < end {
        if let Ok(Some(_)) = child.try_wait() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[cfg(test)]
mod tests;
