//! The path calls that the audit of TODO/interpose.md T-1754 found
//! imported by the programs of common images (node, python, dpkg, tar,
//! coreutils, apt, git, busybox) and not exported here: each reached the
//! host unmapped. `lutimes` failed each package of dpkg (T-1755);
//! `scandir64` made node list the host's /etc (T-1754).

use super::*;

crate::real!(pub fn next_scandir64 = "scandir64"(*const c_char, *mut *mut c_void, Filter, Compar) -> c_int);
crate::real!(pub fn next_lutimes = "lutimes"(*const c_char, *const c_void) -> c_int);
crate::real!(pub fn next_futimesat = "futimesat"(c_int, *const c_char, *const c_void) -> c_int);
crate::real!(pub fn next_lchmod = "lchmod"(*const c_char, c_uint) -> c_int);
crate::real!(pub fn next_mkfifo = "mkfifo"(*const c_char, c_uint) -> c_int);
crate::real!(pub fn next_mkfifoat = "mkfifoat"(c_int, *const c_char, c_uint) -> c_int);
crate::real!(pub fn next_mknodat = "mknodat"(c_int, *const c_char, c_uint, u64) -> c_int);
crate::real!(pub fn next_pathconf = "pathconf"(*const c_char, c_int) -> isize);
crate::real!(pub fn next_statfs64 = "statfs64"(*const c_char, *mut c_void) -> c_int);
crate::real!(pub fn next_statvfs64 = "statvfs64"(*const c_char, *mut c_void) -> c_int);
crate::real!(pub fn next_truncate64 = "truncate64"(*const c_char, i64) -> c_int);
crate::real!(pub fn next_inotify_add_watch = "inotify_add_watch"(c_int, *const c_char, u32) -> c_int);

path_nonneg!(scandir64, next_scandir64, (dir: *const c_char, list: *mut *mut c_void, filter: Filter, compar: Compar), dir, c_int);
path_int!(lutimes, next_lutimes, (path: *const c_char, times: *const c_void), path);
path_at_int!(futimesat, next_futimesat, (dirfd: c_int, path: *const c_char, times: *const c_void), dirfd, path);
path_int!(lchmod, next_lchmod, (path: *const c_char, mode: c_uint), path);
path_int!(mkfifo, next_mkfifo, (path: *const c_char, mode: c_uint), path);
path_at_int!(mkfifoat, next_mkfifoat, (dirfd: c_int, path: *const c_char, mode: c_uint), dirfd, path);
path_at_int!(mknodat, next_mknodat, (dirfd: c_int, path: *const c_char, mode: c_uint, dev: u64), dirfd, path);
path_nonneg!(pathconf, next_pathconf, (path: *const c_char, name: c_int), path, isize);
path_int!(statfs64, next_statfs64, (path: *const c_char, buf: *mut c_void), path);
path_int!(statvfs64, next_statvfs64, (path: *const c_char, buf: *mut c_void), path);
path_int!(truncate64, next_truncate64, (path: *const c_char, len: i64), path);

/// `inotify_add_watch(fd, path, mask)`: the watched path through the table
/// (node's `fs.watch`, `tail -F`).
///
/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int {
    let Some(f) = next_inotify_add_watch() else {
        set_errno(EINVAL);
        return -1;
    };
    let mut buf = [0u8; crate::map::OUT];
    let p =
        unsafe { crate::map::prepare(b"inotify_add_watch", crate::map::AT_FDCWD, path, &mut buf) };
    if p.is_null() {
        return -1;
    }
    unsafe { f(fd, p, mask) }
}
