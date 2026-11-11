use super::*;

/// `statx(2)`, and it is the entry point a modern `stat(1)` actually calls.
///
/// # Safety
/// The payload's own contract for `statx(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn statx(
    dirfd: c_int,
    path: *const c_char,
    flags: c_int,
    mask: c_uint,
    st: *mut c_void,
) -> c_int {
    let Some(f) = next_statx() else {
        unsafe { *__errno_location() = EINVAL };
        return -1;
    };
    // ⭐ T-1409, as in the `stat` shapes: the guest path is absolute, so the
    // call goes with `AT_FDCWD` where the override serves it.
    if unsafe { crate::procfs::exe_override_hit(path) } {
        let mut gout = [0u8; crate::map::OUT];
        if let Some(gp) = unsafe { stat_exe_path(&mut gout) } {
            let rc = unsafe {
                f(
                    crate::map::AT_FDCWD,
                    gp,
                    flags,
                    mask | STATX_UID | STATX_GID | 0x0000_0100,
                    st,
                )
            };
            if rc == 0 {
                unsafe { report_statx(st) };
            }
            return rc;
        }
    }
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(b"statx", dirfd, path, &mut buf) };
    if p.is_null() {
        return -1;
    }
    let (guest_fd, guest) = (dirfd, path);
    let (dirfd, path) = if p == path {
        (dirfd, path)
    } else {
        (crate::map::AT_FDCWD, p)
    };
    // ⛔ The caller's mask is widened to include the identity fields. A caller
    // that did not ask for `STATX_UID` gets it anyway, which `statx(2)` permits
    // -- the kernel may return more than was asked for -- and without it podvm
    // would have no uid to overwrite and no `stx_ino` to key the memo on.
    let wide = mask | STATX_UID | STATX_GID | 0x0000_0100;
    let mut rc = unsafe { f(dirfd, path, flags, wide, st) };
    // T-1646: a miss that follows the last link tries the rootfs walk once.
    let mut walked = [0u8; crate::map::OUT];
    if rc != 0
        && flags & 0x100 == 0
        && let Some(q) = unsafe { crate::exec::lookup::again(guest_fd, guest, p, &mut walked) }
    {
        rc = unsafe { f(crate::map::AT_FDCWD, q, flags, wide, st) };
    }
    if rc == 0 {
        unsafe { report_statx(st) };
    }
    rc
}

// ------------------------------------------- T-0703: the path entry points
//
// ⭐ One macro per return shape, so the macro-covered entry points cannot
// drift apart, which is `references/VHSgunzo__pathmap/tree/path-mapping.c:250-262`'s
// own decision. Every one funnels its path through [`map::prepare`] and calls
// the real function with what comes back. `$path` is the argument the rewrite
// applies to.
//
// ⚠ Functions taking only descriptors stay out: `fstat`, `fchdir`,
// `fexecve`, `fchmod` and their kin carry nothing to rewrite, which is
// `experiments/100-interpose-symbols.sh`'s own rule for the same absence.
//
// ⚠ What these macros deliberately do NOT do: save and restore `errno` around
// the real call. Linux leaves `errno` alone on success, and every internal
// step that can fail returns before the real call runs, so there is nothing
// to restore on the success path and the failure path carries the real
// call's own number.

macro_rules! path_int {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* ), $path:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            let Some(f) = $real() else { set_errno(EINVAL); return -1; };
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path,
                    &mut buf,
                )
            };
            if p.is_null() {
                return -1;
            }
            let $path = p;
            unsafe { f($($arg),*) }
        }
    };
}

macro_rules! path_nonneg {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* ), $path:ident, $ret:ty) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> $ret {
            let fail: $ret = -1 as $ret;
            let Some(f) = $real() else { set_errno(EINVAL); return fail; };
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path,
                    &mut buf,
                )
            };
            if p.is_null() {
                return fail;
            }
            let $path = p;
            unsafe { f($($arg),*) }
        }
    };
}

