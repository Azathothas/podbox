//! TODO/enter.md T-1651. The emulate tier: `run --podvm-tier=emulate`
//! runs the payload under a static qemu-user of the host, and the
//! emulator itself runs on the native-static rung (T-1652).
//!
//! So each path of the emulator, and so of the guest, resolves in the image
//! with `openat2(RESOLVE_IN_ROOT)`: a path absent from the image is ENOENT,
//! never a file of the host (qemu's own `-L` serves the host on a miss, so
//! podvm does not pass it), an absolute link stays in the image, and an
//! `execve` of the guest enters the emulator again through podvm. The
//! emulator is a separate program of the host, found on `PATH` as the
//! machine tier finds `qemu-system-*`; podvm loads it as a program, and
//! links nothing of it.

use super::*;

/// The rung word of the emulate tier.
pub const WORD: &str = "emulate";

/// The qemu name and the ELF machine of an OCI architecture.
pub fn qemu_arch(arch: &str) -> Option<(&'static str, u16)> {
    Some(match arch {
        "amd64" | "x86_64" => ("x86_64", 62),
        "arm64" | "aarch64" => ("aarch64", 183),
        "arm" => ("arm", 40),
        "386" | "i386" => ("i386", 3),
        "riscv64" => ("riscv64", 243),
        "ppc64le" => ("ppc64le", 21),
        "s390x" => ("s390x", 22),
        "loong64" | "loongarch64" => ("loongarch64", 258),
        _ => return None,
    })
}

/// The static qemu-user of `arch` on `PATH`: `qemu-ARCH-static`, then
/// `qemu-ARCH`. Err names each place that was read.
pub fn find_qemu(arch: &str, path: &str) -> Result<std::path::PathBuf, String> {
    let (q, _) = qemu_arch(arch)
        .ok_or_else(|| format!("podvm knows no qemu-user for the architecture {arch}"))?;
    let mut tried = Vec::new();
    for name in [format!("qemu-{q}-static"), format!("qemu-{q}")] {
        for dir in path.split(':').filter(|d| !d.is_empty()) {
            let p = std::path::Path::new(dir).join(&name);
            if !p.is_file() {
                continue;
            }
            match podvm_dispatch::static_host_program(&p) {
                Ok(()) => return Ok(p),
                Err(why) => tried.push(format!("{}: {why}", p.display())),
            }
        }
    }
    if tried.is_empty() {
        tried.push(format!("no qemu-{q}-static or qemu-{q} on PATH"));
    }
    Err(format!(
        "the emulate tier needs a static qemu-user of x86_64 for {arch}: {}",
        tried.join("; ")
    ))
}

/// The entry of the emulate tier, or a refusal by name (exit 125).
pub fn decide(
    verb: &str,
    maps: &podvm_enter::userland::Maps,
    argv: &[String],
    working_dir: &str,
    env: &[String],
    arch: &str,
) -> Result<Userland, i32> {
    let refuse = |why: String| {
        eprintln!("podvm {verb}: --podvm-tier=emulate: {why} (TODO/enter.md T-1651)");
        Err(podvm_image::error::EXIT_RUNTIME_ERROR)
    };
    if let Err(why) = podvm_dispatch::available() {
        return refuse(format!(
            "the emulator runs on the native-static rung, and {why}"
        ));
    }
    let host_path = std::env::var("PATH").unwrap_or_default();
    let qemu = match find_qemu(arch, &host_path) {
        Ok(q) => q,
        Err(why) => return refuse(why),
    };
    let path_dirs = podvm_enter::Plan::path_from(env);
    let f = match script::follow(maps, argv, &path_dirs, working_dir) {
        Ok(f) => f,
        Err((code, note, _)) => {
            eprintln!("podvm {verb}: --podvm-tier=emulate: {note} (TODO/enter.md T-1651)");
            return Err(code);
        }
    };
    let root = maps.rootfs().to_string();
    let Some(guest) = f
        .resolved
        .strip_prefix(&root)
        .filter(|g| g.starts_with('/'))
    else {
        return refuse(format!(
            "the program {} is not in the image's rootfs",
            f.resolved
        ));
    };
    // A foreign image: the emulator serves its machine. An image of this
    // machine: the caller asked for emulation, so it serves each ELF.
    let machine = match qemu_arch(arch) {
        Some((_, m)) if m != 62 => m,
        _ => 0,
    };
    Ok(Userland {
        family: podvm_enter::userland::Family::Memfd,
        loader_argv: Vec::new(),
        lib_dirs: Vec::new(),
        banner: format!(
            "entering on the emulate tier: {} emulates {arch} for the payload, on the \
             native-static rung: each path of the emulator resolves in the image with \
             openat2(RESOLVE_IN_ROOT), so a path absent from the image is ENOENT and \
             never a file of the host, and an exec of the guest enters the emulator \
             again; the emulator's process is the boundary, not hardware \
             (TODO/enter.md T-1651)",
            qemu.display()
        ),
        guest_argv: f.argv.clone(),
        gaps: Vec::new(),
        maps: maps.clone(),
        dispatch_exe: Some(guest.to_string()),
        emulator: Some(format!("{machine}:{}", qemu.display())),
    })
}
