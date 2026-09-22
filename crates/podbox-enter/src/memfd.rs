//! `TOOL.md` section 3.5, [`TODO/deps.md`](../../../TODO/deps.md) T-0909:
//! podbox's own memfd launch path.
//!
//! Three syscalls (`memfd_create`, `write`, `fexecve` by fd), declared here
//! rather than taken from a crate: the fork's value is two mechanisms and a
//! ladder, all smaller re-implemented than carried, and carrying it would
//! import a regression this tree would then have to patch out (an
//! `MFD_CLOEXEC` stub that never sets the flag).
//!
//! The two mechanisms, stated in this project's own words:
//!
//! 1. `MFD_CLOEXEC` is the unconditional base of every `memfd_create` call,
//!    so the memfd never leaks across a later `execve`;
//! 2. a payload is probed for executability **before** it is written, so a
//!    non-executable memfd becomes `EACCES` at the probe rather than a
//!    confusing `fexecve` failure later.
//!
//! ⚠ `#!` scripts fail through fd-exec and route past it: the ladder execs
//! them by path instead of through the memfd.

#![forbid(unsafe_op_in_unsafe_fn)]

use std::path::Path;

use podbox_probe::sys::{self, Sysres};

/// Close-on-exec, from `linux/memfd.h`. Unconditional: every call below sets
/// it, so no later `execve` inherits the memfd.
pub const MFD_CLOEXEC: u64 = 0x0001;

/// Allow seals, from `linux/memfd.h`. Added only when the caller asks for
/// sealing and the kernel accepts it.
pub const MFD_ALLOW_SEALING: u64 = 0x0002;

/// Whether these bytes route past fd-exec: a `#!` script fails through it,
/// so the ladder execs scripts by path instead of through the memfd.
pub fn has_shebang(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == b'#' && bytes[1] == b'!'
}

/// Whether `path` names something the kernel would execute: a file, with an
/// execute bit set, passing an `X_OK` access check.
///
/// ⚠ Both halves, because either alone mis-answers: for root the bits may
/// read `0o644` while the check still succeeds, and for a non-owner they may
/// read `0o755` while it fails. A path that is not UTF-8 answers false: the
/// probe cannot name it to the kernel through a `&str` buffer, and the
/// ladder's `EACCES` names the probe rather than failing later.
pub fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() || meta.permissions().mode() & 0o111 == 0 {
        return false;
    }
    let Some(text) = path.to_str() else {
        return false;
    };
    let Some(name) = sys::CBuf::new(text) else {
        return false;
    };
    sys::faccessat(sys::AT_FDCWD as i64, &name, sys::X_OK, 0).is_ok()
}

/// `memfd_create(2)` with [`MFD_CLOEXEC`] always set.
///
/// When `allow_sealing` is set the call first tries with
/// [`MFD_ALLOW_SEALING`]; a kernel without sealing answers `EINVAL`, and the
/// call is retried on the unconditional base rather than failed. The image
/// still runs, it just stays unsealed.
///
/// # Safety
///
/// Issues a raw syscall, like every other `podbox_probe::sys` caller.
pub unsafe fn memfd_create(name: &sys::CBuf, allow_sealing: bool) -> Sysres {
    let flags = MFD_CLOEXEC | if allow_sealing { MFD_ALLOW_SEALING } else { 0 };
    match unsafe { sys::sys(sys::SYS_MEMFD_CREATE, [name.ptr(), flags, 0, 0, 0, 0]) } {
        Err(e) if allow_sealing && e == sys::EINVAL => unsafe {
            sys::sys(sys::SYS_MEMFD_CREATE, [name.ptr(), MFD_CLOEXEC, 0, 0, 0, 0])
        },
        answered => answered,
    }
}

/// Whether the kernel takes `memfd_create`: create one and close it.
///
/// The ladder asks this before promising the memfd rung, so a forced mode
/// refuses with the kernel as the reason rather than failing mid-launch.
pub fn kernel_takes_memfd() -> bool {
    let Some(name) = sys::CBuf::new("podbox-probe") else {
        return false;
    };
    match unsafe { memfd_create(&name, false) } {
        Ok(fd) => {
            let _ = sys::close(fd);
            true
        }
        Err(_) => false,
    }
}

