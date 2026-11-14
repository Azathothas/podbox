//! TODO/enter.md T-1652. The native-static rung: a static payload runs in
//! a podvm process of its own, and Syscall User Dispatch gives each of its
//! system calls to podvm (`podvm-dispatch`), which resolves each path in
//! the image with `openat2(RESOLVE_IN_ROOT)`.
//!
//! It is the first choice for a native static payload where chroot is
//! denied and the caller did not ask for host paths
//! (`--unsafe-host-paths` keeps the memfd family). Where the kernel
//! refuses the `prctl` or the build is not x86_64, the memfd family's rules
//! hold (T-1648), and the refusal names the reason. A volume (`-v`) maps to
//! a host directory descriptor of the handler (T-1724).

use super::*;
use podvm_enter::ExitCode as _;

/// The hidden first argument of podvm that enters a payload:
/// `podvm --podvm-dispatch-enter ROOT EXE ARGV...`.
pub const ENTER: &str = podvm_dispatch::ENTER_ARG;

/// TODO/enter.md T-1648. Why a static payload refuses where chroot is
/// denied and `--unsafe-host-paths` is absent, and the routes past the
/// refusal. A constant, so the tests pin the sentence a caller reads.
pub const STATIC_HOST_PATHS: &str = "the payload is statically linked, so \
     it makes raw syscalls past the interposer and podvm rewrites no path: \
     its absolute paths would resolve on the host, and a write would land \
     on the host tree. podvm refuses before the exec. Use a host where \
     chroot(2) is permitted, or `--unsafe-host-paths` to accept this host \
     pass-through on the memfd family. The native-static rung \
     (TODO/enter.md T-1652) serves a static payload where the kernel takes \
     prctl(PR_SET_SYSCALL_USER_DISPATCH); the planned rung where it does \
     not is the emulate tier (TODO/enter.md T-1651) (TODO/enter.md T-1648)";

/// The rung word of the native-static rung.
pub const WORD: &str = "native-static";

/// The entry on the native-static rung, or the reason why this run cannot
/// take it.
pub(super) fn decide(
    f: &script::Followed,
    maps: &podvm_enter::userland::Maps,
    script_note: &str,
) -> Result<Userland, String> {
    podvm_dispatch::available()?;
    let root = maps.rootfs().to_string();
    // The guest path of the program: in the rootfs, or in a volume, whose
    // host path the handler maps back (TODO/cli.md T-1659: a static program
    // of the project runs in a distroless image).
    let in_volume = maps.volumes().iter().find_map(|v| {
        let host = v.host.trim_end_matches('/');
        let rest = f.resolved.strip_prefix(host)?;
        (rest.is_empty() || rest.starts_with('/'))
            .then(|| format!("{}{rest}", v.guest.trim_end_matches('/')))
    });
    let guest = match f
        .resolved
        .strip_prefix(&root)
        .filter(|g| g.starts_with('/'))
    {
        Some(g) => g.to_string(),
        None => in_volume.ok_or_else(|| {
            format!(
                "the program {} is in neither the image's rootfs nor a volume",
                f.resolved
            )
        })?,
    };
    Ok(Userland {
        family: podvm_enter::userland::Family::Memfd,
        loader_argv: Vec::new(),
        lib_dirs: Vec::new(),
        banner: format!(
            "entering without chroot on the native-static rung: the static payload \
             runs in a podvm process of its own, Syscall User Dispatch gives each of \
             its system calls to podvm, and podvm resolves each path in the image \
             with openat2(RESOLVE_IN_ROOT); /proc, /sys and /dev are the host's \
             (TODO/enter.md T-1652){script_note}"
        ),
        guest_argv: f.argv.clone(),
        gaps: Vec::new(),
        maps: maps.clone(),
        dispatch_exe: Some(guest.to_string()),
        emulator: None,
        syscall: false,
    })
}

/// Run a payload of the native-static rung: podvm itself, with the root,
/// the program and the payload's argv, through the entry of the userland
/// family (its working directory, its limits and its wait).
pub(super) fn enter(
    verb: &str,
    root: &podvm_enter::RootDir,
    plan: &podvm_enter::Plan,
    userland: &Userland,
    exe: &str,
    err: &mut dyn Write,
    on_spawn: &mut dyn FnMut(i64, &mut dyn Write),
) -> UserlandOutcome {
    let me = match std::fs::read_link("/proc/self/exe") {
        Ok(p) => p.to_string_lossy().into_owned(),
        Err(e) => {
            let _ = writeln!(err, "podvm {verb}: /proc/self/exe: {e}");
            return UserlandOutcome {
                code: podvm_image::error::EXIT_RUNTIME_ERROR,
                entered: false,
            };
        }
    };
    let mut argv = vec![me, ENTER.to_string()];
    // T-1651: the emulator of the emulate tier comes before the root.
    if let Some(spec) = &userland.emulator {
        argv.push(format!("{}{spec}", podvm_dispatch::EMU_ARG));
    }
    argv.extend([root.path.clone(), exe.to_string()]);
    argv.extend(userland.guest_argv.iter().cloned());
    let word = if userland.emulator.is_some() {
        super::emulate::WORD
    } else if userland.syscall {
        super::syscall::WORD
    } else {
        WORD
    };
    // TODO/supervise.md T-1786: the pid goes to the hold of a foreground
    // run before the wait.
    let ran = podvm_enter::spawn_userland(root, plan, &userland.maps, argv, word, None, err)
        .and_then(|c| {
            on_spawn(c.pid, err);
            c.wait_forwarding(err)
        });
    match ran {
        Ok(c) => UserlandOutcome {
            code: c,
            entered: true,
        },
        Err(e) => {
            let _ = writeln!(err, "podvm {verb}: {e}");
            UserlandOutcome {
                code: e.exit_code(),
                entered: true,
            }
        }
    }
}
