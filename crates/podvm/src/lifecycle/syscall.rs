//! TODO/enter.md T-1657. The syscall tier: `run --podvm-tier=syscall` runs
//! a static payload on the native-static rung (T-1652), and the handler
//! answers more than the paths: `uname` gives a fixed release that names
//! the tier, `/tmp` is a directory of the run with a size cap, and the
//! links of `/proc/self` name guest paths. A `-v HOST:GUEST` volume writes
//! through to the host.
//!
//! It is the gVisor view without a user namespace, mount or ptrace. The
//! handler shares the payload's address space, so the tier is a
//! compatibility layer and not a boundary, and the banner says so.

use super::*;

/// The rung word of the syscall tier.
pub const WORD: &str = "syscall";

/// The cap of `/tmp` where `--tmpfs /tmp:size=` gives none: the default of
/// issue 169's recipes (`tmp 256m`).
pub const TMP_CAP: u64 = 256 << 20;

/// The entry of the syscall tier, or a refusal by name (exit 125).
pub fn decide(
    verb: &str,
    maps: &podvm_enter::userland::Maps,
    argv: &[String],
    working_dir: &str,
    env: &[String],
    cap: u64,
) -> Result<Userland, i32> {
    let refuse = |why: String| {
        eprintln!("podvm {verb}: --podvm-tier=syscall: {why} (TODO/enter.md T-1657)");
        Err(podvm_image::error::EXIT_RUNTIME_ERROR)
    };
    if let Err(why) = podvm_dispatch::available() {
        return refuse(format!(
            "the tier runs on the native-static rung, and {why}"
        ));
    }
    let path_dirs = podvm_enter::Plan::path_from(env);
    let f = match script::follow(maps, argv, &path_dirs, working_dir) {
        Ok(f) => f,
        Err((code, note, _)) => {
            eprintln!("podvm {verb}: --podvm-tier=syscall: {note} (TODO/enter.md T-1657)");
            return Err(code);
        }
    };
    if let Err(why) = podvm_dispatch::static_host_program(std::path::Path::new(&f.resolved)) {
        return refuse(format!(
            "the tier runs a statically linked x86_64 program, and {}: {why}. \
             --podvm-tier=emulate runs a dynamic one",
            f.resolved
        ));
    }
    let mut u = native::decide(&f, maps, "").map_err(|why| {
        eprintln!("podvm {verb}: --podvm-tier=syscall: {why} (TODO/enter.md T-1657)");
        podvm_image::error::EXIT_RUNTIME_ERROR
    })?;
    u.syscall = true;
    u.banner = format!(
        "entering on the syscall tier: the static payload runs in a podvm process of \
         its own, and Syscall User Dispatch gives each of its system calls to podvm's \
         handler, which resolves each path in the image with \
         openat2(RESOLVE_IN_ROOT), answers uname with {}, gives /tmp as a directory \
         of the run capped at {} MiB, and names guest paths in /proc/self. The handler \
         shares the payload's address space: it is a compatibility layer, not a \
         boundary (TODO/enter.md T-1657)",
        podvm_dispatch::TIER_RELEASE,
        cap >> 20
    );
    Ok(u)
}

/// The variables of the tier in the payload's environment: the tier's own
/// (the cap of /tmp, and the port of the broker that holds the payload),
/// and the proxy variables that name the broker (TODO/net.md T-1665).
pub fn tier_env(
    env: &mut Vec<String>,
    syscall: bool,
    cap: u64,
    broker: Option<&crate::broker::Started>,
) {
    let tier = podvm_dispatch::TIER_VAR;
    env.retain(|e| e.split('=').next().unwrap_or("") != tier);
    if let Some(b) = broker {
        crate::broker::replace_proxy_env(env, b.env("127.0.0.1", b.port));
    }
    if syscall {
        match broker {
            Some(b) => env.push(format!("{tier}={cap}:-:{}", b.port)),
            None => env.push(format!("{tier}={cap}")),
        }
    }
}
