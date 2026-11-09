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
                crate::map::prepare(stringify!($name).as_bytes(), $dirfd, $path, &mut buf)
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

/// A `mk*` template is rewritten in place, because the caller reads the
/// chosen name back out of the same buffer.
///
/// ⛔ The mapped name lands only where it fits in the bytes the template
/// already occupies. Writing past them would overflow a buffer whose size
/// the caller never states, so a longer mapping is `ERANGE` rather than a
/// truncation. That is T-0705's decision, applied early.
unsafe fn place_template(what: &[u8], template: *mut c_char, rewritten: *const c_char) -> bool {
    let mut n = 0usize;
    while n < crate::map::OUT {
        if unsafe { *(rewritten as *const u8).add(n) } == 0 {
            break;
        }
        n += 1;
    }
    let mut cap = 0usize;
    while cap < crate::map::OUT {
        if unsafe { *(template as *const u8).add(cap) } == 0 {
            break;
        }
        cap += 1;
    }
    if n > cap {
        crate::say::line(&[what, b": the mapped name does not fit the template"]);
        set_errno(ERANGE);
        return false;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(rewritten as *const u8, template as *mut u8, n + 1);
    }
    true
}

/// Rewrite `template` through the table where it matches, leaving it alone
/// where it does not. Returns the pointer to call with, or null on failure.
pub(super) unsafe fn map_template(
    what: &[u8],
    template: *mut c_char,
    out: &mut [u8; crate::map::OUT],
) -> *mut c_char {
    if template.is_null() {
        return template;
    }
    let t = unsafe { crate::map::table() };
    if t.n == 0 {
        return template;
    }
    let n = crate::map::strnlen(template, crate::map::OUT);
    if n >= crate::map::OUT {
        return template;
    }
    // Templates are relative to the working directory by construction.
    let mut abs = [0u8; crate::map::OUT];
    if unsafe { crate::map::absolutize(&t, crate::map::AT_FDCWD, template, &mut abs) } < 0 {
        crate::say::line(&[what, b": podvm cannot resolve the path, so the call fails"]);
        set_errno(2);
        return core::ptr::null_mut();
    }
    let abs_n = crate::map::strnlen(abs.as_ptr() as *const c_char, crate::map::OUT);
    match unsafe { crate::map::rewrite(&t, abs.as_ptr(), abs_n, out) } {
        -1 => template,
        -2 => {
            crate::say::line(&[what, b": the mapped path does not fit"]);
            set_errno(ENAMETOOLONG);
            core::ptr::null_mut()
        }
        _ => {
            if !unsafe { place_template(what, template, out.as_ptr() as *const c_char) } {
                return core::ptr::null_mut();
            }
            template
        }
    }
}

/// # Safety
/// The payload's own contract for `mkstemp(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mkstemp(template: *mut c_char) -> c_int {
    let Some(f) = next_mkstemp() else {
        set_errno(EINVAL);
        return -1;
    };
    let mut buf = [0u8; crate::map::OUT];
    let t = unsafe { map_template(b"mkstemp", template, &mut buf) };
    if t.is_null() {
        return -1;
    }
    unsafe { f(t) }
}

/// # Safety
/// The payload's own contract for `mkostemp(3)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mkostemp(template: *mut c_char, flags: c_int) -> c_int {
    let Some(f) = next_mkostemp() else {
        set_errno(EINVAL);
        return -1;
    };
    let mut buf = [0u8; crate::map::OUT];
    let t = unsafe { map_template(b"mkostemp", template, &mut buf) };
    if t.is_null() {
        return -1;
    }
    unsafe { f(t, flags) }
}
