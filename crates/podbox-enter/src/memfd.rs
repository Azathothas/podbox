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
}
