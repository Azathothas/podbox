//! The scope group of `podvm doctor` (TODO/cli.md T-1667, issue 188): one
//! check for each capability of the widened scope, with one of three
//! answers: `ok`, `MISSING` with a `fix:` line, or "cannot be fixed on
//! this host" with the reason. Where it can, a check proves that the thing
//! runs (a payload enters the native-static rung), not only that a file is
//! present (T-1301).

use super::Doctor;
use podvm_probe::sys;

/// The free space of the store under which a check says MISSING.
pub const SPACE_FLOOR: u64 = 2 << 30;

/// Each check of the group.
pub fn examine(store_root: Option<&std::path::Path>, report: &mut Doctor) {
    native_static(report);
    openat2(report);
    landlock(report);
    user_net(report);
    emulate(report);
    incus(report);
    proxy(report);
    match store_root {
        Some(root) => {
            space(root, report);
            assets(root, report);
        }
        None => {
            report.missing(
                "scope.space",
                "podvm cannot open its store".into(),
                "set PODVM_STORE to a directory that podvm can write".into(),
            );
            report.missing(
                "scope.assets",
                "podvm cannot open its store".into(),
                "set PODVM_STORE to a directory that podvm can write".into(),
            );
        }
    }
}

/// The native-static rung, by a run: podvm enters itself in a root of its
/// own with Syscall User Dispatch and prints its version. The syscall tier
/// takes the same run with its variable, which maps the count of /tmp.
fn native_static(report: &mut Doctor) {
    if let Err(why) = podvm_dispatch::available() {
        report.cannot("scope.native-static", why.clone());
        report.cannot(
            "scope.syscall-tier",
            format!("the tier runs on the native-static rung, and {why}"),
        );
        return;
    }
    let run = |tier: bool| -> Result<String, String> {
        let me = std::env::current_exe().map_err(|e| format!("/proc/self/exe: {e}"))?;
        let root = std::env::temp_dir().join(format!("podvm-doctor-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
        std::fs::copy(&me, root.join("podvm")).map_err(|e| format!("{}: {e}", root.display()))?;
        let mut cmd = std::process::Command::new(&me);
        cmd.args([
            podvm_dispatch::ENTER_ARG,
            &root.to_string_lossy(),
            "/podvm",
            "podvm",
            "version",
        ]);
        if tier {
            cmd.env(podvm_dispatch::TIER_VAR, "1048576");
        } else {
            cmd.env_remove(podvm_dispatch::TIER_VAR);
        }
        let out = cmd.output().map_err(|e| e.to_string());
        let _ = std::fs::remove_dir_all(&root);
        let out = out?;
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if out.status.success() && text.starts_with("podvm") {
            Ok(text)
        } else {
            Err(format!(
                "exit {:?}: {}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    };
    match run(false) {
        Ok(v) => report.ok(
            "scope.native-static",
            format!("a static payload entered and ran ({v})"),
        ),
        Err(e) => report.cannot("scope.native-static", format!("the entry failed: {e}")),
    }
    match run(true) {
        Ok(_) => report.ok(
            "scope.syscall-tier",
            "a static payload entered with the tier's count of /tmp mapped".into(),
        ),
        Err(e) => report.cannot("scope.syscall-tier", format!("the entry failed: {e}")),
    }
}

/// `openat2(RESOLVE_IN_ROOT)` (Linux 5.6), which each path of the rung
/// takes.
fn openat2(report: &mut Doctor) {
    const RESOLVE_IN_ROOT: u64 = 0x10;
    const O_PATH: u64 = 0o10000000;
    let how = [O_PATH, 0u64, RESOLVE_IN_ROOT];
    // SAFETY: openat2 reads the NUL-ended path and the 24-byte `open_how`.
    let r = unsafe {
        sys::sys(
            sys::SYS_OPENAT2,
            [
                (-100i64) as u64,
                c"/".as_ptr() as u64,
                how.as_ptr() as u64,
                24,
                0,
                0,
            ],
        )
    };
    match r {
        Ok(fd) => {
            let _ = sys::close(fd);
            report.ok("scope.openat2", "openat2(RESOLVE_IN_ROOT) answers".into())
        }
        Err(e) => report.cannot(
            "scope.openat2",
            format!(
                "openat2: {} (the kernel is older than 5.6, or a filter denies it)",
                e.name()
            ),
        ),
    }
}

/// The Landlock ABI, for the confine layer (T-1658).
fn landlock(report: &mut Doctor) {
    // SAFETY: landlock_create_ruleset(NULL, 0, VERSION) reads no memory.
    match unsafe { sys::sys(sys::SYS_LANDLOCK_CREATE_RULESET, [0, 0, 1, 0, 0, 0]) } {
        Ok(abi) if abi < i64::from(podvm_enter::confine::MIN_ABI) => report.missing(
            "scope.landlock",
            format!(
                "Landlock ABI {abi}; `--podvm-confine` needs ABI {} (TODO/enter.md T-1658)",
                podvm_enter::confine::MIN_ABI
            ),
            "a kernel of Linux 6.2 or later has ABI 3".into(),
        ),
        Ok(abi) if abi < i64::from(podvm_enter::confine::NET_ABI) => report.ok(
            "scope.landlock",
            format!(
                "Landlock ABI {abi}: `--podvm-confine` runs, with no TCP rule (`connect` needs ABI {}; TODO/enter.md T-1658)",
                podvm_enter::confine::NET_ABI
            ),
        ),
        Ok(abi) => report.ok(
            "scope.landlock",
            format!("Landlock ABI {abi}: `--podvm-confine` runs, with its TCP rules (TODO/enter.md T-1658)"),
        ),
        Err(e) => report.missing(
            "scope.landlock",
            format!("landlock_create_ruleset: {}", e.name()),
            "boot the kernel with landlock in its lsm= list (CONFIG_SECURITY_LANDLOCK, Linux 5.13 or later)".into(),
        ),
    }
}

/// A program on PATH.
fn on_path(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|p| {
        std::env::split_paths(&p)
            .map(|d| d.join(name))
            .find(|f| f.is_file())
    })
}

/// User-mode networking of the machine tier: the emulator's `user` netdev.
fn user_net(report: &mut Doctor) {
    let Some(q) = on_path("qemu-system-x86_64") else {
        report.missing(
            "scope.user-net",
            "no qemu-system-x86_64 on PATH".into(),
            "install a qemu-system-x86_64 built with libslirp (the guest network of TODO/machine.md T-1656)".into(),
        );
        return;
    };
    let out = std::process::Command::new(&q)
        .args(["-netdev", "help"])
        .output();
    match out {
        Ok(o)
            if String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|l| l.trim() == "user") =>
        {
            report.ok(
                "scope.user-net",
                format!("{} has the user netdev", q.display()),
            )
        }
        Ok(_) => report.missing(
            "scope.user-net",
            format!("{} lists no user netdev", q.display()),
            "install a qemu-system-x86_64 built with libslirp".into(),
        ),
        Err(e) => report.missing(
            "scope.user-net",
            format!("{}: {e}", q.display()),
            "install a working qemu-system-x86_64".into(),
        ),
    }
}

/// The emulate tier: a static qemu-user of the host on PATH.
fn emulate(report: &mut Doctor) {
    let path = std::env::var("PATH").unwrap_or_default();
    match crate::lifecycle::emulate::find_qemu("arm64", &path) {
        Ok(q) => report.ok("scope.emulate", format!("{} is a static qemu-user", q.display())),
        Err(why) => report.missing(
            "scope.emulate",
            why,
            "install qemu-user-static (a static qemu-aarch64-static and its kin on PATH; TODO/enter.md T-1651)".into(),
        ),
    }
}

/// The incus tier: an incusd that answers on its socket.
fn incus(report: &mut Doctor) {
    let sock = std::env::var_os("PODVM_INCUS_SOCKET")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("INCUS_DIR").map(|d| std::path::Path::new(&d).join("unix.socket"))
        })
        .unwrap_or_else(|| "/var/lib/incus/unix.socket".into());
    match std::os::unix::net::UnixStream::connect(&sock) {
        Ok(_) => report.ok("scope.incus", format!("an incusd answers at {}", sock.display())),
        Err(e) => report.missing(
            "scope.incus",
            format!("no incusd answers at {}: {e}", sock.display()),
            "start an incusd as in docs/incus.md (route A), or set INCUS_DIR (TODO/machine.md T-1718)".into(),
        ),
    }
}

/// The proxy of the environment, where one is set: it must answer.
fn proxy(report: &mut Doctor) {
    let p = ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"]
        .iter()
        .find_map(|n| std::env::var(n).ok().filter(|v| !v.is_empty()));
    let Some(url) = p else {
        report.ok(
            "scope.proxy",
            "no proxy is set: the network is direct".into(),
        );
        return;
    };
    let shown = podvm_probe::inside::scrub::scrub(&url);
    let hostport = url
        .split("://")
        .last()
        .unwrap_or("")
        .rsplit('@')
        .next()
        .unwrap_or("")
        .trim_end_matches('/')
        .to_string();
    use std::net::ToSocketAddrs;
    let addr = hostport.to_socket_addrs().ok().and_then(|mut a| a.next());
    match addr.map(|a| std::net::TcpStream::connect_timeout(&a, std::time::Duration::from_secs(3)))
    {
        Some(Ok(_)) => report.ok("scope.proxy", format!("the proxy {shown} answers")),
        Some(Err(e)) => report.missing(
            "scope.proxy",
            format!("the proxy {shown} does not answer: {e}"),
            "start the proxy, or unset the proxy variables".into(),
        ),
        None => report.missing(
            "scope.proxy",
            format!("the proxy {shown} does not resolve"),
            "correct the proxy variables".into(),
        ),
    }
}

/// The free space of the store against the floor.
fn space(root: &std::path::Path, report: &mut Doctor) {
    match podvm_image::space::read(&root.to_string_lossy()) {
        Ok(h) if h.free.bytes >= SPACE_FLOOR => report.ok(
            "scope.space",
            format!(
                "{} free at {}",
                podvm_image::space::mib(h.free.bytes),
                root.display()
            ),
        ),
        Ok(h) => report.missing(
            "scope.space",
            format!(
                "{} free at {}, under the floor of {}",
                podvm_image::space::mib(h.free.bytes),
                root.display(),
                podvm_image::space::mib(SPACE_FLOOR)
            ),
            format!(
                "free space at {}, or set PODVM_STORE to a directory with room",
                root.display()
            ),
        ),
        Err(e) => report.missing(
            "scope.space",
            format!("{}: {e}", root.display()),
            "set PODVM_STORE to a directory that podvm can read".into(),
        ),
    }
}

/// The downloaded assets and what their check proves.
fn assets(root: &std::path::Path, report: &mut Doctor) {
    let pinned = crate::assets::Proof::Pinned.words();
    let mut parts = vec![format!("the guest kernel and its modules: {pinned}")];
    if let Some(k) = std::env::var_os("PODVM_MACHINE_KERNEL") {
        let k = std::path::PathBuf::from(k);
        let rec = crate::assets::record_path(root, "machine-kernel", &k);
        let state = if rec.is_file() {
            crate::assets::Proof::Known.words()
        } else {
            "not used yet: its first use records its SHA-256"
        };
        parts.push(format!("PODVM_MACHINE_KERNEL {}: {state}", k.display()));
    }
    report.ok("scope.assets", parts.join("; "));
}