/// Write `bytes` to a new memfd, sealed where the kernel accepts it.
///
/// The caller hands the descriptor to the ladder entry and closes its own
/// copy past the fork (see `run_ladder`): a parent copy held past that is a
/// leak, and the child execs from its own.
pub fn stage(bytes: &[u8]) -> crate::Result<i64> {
    let name = sys::CBuf::new("podbox-payload").expect("a literal carries no NUL");
    let fd = unsafe { memfd_create(&name, true) }.map_err(|e| {
        crate::Error::Runtime(format!(
            "memfd_create for the payload failed: {} ({})",
            e.name(),
            e.0
        ))
    })?;
    if let Err(e) = unsafe { write_full(fd, bytes) } {
        let _ = sys::close(fd);
        return Err(crate::Error::Runtime(format!(
            "writing the payload to its memfd failed: {} ({})",
            e.name(),
            e.0
        )));
    }
    match seal(fd) {
        Ok(_) => {}
        // A kernel without sealing answers EINVAL; the payload still runs, it
        // just stays unsealed, which is `memfd_create`'s own fallback shape.
        Err(e) if e == sys::EINVAL => {}
        Err(e) => {
            let _ = sys::close(fd);
            return Err(crate::Error::Runtime(format!(
                "sealing the payload memfd failed: {} ({})",
                e.name(),
                e.0
            )));
        }
    }
    Ok(fd)
}
/// Why bytes cannot go through fd-exec, naming what they take instead.
///
/// ⛔ Named rather than a boolean, for the ladder's refusal: a caller told only
/// "no" cannot say whether to exec by path, refuse a forced mode, or fall
/// through, and each of those is a different exit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemfdRefusal {
    /// A `#!` script fails through fd-exec and is exec'd by path instead.
    RoutePastScript,
    /// Not an ELF file at all, with why not.
    NotElf(String),
    /// Dynamically linked: `PT_INTERP` names the loader that must resolve it
    /// from a filesystem, which a memfd never puts in place.
    HasInterp(String),
}

impl std::fmt::Display for MemfdRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemfdRefusal::RoutePastScript => {
                write!(f, "a #! script routes past fd-exec and is exec'd by path")
            }
            MemfdRefusal::NotElf(why) => write!(f, "not an ELF file for fd-exec: {why}"),
            MemfdRefusal::HasInterp(interp) => write!(
                f,
                "dynamically linked against {interp}: a memfd exec never puts \
                 its libraries on disk, so the loader would fail on the first \
                 one it cannot find"
            ),
        }
    }
}

/// Whether `bytes` may be exec'd from a memfd: a static-PIE with no
/// `PT_INTERP`, reached through the program-header walk in [`crate::abi`].
///
/// ⚠ The architecture is NOT decided here: whether this machine executes the
/// payload's `e_machine` is [`crate::binfmt`]'s question (T-0506), asked with
/// the file in hand rather than guessed from its bytes.
pub fn eligible(bytes: &[u8]) -> Result<(), MemfdRefusal> {
    if has_shebang(bytes) {
        return Err(MemfdRefusal::RoutePastScript);
    }
    match crate::abi::interp_of("payload", bytes) {
        Ok(None) => Ok(()),
        Ok(Some(interp)) => Err(MemfdRefusal::HasInterp(interp)),
        Err(e) => Err(MemfdRefusal::NotElf(format!("{e}"))),
    }
}

