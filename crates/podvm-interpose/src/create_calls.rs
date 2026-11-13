use super::*;

/// # Safety
/// The payload's own contract for this entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unshare(flags: c_int) -> c_int {
    // A flag-less call succeeds genuinely: forward it rather than counting
    // a no-op as an emulation.
    if flags == 0 {
        let Some(f) = next_unshare() else {
            set_errno(EINVAL);
            return -1;
        };
        return unsafe { f(flags) };
    }
    // No tally behind it, no emulation.
    if crate::memo::memo_fd().is_none() {
        let Some(f) = next_unshare() else {
            set_errno(EINVAL);
            return -1;
        };
        return unsafe { f(flags) };
    }
    if (flags as u32) & crate::emulate::CLONE_NEWNET != 0 {
        say_newnet(b"unshare", flags as u32);
    }
    if crate::emulate::tally(crate::emulate::OP_UNSHARE, flags as u32, 0, 0) {
        return 0;
    }
    // The tally did not land: the real call and whatever it answers,
    // rather than an uncounted success.
    let Some(f) = next_unshare() else {
        set_errno(EINVAL);
        return -1;
    };
    unsafe { f(flags) }
}

/// # Safety
/// The payload's own contract for this entry point: seven registers, so a
/// caller passing four leaves garbage the callee never reads, and a caller
/// passing seven has every one forwarded. Stable Rust cannot declare the
/// C-variadic shape, so this is the fixed-arity equivalent (the `open`
/// family in this file is the precedent).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn clone(
    func: *mut c_void,
    stack: *mut c_void,
    flags: c_int,
    arg: *mut c_void,
    ptid: *mut c_void,
    tls: *mut c_void,
    ctid: *mut c_void,
) -> c_int {
    let Some(f) = next_clone() else {
        set_errno(EINVAL);
        return -1;
    };
    let stripped = crate::emulate::strip_ns(flags as u32) as c_int;
    // Nothing of ours in the flags: forward exactly, count nothing. This
    // is the path every thread creation takes.
    if stripped == flags {
        return unsafe { f(func, stack, flags, arg, ptid, tls, ctid) };
    }
    // No tally behind it, no emulation: forward the caller's own flags.
    if crate::memo::memo_fd().is_none() {
        return unsafe { f(func, stack, flags, arg, ptid, tls, ctid) };
    }
    if (flags as u32) & crate::emulate::CLONE_NEWNET != 0 {
        say_newnet(b"clone", flags as u32);
    }
    if crate::emulate::tally(crate::emulate::OP_CLONE, flags as u32, stripped as u32, 0) {
        return unsafe { f(func, stack, stripped, arg, ptid, tls, ctid) };
    }
    // The tally did not land: the caller's own flags go through, so
    // whatever the kernel answers is genuine rather than uncounted.
    unsafe { f(func, stack, flags, arg, ptid, tls, ctid) }
}

// ------------------------------------------------- shapes no macro covers

pub(super) const ERANGE: c_int = 34;
pub(super) const ENAMETOOLONG: c_int = 36;