macro_rules! path_ptr {
    // TODO/cli.md T-1655: `fopen` and `freopen` write where their mode
    // does, so the read-only layer reads the mode.
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* ), $path:ident, mode = $mode:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> *mut c_void {
            let Some(f) = $real() else { set_errno(EINVAL); return core::ptr::null_mut(); };
            let mut buf = [0u8; crate::map::OUT];
            let writes = unsafe { crate::map::mode_writes($mode) };
            let p = unsafe {
                crate::map::prepare_mode(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path,
                    writes,
                    &mut buf,
                )
            };
            if p.is_null() {
                return core::ptr::null_mut();
            }
            let $path = p;
            unsafe { f($($arg),*) }
        }
    };
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* ), $path:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> *mut c_void {
            let Some(f) = $real() else { set_errno(EINVAL); return core::ptr::null_mut(); };
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path,
                    &mut buf,
                )
            };
            if p.is_null() {
                return core::ptr::null_mut();
            }
            let $path = p;
            unsafe { f($($arg),*) }
        }
    };
}

macro_rules! path_at_int {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* ), $dirfd:ident, $path:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            let Some(f) = $real() else { set_errno(EINVAL); return -1; };
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare(stringify!($name).as_bytes(), $dirfd, $path, &mut buf)
            };
            if p.is_null() {
                return -1;
            }
            let $path = p;
            unsafe { f($($arg),*) }
        }
    };
}

/// The `access` shapes, T-0703 and T-1646: a miss tries the rootfs walk
/// once, because a shell tests a command with `access` before its exec.
macro_rules! access_int {
    ($name:ident, $real:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(path: *const c_char, mode: c_int) -> c_int {
            let call = |p: *const c_char| $real().map(|f| unsafe { f(p, mode) });
            unsafe {
                faccess(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    path,
                    0,
                    call,
                )
            }
        }
    };
}

/// The shared body of the `access` shapes. `call` is the real call with
/// the path to try, or `None` where the libc has no such symbol.
unsafe fn faccess(
    what: &[u8],
    dirfd: c_int,
    path: *const c_char,
    flags: c_int,
    call: impl Fn(*const c_char) -> Option<c_int>,
) -> c_int {
    let mut buf = [0u8; crate::map::OUT];
    let p = unsafe { crate::map::prepare(what, dirfd, path, &mut buf) };
    if p.is_null() {
        return -1;
    }
    let Some(mut rc) = call(p) else {
        set_errno(EINVAL);
        return -1;
    };
    let mut walked = [0u8; crate::map::OUT];
    if rc != 0
        && flags & 0x100 == 0
        && let Some(q) = unsafe { crate::exec::lookup::again(dirfd, path, p, &mut walked) }
    {
        rc = call(q).unwrap_or(-1);
    }
    rc
}

/// # Safety
/// The payload's own contract for `faccessat(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn faccessat(
    dirfd: c_int,
    path: *const c_char,
    mode: c_int,
    flags: c_int,
) -> c_int {
    // Where the rewrite moved the path the call goes with an absolute one.
    let call = |p: *const c_char| {
        let fd = if p == path {
            dirfd
        } else {
            crate::map::AT_FDCWD
        };
        next_faccessat().map(|f| unsafe { f(fd, p, mode, flags) })
    };
    unsafe { faccess(b"faccessat", dirfd, path, flags, call) }
}

path_int!(chdir, next_chdir, (path: *const c_char), path);
path_int!(unlink, next_unlink, (path: *const c_char), path);
path_int!(rmdir, next_rmdir, (path: *const c_char), path);
path_int!(remove, next_remove, (path: *const c_char), path);
path_int!(mkdir, next_mkdir, (path: *const c_char, mode: c_uint), path);
path_int!(chmod, next_chmod, (path: *const c_char, mode: c_uint), path);
path_int!(truncate, next_truncate, (path: *const c_char, len: i64), path);
path_int!(utime, next_utime, (path: *const c_char, times: *const c_void), path);
path_int!(utimes, next_utimes, (path: *const c_char, times: *const c_void), path);
access_int!(access, next_access);
access_int!(eaccess, next_eaccess);
access_int!(euidaccess, next_euidaccess);
path_int!(statfs, next_statfs, (path: *const c_char, buf: *mut c_void), path);
path_int!(statvfs, next_statvfs, (path: *const c_char, buf: *mut c_void), path);
path_int!(setxattr, next_setxattr, (path: *const c_char, name: *const c_char, value: *const c_void, size: usize, flags: c_int), path);
path_int!(lsetxattr, next_lsetxattr, (path: *const c_char, name: *const c_char, value: *const c_void, size: usize, flags: c_int), path);
path_int!(removexattr, next_removexattr, (path: *const c_char, name: *const c_char), path);
path_int!(lremovexattr, next_lremovexattr, (path: *const c_char, name: *const c_char), path);
path_int!(glob, next_glob, (pattern: *const c_char, flags: c_int, errfunc: ErrFunc, pglob: *mut c_void), pattern);
path_int!(glob64, next_glob64, (pattern: *const c_char, flags: c_int, errfunc: ErrFunc, pglob: *mut c_void), pattern);
path_int!(ftw, next_ftw, (path: *const c_char, func: FtwFunc, nfds: c_int), path);
path_int!(ftw64, next_ftw64, (path: *const c_char, func: FtwFunc, nfds: c_int), path);
path_int!(nftw, next_nftw, (path: *const c_char, func: NftwFunc, nfds: c_int, flags: c_int), path);
path_int!(nftw64, next_nftw64, (path: *const c_char, func: NftwFunc, nfds: c_int, flags: c_int), path);

