//! The three things the standard library does not answer, declared directly.
//!
//! ⛔ There is no crate here on purpose. A launcher that pulls a general libc
//! binding in to ask three questions has taken a dependency this project would
//! then own, and the target links a libc anyway.
//!
//! SPDX-License-Identifier: 0BSD

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

extern "C" {
    fn access(path: *const i8, mode: i32) -> i32;
    fn geteuid() -> u32;
    fn getuid() -> u32;
    fn memfd_create(name: *const i8, flags: u32) -> i32;
}

/// W_OK, from `unistd.h`. It is 2 on every Linux ABI.
const W_OK: i32 = 2;

/// Whether the caller may write to a path.
///
/// ⚠ This asks the kernel with the REAL uid rather than reasoning from the
/// mode bits. A read-only mount, a container's user namespace and a POSIX ACL
/// all make the mode bits the wrong answer, and the launcher uses this to
/// decide whether it may write into the bundle - which is exactly the case
/// where the bundle is a read-only image.
pub fn writable(path: &Path) -> bool {
    let Ok(c) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    unsafe { access(c.as_ptr() as *const i8, W_OK) == 0 }
}

/// The effective user id.
pub fn euid() -> u32 {
    unsafe { geteuid() }
}

/// The real user id, which is what a bundle reports to the payload.
pub fn uid() -> u32 {
    unsafe { getuid() }
}

/// Whether a path is a file somebody may execute.
pub fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(md) => md.is_file() && md.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

/// An anonymous file, open, with no name in any directory.
///
/// ⭐ This is how the launcher hands a long option list to a program that
/// wants it on a file descriptor. The obvious route is a pipe, and a pipe is
/// wrong here: nothing reads the far end until the program has started, so a
/// list longer than the pipe buffer blocks the launcher forever, on exactly
/// the payloads whose option lists are long enough to be passed this way.
///
/// ⚠ Deliberately WITHOUT close-on-exec: the whole point is that the number
/// survives into the program being started.
pub fn anon_file(name: &str) -> Option<std::fs::File> {
    use std::os::unix::io::FromRawFd;
    let c = CString::new(name).ok()?;
    let fd = unsafe { memfd_create(c.as_ptr() as *const i8, 0) };
    if fd < 0 {
        return None;
    }
    Some(unsafe { std::fs::File::from_raw_fd(fd) })
}

/// The same thing, for a kernel with no anonymous files.
///
/// ⛔ The file is UNLINKED as soon as it is open, so it has no name for
/// anything else to reach, and the directory it is made in is one
/// `loaderdir::ensure` has already established is this user's alone. Without
/// both halves this would be the world-writable temporary file the third
/// improvement exists to remove.
pub fn scratch_file(dir: &Path, name: &str) -> Option<std::fs::File> {
    let at = dir.join(format!(".{name}.{}", std::process::id()));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&at)
        .ok()?;
    let _ = std::fs::remove_file(&at);
    Some(f)
}