/// `open` with a fixed third argument rather than `...`.
///
/// ⭐ On x86_64 SysV the first six integer arguments travel in registers, so a
/// two-argument caller leaves whatever in the third register and a
/// three-argument one puts the mode there. Reading it only under
/// `O_CREAT|O_TMPFILE` is exact for both shapes, and stable Rust cannot
/// define C-variadic functions at all (rust-lang/rust#44930), so this is the
/// shape rather than a workaround.
macro_rules! open_fixed {
    ($name:ident, $real:ident, $path:ident, $flags:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[allow(invalid_runtime_symbol_definitions)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($path: *const c_char, $flags: c_int, mode: c_uint) -> c_int {
            let Some(f) = $real() else {
                set_errno(EINVAL);
                return -1;
            };
            // ⭐ T-1409, as in `readlink`: the guest exe opens even where the
            // real call succeeds, gated on the CLI-set flag.
            if unsafe { crate::procfs::exe_override_hit($path) } {
                if let Some(e) = unsafe { proc_open_exe($flags, mode) } {
                    return e;
                }
            }
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare_follow(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path,
                    crate::map::open_writes($flags),
                    // T-1725: O_NOFOLLOW, and O_CREAT|O_EXCL, do not follow the last link.
                    $flags & crate::oflags::O_NOFOLLOW == 0 && $flags & 0o300 != 0o300,
                    &mut buf,
                )
            };
            if p.is_null() {
                return -1;
            }
            let rc = unsafe { f(p, $flags, mode) };
            // T-0413: where no procfs is mounted the real call fails
            // `ENOENT`, and only there does the emulation answer. `p` is
            // the caller's own pointer exactly where the table left the
            // path untouched, so a deliberate mapping always wins.
            // T-0501 beside it: a `--device` guest path names no node in
            // the image, so the same `ENOENT` gate serves the duplicate
            // of the host descriptor the entry opened before the chroot.
            if rc < 0 && errno() == crate::procfs::ENOENT && p == $path {
                if let Some(e) = unsafe { proc_emulate_open($path, $flags, mode) } {
                    return e;
                }
                if let Some(e) = unsafe { crate::device::serve_open($path, $flags) } {
                    return e;
                }
            }
            rc
        }
    };
    ($name:ident, $real:ident, $dirfd:ident, $path:ident, $flags:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[allow(invalid_runtime_symbol_definitions)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            $dirfd: c_int,
            $path: *const c_char,
            $flags: c_int,
            mode: c_uint,
        ) -> c_int {
            let Some(f) = $real() else {
                set_errno(EINVAL);
                return -1;
            };
            // ⭐ T-1409, as in `readlink`: the guest exe opens even where the
            // real call succeeds, gated on the CLI-set flag.
            if unsafe { crate::procfs::exe_override_hit($path) } {
                if let Some(e) = unsafe { proc_open_exe($flags, mode) } {
                    return e;
                }
            }
            let mut buf = [0u8; crate::map::OUT];
            let p = unsafe {
                crate::map::prepare_follow(
                    stringify!($name).as_bytes(),
                    $dirfd,
                    $path,
                    crate::map::open_writes($flags),
                    // T-1725: O_NOFOLLOW, and O_CREAT|O_EXCL, do not follow the last link.
                    $flags & crate::oflags::O_NOFOLLOW == 0 && $flags & 0o300 != 0o300,
                    &mut buf,
                )
            };
            if p.is_null() {
                return -1;
            }
            // T-0413, as above. Relative paths never name the emulated
            // leaves (the matchers anchor on absolute spellings), so the
            // pointer comparison is the whole guard here too. T-0501's
            // device serve rides the same gate.
            let rc = unsafe { f($dirfd, p, $flags, mode) };
            if rc < 0 && p == $path && errno() == crate::procfs::ENOENT {
                if let Some(e) = unsafe { proc_emulate_open($path, $flags, mode) } {
                    return e;
                }
                if let Some(e) = unsafe { crate::device::serve_open($path, $flags) } {
                    return e;
                }
            }
            rc
        }
    };
}

open_fixed!(open, next_open, path, flags);
open_fixed!(open64, next_open64, path, flags);
open_fixed!(openat, next_openat, dirfd, path, flags);
open_fixed!(openat64, next_openat64, dirfd, path, flags);

/// Two paths in one call: both resolve, and either failing fails the call.
/// `linkat`, `renameat` and `renameat2` share the shape; only the names and
/// the trailing arguments differ.
macro_rules! path_dual {
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* ), $dirfd_a:ident, $path_a:ident, $dirfd_b:ident, $path_b:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            let Some(f) = $real() else { set_errno(EINVAL); return -1; };
            let mut bufa = [0u8; crate::map::OUT];
            let mut bufb = [0u8; crate::map::OUT];
            let pa = unsafe {
                crate::map::prepare(stringify!($name).as_bytes(), $dirfd_a, $path_a, &mut bufa)
            };
            let pb = unsafe {
                crate::map::prepare(stringify!($name).as_bytes(), $dirfd_b, $path_b, &mut bufb)
            };
            if pa.is_null() || pb.is_null() {
                return -1;
            }
            let $path_a = pa;
            let $path_b = pb;
            unsafe { f($($arg),*) }
        }
    };
    ($name:ident, $real:ident, ( $($arg:ident : $ty:ty),* ), $path_a:ident, $path_b:ident) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),*) -> c_int {
            let Some(f) = $real() else { set_errno(EINVAL); return -1; };
            let mut bufa = [0u8; crate::map::OUT];
            let mut bufb = [0u8; crate::map::OUT];
            let pa = unsafe {
                crate::map::prepare(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path_a,
                    &mut bufa,
                )
            };
            let pb = unsafe {
                crate::map::prepare(
                    stringify!($name).as_bytes(),
                    crate::map::AT_FDCWD,
                    $path_b,
                    &mut bufb,
                )
            };
            if pa.is_null() || pb.is_null() {
                return -1;
            }
            let $path_a = pa;
            let $path_b = pb;
            unsafe { f($($arg),*) }
        }
    };
}

