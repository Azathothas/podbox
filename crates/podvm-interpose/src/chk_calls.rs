//! The fortified entry points of the path calls. TODO/interpose.md T-1704.

use super::*;

// ⛔ T-1704. The fortified entry points that `_FORTIFY_SOURCE=2` emits for
// a caller whose buffer size the compiler knows. glibc's own definitions
// call its internal functions, past this object, so a fortified program
// read host paths (measured: `__realpath_chk`, `__readlink_chk`,
// `__readlinkat_chk` and `__getcwd_chk` each gave the host path where the
// plain call gave the guest path). Each keeps glibc's length check and then
// takes the same route as its plain call.
crate::real!(fn next_chk_fail = "__chk_fail"() -> ());

/// glibc's answer to a buffer shorter than the caller said: the process
/// ends. Where no `__chk_fail` resolves (musl has none), `abort`.
fn chk_fail() -> ! {
    if let Some(f) = next_chk_fail() {
        unsafe { f() };
    }
    unsafe extern "C" {
        fn abort() -> !;
    }
    unsafe { abort() }
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __realpath_chk(
    path: *const c_char,
    resolved: *mut c_char,
    resolvedlen: usize,
) -> *mut c_char {
    if resolvedlen < crate::map::OUT {
        chk_fail();
    }
    unsafe { crate::realpath(path, resolved) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __readlink_chk(
    path: *const c_char,
    buf: *mut c_char,
    len: usize,
    buflen: usize,
) -> isize {
    if len > buflen {
        chk_fail();
    }
    unsafe { crate::readlink(path, buf, len) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __readlinkat_chk(
    dirfd: c_int,
    path: *const c_char,
    buf: *mut c_char,
    len: usize,
    buflen: usize,
) -> isize {
    if len > buflen {
        chk_fail();
    }
    unsafe { crate::readlinkat(dirfd, path, buf, len) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __getcwd_chk(buf: *mut c_char, size: usize, buflen: usize) -> *mut c_char {
    if size > buflen {
        chk_fail();
    }
    unsafe { crate::getcwd(buf, size) }
}

// ⛔ T-1743. The fortified opens. `_FORTIFY_SOURCE` makes the inline `open`
// and `openat` of glibc call `__open_2` or `__openat_2` where the flags are
// not a constant and the caller gives no mode. glibc's own definitions then
// call its internal `open`, past this object, so the path reached the host
// unmapped. GNU tar 1.35 of Ubuntu 24.04 opens the directory of `-C` so:
// an absolute `-C /tmp/d` opened the host's `/tmp/d`, and a relative `-C d`
// worked only because the kernel resolved it from the real working
// directory (PodUtils/podcrates#102). Each keeps glibc's check of the mode
// and then takes the same route as its plain call.

#[cfg(target_arch = "aarch64")]
const O_DIRECTORY: c_int = 0o40000;
#[cfg(not(target_arch = "aarch64"))]
const O_DIRECTORY: c_int = 0o200000;
const O_CREAT: c_int = 0o100;
const O_TMPFILE: c_int = 0o20000000 | O_DIRECTORY;

/// glibc's `__OPEN_NEEDS_MODE`: `O_CREAT`, or each bit of `O_TMPFILE`
/// (which holds `O_DIRECTORY`). A fortified open with such flags has no mode
/// to give, and glibc ends the process.
pub(crate) fn needs_mode(flags: c_int) -> bool {
    flags & O_CREAT != 0 || flags & O_TMPFILE == O_TMPFILE
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __open_2(path: *const c_char, flags: c_int) -> c_int {
    if needs_mode(flags) {
        chk_fail();
    }
    unsafe { crate::open(path, flags, 0) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __open64_2(path: *const c_char, flags: c_int) -> c_int {
    if needs_mode(flags) {
        chk_fail();
    }
    unsafe { crate::open64(path, flags, 0) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __openat_2(dirfd: c_int, path: *const c_char, flags: c_int) -> c_int {
    if needs_mode(flags) {
        chk_fail();
    }
    unsafe { crate::openat(dirfd, path, flags, 0) }
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __openat64_2(dirfd: c_int, path: *const c_char, flags: c_int) -> c_int {
    if needs_mode(flags) {
        chk_fail();
    }
    unsafe { crate::openat64(dirfd, path, flags, 0) }
}