/// Write all of `bytes` to `fd`, looping short writes.
///
/// ⚠ The kernel may return short, so one call is not enough: a truncated
/// payload exec'd from the memfd is a corruption, not a short write.
///
/// # Safety
///
/// Issues raw syscalls, like every other `podbox_probe::sys` caller.
pub unsafe fn write_full(fd: i64, bytes: &[u8]) -> Sysres {
    let mut wrote = 0usize;
    while wrote < bytes.len() {
        match sys::write(fd, &bytes[wrote..]) {
            Ok(n) if n > 0 => wrote += n as usize,
            // ⚠ `EINTR` is not a failure: a signal delivered mid-write must not
            // read as the payload being unwritable.
            Err(e) if e == sys::EINTR => continue,
            // A zero write with bytes remaining is not a short write to loop
            // on: nothing progressed, so looping is a hang shaped as patience.
            Ok(_) => return Err(sys::Errno(5)),
            Err(e) => return Err(e),
        }
    }
    Ok(wrote as i64)
}

/// Seal a written memfd against further shrinking, growing and writing.
///
/// A kernel without sealing answers `EINVAL`; the ladder runs the payload
/// unsealed rather than refusing it, which is [`memfd_create`]'s own fallback
/// shape. Sealing is all four seals at once: a payload sealed against writing
/// but still growable is a bound, not a seal.
pub fn seal(fd: i64) -> Sysres {
    sys::fcntl(
        fd,
        sys::F_ADD_SEALS,
        sys::F_SEAL_SEAL | sys::F_SEAL_SHRINK | sys::F_SEAL_GROW | sys::F_SEAL_WRITE,
    )
}