path_nonneg!(creat, next_creat, (path: *const c_char, mode: c_uint), path, c_int);
path_nonneg!(creat64, next_creat64, (path: *const c_char, mode: c_uint), path, c_int);
path_nonneg!(scandir, next_scandir, (dir: *const c_char, list: *mut *mut c_void, filter: Filter, compar: Compar), dir, c_int);
path_nonneg!(getxattr, next_getxattr, (path: *const c_char, name: *const c_char, value: *mut c_void, size: usize), path, isize);
path_nonneg!(lgetxattr, next_lgetxattr, (path: *const c_char, name: *const c_char, value: *mut c_void, size: usize), path, isize);
path_nonneg!(listxattr, next_listxattr, (path: *const c_char, list: *mut c_char, size: usize), path, isize);
path_nonneg!(llistxattr, next_llistxattr, (path: *const c_char, list: *mut c_char, size: usize), path, isize);

path_ptr!(opendir, next_opendir, (path: *const c_char), path);
path_ptr!(fopen, next_fopen, (path: *const c_char, mode: *const c_char), path, mode = mode);
path_ptr!(fopen64, next_fopen64, (path: *const c_char, mode: *const c_char), path, mode = mode);
path_ptr!(freopen, next_freopen, (path: *const c_char, mode: *const c_char, stream: *mut c_void), path, mode = mode);
path_ptr!(freopen64, next_freopen64, (path: *const c_char, mode: *const c_char, stream: *mut c_void), path, mode = mode);

path_at_int!(mkdirat, next_mkdirat, (dirfd: c_int, path: *const c_char, mode: c_uint), dirfd, path);
path_at_int!(unlinkat, next_unlinkat, (dirfd: c_int, path: *const c_char, flags: c_int), dirfd, path);
path_at_int!(fchmodat, next_fchmodat, (dirfd: c_int, path: *const c_char, mode: c_uint, flags: c_int), dirfd, path);
path_at_int!(utimensat, next_utimensat, (dirfd: c_int, path: *const c_char, times: *const c_void, flags: c_int), dirfd, path);
path_at_int!(openat2, next_openat2, (dirfd: c_int, path: *const c_char, how: *const c_void, size: usize), dirfd, path);

// ------------------------------------------- results read back in virtual names, T-0705
//
// The forward map rewrites what goes into the kernel. These six read the
// answer back out: where the real result lies under a TO prefix it is
// rewritten to the virtual FROM, so a payload comparing what it wrote
// with what it reads sees one tree. Unmatched answers travel untouched.
// Too long answers `ERANGE` rather than a truncation or a leak, which is
// the entry's ruled Decision. `readdir`'s `d_name` takes no reversal: a
// bare name carries no prefix, so there is nothing to match it against.

/// Copy `n` bytes at `src` plus a NUL into the caller's `size`-byte buffer.
///
/// Answers `ERANGE` where the buffer is too small, which is the real
/// call's own answer there and T-0705's ruled answer for a virtual name
/// that does not fit.
///
/// # Safety
/// `src` must be readable for `n` bytes, `dst` writable for `size` bytes.
pub(super) unsafe fn copy_fit(
    src: *const u8,
    n: usize,
    dst: *mut c_char,
    size: usize,
) -> *mut c_char {
    if n + 1 > size {
        set_errno(ERANGE);
        return core::ptr::null_mut();
    }
    unsafe {
        core::ptr::copy_nonoverlapping(src, dst as *mut u8, n);
        *(dst as *mut u8).add(n) = 0;
    }
    dst
}

