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