path_dual!(link, next_link, (old: *const c_char, new: *const c_char), old, new);
path_dual!(rename, next_rename, (old: *const c_char, new: *const c_char), old, new);
path_dual!(linkat, next_linkat, (olddirfd: c_int, old: *const c_char, newdirfd: c_int, new: *const c_char, flags: c_int), olddirfd, old, newdirfd, new);
path_dual!(renameat, next_renameat, (olddirfd: c_int, old: *const c_char, newdirfd: c_int, new: *const c_char), olddirfd, old, newdirfd, new);
path_dual!(renameat2, next_renameat2, (olddirfd: c_int, old: *const c_char, newdirfd: c_int, new: *const c_char, flags: c_uint), olddirfd, old, newdirfd, new);

/// `symlink` maps the link path fully and the target literally.
///
/// ⭐ The target is link CONTENT, resolved when the link is used and relative
/// to the link's own directory, so absolutizing it now would answer about the
/// wrong directory. An absolute target matches the table as written; a
/// relative one matches only where it literally starts with a prefix. That is
/// `references/VHSgunzo__pathmap/tree/path-mapping.c:1019-1035`'s shape.
///
/// # Safety
/// The payload's own contract for `symlink(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn symlink(target: *const c_char, linkpath: *const c_char) -> c_int {
    let Some(f) = next_symlink() else {
        set_errno(EINVAL);
        return -1;
    };
    let mut linkb = [0u8; crate::map::OUT];
    let mut targb = [0u8; crate::map::OUT];
    let lp = unsafe { crate::map::prepare(b"symlink", crate::map::AT_FDCWD, linkpath, &mut linkb) };
    let tp = unsafe { crate::map::prepare_literal(b"symlink", target, &mut targb) };
    if lp.is_null() || tp.is_null() {
        return -1;
    }
    unsafe { f(tp, lp) }
}

/// # Safety
/// The payload's own contract for `symlinkat(2)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn symlinkat(
    target: *const c_char,
    newdirfd: c_int,
    linkpath: *const c_char,
) -> c_int {
    let Some(f) = next_symlinkat() else {
        set_errno(EINVAL);
        return -1;
    };
    let mut linkb = [0u8; crate::map::OUT];
    let mut targb = [0u8; crate::map::OUT];
    let lp = unsafe { crate::map::prepare(b"symlinkat", newdirfd, linkpath, &mut linkb) };
    let tp = unsafe { crate::map::prepare_literal(b"symlinkat", target, &mut targb) };
    if lp.is_null() || tp.is_null() {
        return -1;
    }
    unsafe { f(tp, newdirfd, lp) }
}

/// The length of the `X` run that `mk*temp` replaces. glibc and musl both
/// require exactly six `X`, at the end of the template or, for the `s`
/// forms, before the suffix.
const X_RUN: usize = 6;