/// Copy `n` bytes at `p` into freshly allocated memory.
///
/// The allocating result shapes must hand back memory the caller frees,
/// so a reversed name needs its own allocation from the real `malloc`.
/// Returns null where `malloc` fails, with `errno` carrying its number.
///
/// # Safety
/// `p` must be readable for `n` bytes.
unsafe fn malloc_copy(p: *const u8, n: usize) -> *mut c_void {
    let Some(malloc) = next_malloc() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    let q = unsafe { malloc(n + 1) };
    if q.is_null() {
        return q;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(p, q as *mut u8, n);
        *(q as *mut u8).add(n) = 0;
    }
    q
}

/// Free memory the real call allocated, where the allocator answers.
///
/// # Safety
/// `p` must be memory the real call allocated, or null.
unsafe fn free_ptr(p: *mut c_void) {
    if p.is_null() {
        return;
    }
    if let Some(free) = next_free() {
        unsafe { free(p) };
    }
}

/// Reverse an allocated real path into an allocated virtual one.
///
/// Frees `real` on every path out. Unmatched answers the real pointer
/// itself: nothing virtual covers it, so the real path is the truth, not
/// a leak. Too long, or a real path at the buffer ceiling, answers
/// `ERANGE` rather than a truncation or a leak.
///
/// # Safety
/// `real` must be a non-null NUL-terminated string from the real call.
pub(super) unsafe fn reverse_alloc(real: *mut c_void) -> *mut c_char {
    let real_n = crate::map::strnlen(real as *const c_char, crate::map::OUT);
    if real_n >= crate::map::OUT {
        unsafe { free_ptr(real) };
        set_errno(ERANGE);
        return core::ptr::null_mut();
    }
    let t = unsafe { crate::map::table() };
    let mut vbuf = [0u8; crate::map::OUT];
    let v = unsafe { crate::map::unrewrite(&t, real as *const u8, real_n, &mut vbuf) };
    if v == -2 {
        unsafe { free_ptr(real) };
        set_errno(ERANGE);
        return core::ptr::null_mut();
    }
    if v < 0 {
        return real as *mut c_char;
    }
    let q = unsafe { malloc_copy(vbuf.as_ptr(), v as usize) };
    unsafe { free_ptr(real) };
    q as *mut c_char
}

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getcwd(buf: *mut c_char, size: usize) -> *mut c_char {
    let Some(f) = crate::map::next_getcwd() else {
        set_errno(EINVAL);
        return core::ptr::null_mut();
    };
    // ⭐ Empty table, empty wrapper: with nothing to reverse against the
    // call goes straight through, so unmapped runs keep byte-identical
    // behaviour (and no length ceiling below the real call's own).
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        return unsafe { f(buf, size) };
    }
    let mut tmp = [0u8; crate::map::OUT];
    let r = unsafe { f(tmp.as_mut_ptr() as *mut c_char, crate::map::OUT) };
    if r.is_null() {
        return core::ptr::null_mut();
    }
    let real_n = crate::map::strnlen(tmp.as_ptr() as *const c_char, crate::map::OUT);
    let mut vbuf = [0u8; crate::map::OUT];
    let v = unsafe { crate::map::unrewrite(&t, tmp.as_ptr(), real_n, &mut vbuf) };
    if buf.is_null() {
        // The glibc extension: allocate the answer.
        if v == -2 {
            set_errno(ERANGE);
            return core::ptr::null_mut();
        }
        if v < 0 {
            return unsafe { malloc_copy(tmp.as_ptr(), real_n) } as *mut c_char;
        }
        return unsafe { malloc_copy(vbuf.as_ptr(), v as usize) } as *mut c_char;
    }
    if v == -2 {
        set_errno(ERANGE);
        return core::ptr::null_mut();
    }
    // Unmatched answers the real path where it fits, exactly as the real
    // call would have; too small answers `ERANGE` on both sides.
    let (src, n) = if v < 0 {
        (tmp.as_ptr(), real_n)
    } else {
        (vbuf.as_ptr(), v as usize)
    };
    unsafe { copy_fit(src, n, buf, size) }
}