/// Exec the memfd `fd` points at, without resolving a path.
///
/// This is `fexecve` through the number the crates name: `execveat` with the
/// empty path and `AT_EMPTY_PATH` (see [`sys::SYS_EXECVEAT`]). Like `execve`,
/// it returns only on failure, and the error it returns with is the ladder's
/// fallthrough decision.
///
/// ⛔ `empty` is built BEFORE the fork and handed in, because between `clone`
/// and `execve` only async-signal-safe work is permitted and building it
/// allocates.
///
/// # Safety
///
/// `argv` and `envp` must be NUL-terminated arrays of valid C string pointers.
pub unsafe fn exec_fd(
    fd: i64,
    empty: &sys::CBuf,
    argv: *const *const u8,
    envp: *const *const u8,
) -> Sysres {
    unsafe { sys::execveat(fd, empty, argv, envp, sys::AT_EMPTY_PATH) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloexec_is_the_unconditional_base() {
        assert_eq!(MFD_CLOEXEC, 0x0001, "linux/memfd.h MFD_CLOEXEC");
        assert_eq!(MFD_ALLOW_SEALING, 0x0002, "linux/memfd.h MFD_ALLOW_SEALING");
    }

    #[test]
    fn scripts_route_past_fd_exec() {
        assert!(has_shebang(b"#!/bin/sh\necho hi\n"));
        assert!(!has_shebang(b"\x7fELF\x02\x01\x01\x00"));
        assert!(!has_shebang(b""));
        assert!(!has_shebang(b"# not a shebang\n"));
    }

    #[test]
    fn executable_probe_names_what_can_exec() {
        assert!(is_executable(Path::new("/bin/sh")));
        assert!(!is_executable(Path::new(
            "/nonexistent-podbox-probe-target"
        )));
    }

    #[test]
    fn memfd_round_trip_creates_a_usable_fd() {
        let name = sys::CBuf::new("podbox-probe").expect("literal has no NUL");
        let fd = unsafe { memfd_create(&name, false) }.expect("memfd_create runs here");
        assert!(fd >= 0);
        // The flag survives on the descriptor, not just in the constant:
        // the fork's regression was a stub that never set it.
        let flags = sys::fcntl(fd, sys::F_GETFD, 0).expect("the created fd answers fcntl");
        assert_eq!(
            flags & sys::FD_CLOEXEC as i64,
            sys::FD_CLOEXEC as i64,
            "MFD_CLOEXEC is set on the created memfd"
        );
        sys::close(fd).expect("the created fd closes");
    }

    /// ⭐ A static-PIE with no `PT_INTERP` is eligible; anything with one is
    /// refused naming the loader, whatever its sections say.
    #[test]
    fn eligibility_is_the_program_headers_and_not_the_sections() {
        assert_eq!(eligible(&static_pie()), Ok(()));
        let Err(MemfdRefusal::HasInterp(interp)) = eligible(&dynamic_pie()) else {
            panic!("a PT_INTERP payload was not refused as one");
        };
        assert!(interp.contains("ld-linux"), "{interp}");
    }

    /// `#!` routes past fd-exec; garbage is a named refusal, not a silent no.
    #[test]
    fn scripts_route_past_and_garbage_is_named() {
        assert_eq!(
            eligible(b"#!/bin/sh\necho hi\n"),
            Err(MemfdRefusal::RoutePastScript)
        );
        assert!(matches!(eligible(b""), Err(MemfdRefusal::NotElf(_))));
        assert!(matches!(
            eligible(b"not elf at all"),
            Err(MemfdRefusal::NotElf(_))
        ));
    }

    /// Written bytes seal where accepted and run unsealed where not.
    #[test]
    fn written_bytes_seal_where_accepted() {
        let name = sys::CBuf::new("podbox-seal").expect("literal has no NUL");
        let fd = unsafe { memfd_create(&name, true) }.expect("memfd_create runs here");
        let bytes = b"\x7fELF podbox seal probe";
        let n = unsafe { write_full(fd, bytes) }.expect("write_full writes all");
        assert_eq!(n as usize, bytes.len());
        match seal(fd) {
            Ok(_) => {}
            // A kernel without sealing answers EINVAL; the payload still runs,
            // it just stays unsealed. Anything else is a real failure.
            Err(e) => assert_eq!(e, sys::EINVAL, "sealing failed with {e:?}"),
        }
        sys::close(fd).expect("the created fd closes");
    }

    /// `stage` hands back a live descriptor with close-on-exec set: the
    /// staged memfd must not leak across a later `execve`. The exec half is
    /// the lane's Prove; this pins the create, write and seal path runs.
    #[test]
    fn staging_hands_back_a_live_cloexec_descriptor() {
        assert!(kernel_takes_memfd(), "memfd_create runs here");
        let fd = stage(&static_pie()).expect("staging runs here");
        assert!(fd >= 0);
        let flags = sys::fcntl(fd, sys::F_GETFD, 0).expect("the staged fd answers fcntl");
        assert_eq!(
            flags & sys::FD_CLOEXEC as i64,
            sys::FD_CLOEXEC as i64,
            "MFD_CLOEXEC is set on the staged memfd"
        );
        sys::close(fd).expect("the staged fd closes");
    }

    /// A 64-bit header with one `PT_LOAD` and no `PT_INTERP`: the smallest
    /// bytes `eligible` must accept.
    fn static_pie() -> Vec<u8> {
        let mut b = header(1);
        // `PT_LOAD`, at file offset 120, with no strings attached.
        b[64..68].copy_from_slice(&1u32.to_le_bytes());
        b
    }

    /// The same with one `PT_INTERP` naming a loader: `eligible` must refuse
    /// it naming that loader.
    fn dynamic_pie() -> Vec<u8> {
        let mut b = header(2);
        let interp = b"/lib64/ld-linux-x86-64.so.2\0";
        // First program header: `PT_LOAD`. Second: `PT_INTERP`, whose 64-bit
        // `p_offset` sits at phdr+8 and points past both headers.
        b[64..68].copy_from_slice(&1u32.to_le_bytes());
        b[120..124].copy_from_slice(&3u32.to_le_bytes());
        b[128..136].copy_from_slice(&176u64.to_le_bytes());
        b.extend_from_slice(interp);
        b
    }

    /// A 64-bit little-endian header with `e_phoff = 64`, `e_phentsize = 56`
    /// and room for `phnum` program headers.
    fn header(phnum: u16) -> Vec<u8> {
        let mut b = vec![0u8; 64 + 56 * phnum as usize];
        b[0..4].copy_from_slice(b"\x7fELF");
        b[4] = 2;
        b[5] = 1;
        b[32..40].copy_from_slice(&64u64.to_le_bytes());
        b[54..56].copy_from_slice(&56u16.to_le_bytes());
        b[56..58].copy_from_slice(&phnum.to_le_bytes());
        b
    }
}