/// Run one `mk*temp` call on a mapped copy of `template`, T-1699.
///
/// ⭐ The real call gets a mapped copy in `out`, so the file lands in the
/// rootfs. After a success, this writes back only the chosen `X` run into
/// the caller's template. The caller's buffer thus keeps a guest name of its
/// own length, and no host path goes back to the payload. This replaces the
/// T-0705 rule of 2026-09-22, which wrote the mapped name into the caller's
/// buffer and answered `ERANGE` when the mapped name was longer.
///
/// `suffix` is the suffix length of `mkstemps` and `mkostemps` (T-1701),
/// and 0 for the other forms. The `X` run sits just before the suffix.
///
/// ⛔ A negative suffix length, a suffix length past the template, or six
/// bytes before the suffix that are not all `X` answers `EINVAL`, as libc
/// does. The check runs on the guest template. The mapped copy must end in
/// the same run and suffix: normalisation can remove a trailing slash, and
/// then the copy names a different file, so that also answers `EINVAL`. A
/// mapped path past the stack buffer answers `ENAMETOOLONG`, never a
/// truncation.
///
/// `ok` tells a success of the real call from a failure, and `fail` is the
/// answer when the call does not run.
///
/// # Safety
/// `template` must be null or a writable NUL-terminated string, which is
/// the payload's own contract for `mk*temp`.
pub(super) unsafe fn on_template<R: Copy>(
    what: &[u8],
    template: *mut c_char,
    suffix: c_int,
    fail: R,
    ok: impl Fn(R) -> bool,
    call: impl FnOnce(*mut c_char) -> R,
) -> R {
    if template.is_null() {
        return call(template);
    }
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        return call(template);
    }
    let n = crate::map::strnlen(template, crate::map::OUT);
    if n >= crate::map::OUT {
        return call(template);
    }
    // Templates are relative to the working directory by construction.
    let mut abs = [0u8; crate::map::OUT];
    if unsafe { crate::map::absolutize(&t, crate::map::AT_FDCWD, template, &mut abs) } < 0 {
        crate::say::line(&[what, b": podvm cannot resolve the path, so the call fails"]);
        set_errno(2);
        return fail;
    }
    let abs_n = crate::map::strnlen(abs.as_ptr() as *const c_char, crate::map::OUT);
    // TODO/cli.md T-1655: a template under a read-only prefix makes no file.
    if unsafe { crate::map::denied(abs.as_ptr(), abs_n) } {
        set_errno(crate::map::EROFS);
        return fail;
    }
    let mut out = [0u8; crate::map::OUT];
    let m = match unsafe { crate::map::rewrite(&t, abs.as_ptr(), abs_n, &mut out) } {
        -1 => return call(template),
        -2 => {
            crate::say::line(&[what, b": the mapped path does not fit"]);
            set_errno(ENAMETOOLONG);
            return fail;
        }
        m => m as usize,
    };
    let guest = unsafe { core::slice::from_raw_parts_mut(template as *mut u8, n) };
    // The tail is the run and the suffix: its length, where it fits.
    let tail = match usize::try_from(suffix) {
        Ok(s) if n >= X_RUN && s <= n - X_RUN => s + X_RUN,
        _ => {
            set_errno(EINVAL);
            return fail;
        }
    };
    if m < tail
        || !x_run(&guest[n - tail..n - tail + X_RUN])
        || guest[n - tail..] != out[m - tail..m]
    {
        set_errno(EINVAL);
        return fail;
    }
    // `rewrite` writes no NUL, and `out` starts zeroed with `m < OUT`.
    let r = call(out.as_mut_ptr() as *mut c_char);
    if ok(r) {
        guest[n - tail..n - tail + X_RUN].copy_from_slice(&out[m - tail..m - tail + X_RUN]);
    }
    r
}

/// Are all bytes of `s` the template letter `X`?
fn x_run(s: &[u8]) -> bool {
    s.iter().all(|&b| b == b'X')
}

/// One `mk*stemp` entry point that answers a descriptor. The `64` forms
/// resolve their own real name first, and the plain name where the libc
/// has no `64` name (musl 1.2.4 and later): both open with the same flags
/// on a 64-bit target, T-1701.
macro_rules! mkstemp_fd {
    ($name:ident, $($real:ident),+ ; ( $($arg:ident : $ty:ty),* ), $suffix:expr) => {
        /// # Safety
        /// The payload's own contract for this entry point.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(template: *mut c_char $(, $arg: $ty)*) -> c_int {
            let Some(f) = None $(.or_else($real))+ else {
                set_errno(EINVAL);
                return -1;
            };
            let what = stringify!($name).as_bytes();
            unsafe { on_template(what, template, $suffix, -1, |fd| fd >= 0, |t| f(t $(, $arg)*)) }
        }
    };
}

mkstemp_fd!(mkstemp, next_mkstemp; (), 0);
mkstemp_fd!(mkostemp, next_mkostemp; (flags: c_int), 0);
mkstemp_fd!(mkstemp64, next_mkstemp64, next_mkstemp; (), 0);
mkstemp_fd!(mkostemp64, next_mkostemp64, next_mkostemp; (flags: c_int), 0);
mkstemp_fd!(mkstemps, next_mkstemps; (suffixlen: c_int), suffixlen);
mkstemp_fd!(mkostemps, next_mkostemps; (suffixlen: c_int, flags: c_int), suffixlen);
