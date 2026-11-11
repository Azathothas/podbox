//! The native-static rung, TODO/enter.md T-1652: a static program runs in
//! podvm's own process, and each of its system calls comes to podvm.
//!
//! `PR_SET_SYSCALL_USER_DISPATCH` (Linux 5.11) sends each system call from
//! outside podvm's own code to a SIGSYS handler in podvm
//! ([`handler`]). The handler serves a path in the root through
//! `openat2(RESOLVE_IN_ROOT)` ([`calls`], [`path`]), arms dispatch in each
//! new thread and process ([`handler`]'s clone), and runs `execve` as an
//! exec of podvm itself ([`exec`]), so the next program enters the same
//! way. No ptrace, no namespace, no mount, no chroot and no privilege.
//!
//! The loader ([`elf`], [`enter`]) maps a static x86_64 program (ET_EXEC or
//! a static PIE) and a script whose interpreter is one. A dynamic program
//! stays on the loader family.
//!
//! podvm enters through [`main_if_entered`] at the start of `main`: the
//! variables [`ROOT_VAR`] and [`EXE_VAR`] name the root and the program,
//! and argv is the payload's.

#[cfg(target_arch = "x86_64")]
mod calls;
#[cfg(target_arch = "x86_64")]
pub mod elf;
#[cfg(target_arch = "x86_64")]
mod enter;
#[cfg(target_arch = "x86_64")]
mod exec;
#[cfg(target_arch = "x86_64")]
mod handler;
#[cfg(target_arch = "x86_64")]
mod path;
#[cfg(target_arch = "x86_64")]
mod pool;
#[cfg(target_arch = "x86_64")]
mod procself;
#[cfg(target_arch = "x86_64")]
mod signals;
#[cfg(target_arch = "x86_64")]
mod state;
#[cfg(target_arch = "x86_64")]
mod sys;

#[cfg(all(test, target_arch = "x86_64"))]
mod tests;

use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt;

/// The variable that names the root (its value is the host path).
pub const ROOT_VAR: &str = "PODVM_DISPATCH_ROOT";
/// The variable that names the program (its guest path).
pub const EXE_VAR: &str = "PODVM_DISPATCH_EXE";
/// The variable that carries the emulator of the emulate tier across an
/// `execve` (`MACHINE:HOSTPATH`, TODO/enter.md T-1651).
pub const EMU_VAR: &str = "PODVM_DISPATCH_EMU";
/// The argument after [`ENTER_ARG`] that names the emulator.
pub const EMU_ARG: &str = "--podvm-dispatch-emu=";

/// The variable that names podvm itself (its host path) for the
/// interposer: a static child of the loader family enters this rung
/// through it (TODO/enter.md T-1724). podvm sets it where [`available`]
/// says yes.
pub const SELF_VAR: &str = "PODVM_DISPATCH_SELF";

/// Whether this build and this kernel can serve the rung: x86_64, and a
/// `prctl(PR_SET_SYSCALL_USER_DISPATCH)` that the kernel takes. The error
/// names the reason.
pub fn available() -> Result<(), String> {
    #[cfg(target_arch = "x86_64")]
    {
        probe_prctl()
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        Err(format!(
            "the native-static rung is x86_64 only, and this build is {}",
            std::env::consts::ARCH
        ))
    }
}

/// Whether `path` is a static program of this machine that the rung can
/// load (TODO/enter.md T-1651: the emulator of the emulate tier).
pub fn static_host_program(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_arch = "x86_64")]
    {
        let b = std::fs::read(path).map_err(|e| e.to_string())?;
        let read = |off: u64, out: &mut [u8]| {
            let s = b.get(off as usize..off as usize + out.len()).unwrap_or(&[]);
            out[..s.len()].copy_from_slice(s);
            s.len() as i64
        };
        elf::static_x86_64(b.get(..64).unwrap_or(&[]), read).map_err(str::to_string)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = path;
        available()
    }
}

/// Arm and disarm dispatch in a thread of its own; the thread makes no
/// call while armed.
#[cfg(target_arch = "x86_64")]
fn probe_prctl() -> Result<(), String> {
    std::thread::spawn(|| {
        // SAFETY: a NULL selector and a region of the whole address space:
        // no call of this thread dispatches, and the kernel reads no memory.
        let on = unsafe { sys::sc6(sys::nr::PRCTL, 59, 1, 0, i64::MAX, 0, 0) };
        // SAFETY: as above; PR_SYS_DISPATCH_OFF.
        unsafe { sys::sc6(sys::nr::PRCTL, 59, 0, 0, 0, 0, 0) };
        if on < 0 {
            Err(format!(
                "prctl(PR_SET_SYSCALL_USER_DISPATCH) gives errno {}",
                -on
            ))
        } else {
            Ok(())
        }
    })
    .join()
    .unwrap_or_else(|_| Err("the probe thread panicked".into()))
}

/// The first argument that enters a payload from podvm's own entry:
/// `podvm --podvm-dispatch-enter ROOT EXE ARGV...`.
pub const ENTER_ARG: &str = "--podvm-dispatch-enter";

/// Enter the payload where podvm was started for it: with [`ENTER_ARG`]
/// (the first entry), or with [`ROOT_VAR`] set (the `execve` of a payload).
/// It never returns then, except with exit code 127 and a message where
/// the program cannot be loaded. Call it first in `main`.
pub fn main_if_entered(product: &str) {
    let mut args: Vec<OsString> = std::env::args_os().collect();
    let mut emu: Option<OsString> = std::env::var_os(EMU_VAR);
    if args.get(1).is_some_and(|a| a == ENTER_ARG)
        && let Some(spec) = args
            .get(2)
            .and_then(|a| a.as_bytes().strip_prefix(EMU_ARG.as_bytes()))
    {
        emu = Some(OsString::from(std::ffi::OsStr::from_bytes(spec)));
        args.remove(2);
    }
    let (root, exe, argv) = if args.get(1).is_some_and(|a| a == ENTER_ARG) && args.len() >= 5 {
        let rest = args.split_off(4);
        (args[2].clone(), args[3].clone(), rest)
    } else if let Some(root) = std::env::var_os(ROOT_VAR) {
        (root, std::env::var_os(EXE_VAR).unwrap_or_default(), args)
    } else {
        return;
    };
    // SAFETY: the process has one thread at the start of main.
    unsafe {
        std::env::remove_var(ROOT_VAR);
        std::env::remove_var(EXE_VAR);
        std::env::remove_var(EMU_VAR);
    }
    let envp: Vec<OsString> = std::env::vars_os()
        .map(|(k, v)| {
            let mut s = k;
            s.push("=");
            s.push(v);
            s
        })
        .collect();
    let why = run(
        std::path::Path::new(&root),
        exe.as_bytes(),
        emu.as_ref().map(|e| e.as_bytes()),
        argv,
        envp,
    );
    eprintln!("{product}: native-static rung: {why} (TODO/enter.md T-1652)");
    std::process::exit(127);
}

#[cfg(target_arch = "x86_64")]
fn run(
    root: &std::path::Path,
    exe: &[u8],
    emu: Option<&[u8]>,
    argv: Vec<OsString>,
    envp: Vec<OsString>,
) -> String {
    enter::enter(root, exe, emu, argv, envp)
}

#[cfg(not(target_arch = "x86_64"))]
fn run(
    _: &std::path::Path,
    _: &[u8],
    _: Option<&[u8]>,
    _: Vec<OsString>,
    _: Vec<OsString>,
) -> String {
    available().err().unwrap_or_default()
}
