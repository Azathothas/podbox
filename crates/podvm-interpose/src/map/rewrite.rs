use super::*;

/// The variable that carries the user exclusion list, T-0707.
pub const EXCLUDE_VAR: &[u8] = b"PODVM_EXCLUDE_PATH";

/// Built-in never-rewrite prefix, T-0707.
///
/// The interposer's own machinery reads through `/proc/self`: `*at`
/// resolution reads `/proc/self/fd/<n>`, and classification reads the
/// payload's ELF through `/proc/self/exe`. One tree prefix covers the
/// leaves the machinery names today and whatever it names next; a mapping
/// that moved them would virtualize the interposer itself, and the failure
/// would name nothing. It is code, not configuration, so no user list
/// removes it. The boundary below keeps `/proctor` out of it.
pub(super) fn builtin_excluded(path: *const u8, path_n: usize) -> bool {
    const PROC: &[u8] = b"/proc";
    prefix(PROC.as_ptr(), PROC.len(), path, path_n)
}

/// One user exclusion prefix against `path`, T-0707.
///
/// The value is colon-separated prefixes sharing the component-boundary
/// rule above, so `/proc` excludes `/proc/self/exe` while `/proctor`
/// excludes nothing. Empty items are skipped: a leading, trailing or
/// doubled colon is a typo, not an entry.
///
/// # Safety
/// `v_p` must be readable for `v_n` bytes, `path` for `path_n`.
pub(super) fn user_excluded(v_p: *const u8, v_n: usize, path: *const u8, path_n: usize) -> bool {
    let mut at = 0usize;
    while at <= v_n {
        let mut end = at;
        while end < v_n && unsafe { *v_p.add(end) } != b':' {
            end += 1;
        }
        if end > at && prefix(unsafe { v_p.add(at) }, end - at, path, path_n) {
            return true;
        }
        at = end + 1;
    }
    false
}

/// Never rewrite `path`, T-0707.
///
/// The built-in first, the user list second: the built-in entry is not
/// removable, and the user list only adds. Both directions consult it:
/// `longest` keeps machinery paths out of the forward map, and
/// `longest_to` keeps results naming them out of the reverse one.
///
/// # Safety
/// `path` must be readable for `path_n` bytes. Reads the process's environ
/// for the user list, which the kernel keeps valid for the process's life.
unsafe fn excluded(path: *const u8, path_n: usize) -> bool {
    if builtin_excluded(path, path_n) {
        return true;
    }
    match unsafe { lookup_env(EXCLUDE_VAR) } {
        Some((v_p, v_n)) => user_excluded(v_p, v_n, path, path_n),
        None => false,
    }
}

/// The longest matching pair for an absolute `path`, or `None`.
///
/// ⭐ Longest wins rather than first, so the answer does not depend on the
/// order pairs were written in. ⛔ Paths under `/.podvm/` never match: the
/// memo and the object itself live there, and a mapping that moved them
/// would silently disarm the ownership wall. T-0707's never-rewrite paths
/// never match either, for the same reason in the other direction: they are
/// the interposer's own machinery.
///
/// # Safety
/// `path` must be readable for `path_n` bytes.
pub unsafe fn longest(t: &Table, path: *const u8, path_n: usize) -> Option<(usize, usize)> {
    const GUARD: &[u8] = b"/.podvm/";
    if path_n >= GUARD.len() && eq(path, GUARD.as_ptr(), GUARD.len()) {
        return None;
    }
    if path_n == b"/.podvm".len() && eq(path, b"/.podvm".as_ptr(), path_n) {
        return None;
    }
    if unsafe { excluded(path, path_n) } {
        return None;
    }
    let mut best: Option<(usize, usize)> = None;
    let mut i = 0usize;
    while i < t.n {
        let p = &t.pairs[i];
        if prefix(p.from_p, p.from_n, path, path_n) {
            let better = match best {
                None => true,
                Some((_, n)) => p.from_n > n,
            };
            if better {
                best = Some((i, p.from_n));
            }
        }
        i += 1;
    }
    best
}

/// Rewrite an absolute path through the table into `out`.
///
/// Returns the new length, or `-1` where nothing matched (`out` untouched)
/// or the rewrite would not fit (`ENAMETOOLONG`, never a truncation).
///
/// # Safety
/// `path` must be readable for `path_n` bytes.
pub unsafe fn rewrite(t: &Table, path: *const u8, path_n: usize, out: &mut [u8; OUT]) -> isize {
    if unsafe { under_host_side(t, path, path_n) } {
        return -1;
    }
    let Some((idx, from_n)) = (unsafe { longest(t, path, path_n) }) else {
        return -1;
    };
    let p = &t.pairs[idx];
    let rest_n = path_n - from_n;
    // ⭐ The remainder of a `/`-prefix match carries no leading slash
    // (`/etc` minus `/` is `etc`), so one goes back in unless the target
    // already ends in one. Without it the join drops a separator and maps
    // the wrong tree.
    let slash = rest_n > 0
        && unsafe { *path.add(from_n) } != b'/'
        && unsafe { *p.to_p.add(p.to_n - 1) } != b'/';
    // ⭐ The mirror of the root join in `unrewrite`: the target ends in the
    // separator the remainder starts with, so one of the two goes or the
    // mapped path doubles its slash. A trailing-slash target beside a
    // rooted remainder is the shape that fires it.
    let double = rest_n > 0
        && unsafe { *path.add(from_n) } == b'/'
        && unsafe { *p.to_p.add(p.to_n - 1) } == b'/';
    if p.to_n + rest_n + (slash as usize) >= OUT {
        return -2;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(p.to_p, out.as_mut_ptr(), p.to_n);
        let mut at = p.to_n;
        if slash {
            *out.as_mut_ptr().add(at) = b'/';
            at += 1;
        }
        let skip = double as usize;
        core::ptr::copy_nonoverlapping(
            path.add(from_n + skip),
            out.as_mut_ptr().add(at),
            rest_n - skip,
        );
        (at + rest_n - skip) as isize
    }
}

/// True where `path` already names a place under the host side of a pair
/// (the rootfs, or the source of a volume), so a rewrite would add the
/// prefix a second time.
///
/// ⛔ TODO/interpose.md T-1705. A runtime opens files by the paths that the
/// loader gives it (`dladdr`, the link map, `/proc/self/maps`), and podvm
/// starts the image's loader with host library directories, so those paths
/// are host paths under the rootfs. CoreCLR of the official runner opened
/// one with the prefix twice and failed (`ENOENT`, exit 137). A rewrite is
/// idempotent: such a path passes unchanged. Only a path under a host side
/// passes, so the rule gives no reach outside the rootfs and the volumes.
/// A host side of `/` (a volume of the host root) is left out: each path
/// is under it. A guest path that names the host source of a volume under
/// another guest name reaches that source: the limit of this rule.
///
/// # Safety
/// `path` must be readable for `path_n` bytes.
pub unsafe fn under_host_side(t: &Table, path: *const u8, path_n: usize) -> bool {
    let mut i = 0usize;
    while i < t.n {
        let p = &t.pairs[i];
        let mut to_n = p.to_n;
        if to_n > 1 && unsafe { *p.to_p.add(to_n - 1) } == b'/' {
            to_n -= 1;
        }
        if to_n > 1 && prefix(p.to_p, to_n, path, path_n) {
            return true;
        }
        i += 1;
    }
    false
}

/// The longest matching pair for a real `path`, matched on the TO side, or
/// `None`. T-0705's mirror of [`longest`]: where the forward match asks
/// which virtual prefix a payload path lies under, this asks which real
/// prefix a result lies under, so the result can be read back in virtual
/// names. A trailing slash on TO is ignored for the match, or the pair
/// could never match anything under it. The `/.podvm/` guard holds on
/// this side too: the object and the memo live there. T-0707's
/// never-rewrite paths hold here as well: a result naming the machinery
/// stays in real names.
///
/// # Safety
/// `path` must be readable for `path_n` bytes.
pub unsafe fn longest_to(t: &Table, path: *const u8, path_n: usize) -> Option<(usize, usize)> {
    const GUARD: &[u8] = b"/.podvm/";
    if path_n >= GUARD.len() && eq(path, GUARD.as_ptr(), GUARD.len()) {
        return None;
    }
    if path_n == b"/.podvm".len() && eq(path, b"/.podvm".as_ptr(), path_n) {
        return None;
    }
    if unsafe { excluded(path, path_n) } {
        return None;
    }
    let mut best: Option<(usize, usize)> = None;
    let mut i = 0usize;
    while i < t.n {
        let p = &t.pairs[i];
        let mut to_n = p.to_n;
        if to_n > 1 && unsafe { *p.to_p.add(to_n - 1) } == b'/' {
            to_n -= 1;
        }
        if prefix(p.to_p, to_n, path, path_n) {
            let better = match best {
                None => true,
                Some((_, n)) => to_n > n,
            };
            if better {
                best = Some((i, to_n));
            }
        }
        i += 1;
    }
    best
}

/// Reverse a real path through the table into `out`, T-0705.
///
/// Mirrors [`rewrite`] with the sides swapped: match the real TO prefix,
/// substitute the virtual FROM, so a payload reading back a path it wrote
/// sees the virtual name. Returns the new length, or `-1` where nothing
/// matched (`out` untouched) or `-2` where the virtual name would not fit.
/// A `-2` is the caller answering `ERANGE`, never a truncation and never
/// the real path, which would leak the mapping.
///
/// # Safety
/// `path` must be readable for `path_n` bytes.
pub unsafe fn unrewrite(t: &Table, path: *const u8, path_n: usize, out: &mut [u8; OUT]) -> isize {
    let Some((idx, to_n)) = (unsafe { longest_to(t, path, path_n) }) else {
        return -1;
    };
    let p = &t.pairs[idx];
    let rest_n = path_n - to_n;
    // ⭐ Mirror of the join in `rewrite`: the remainder of a `/`-prefix
    // match carries no leading slash, so one goes back in unless the
    // virtual side already ends in one.
    let slash = rest_n > 0
        && unsafe { *path.add(to_n) } != b'/'
        && unsafe { *p.from_p.add(p.from_n - 1) } != b'/';
    // ⭐ The root join: FROM `/` ends in the separator the remainder starts
    // with, so one of the two goes or every virtual path doubles its slash
    // (`/tmp` reads back `//tmp`, TODO/enter.md T-1411). The mirror guard
    // lives in `rewrite`.
    let double = rest_n > 0
        && unsafe { *path.add(to_n) } == b'/'
        && unsafe { *p.from_p.add(p.from_n - 1) } == b'/';
    if p.from_n + rest_n + (slash as usize) >= OUT {
        return -2;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(p.from_p, out.as_mut_ptr(), p.from_n);
        let mut at = p.from_n;
        if slash {
            *out.as_mut_ptr().add(at) = b'/';
            at += 1;
        }
        let skip = double as usize;
        core::ptr::copy_nonoverlapping(
            path.add(to_n + skip),
            out.as_mut_ptr().add(at),
            rest_n - skip,
        );
        (at + rest_n - skip) as isize
    }
}

/// Absolutize `path` for `dirfd` into `out` as a guest path, T-0703, T-1643.
///
/// An absolute path is copied. `AT_FDCWD` resolves through the real `getcwd`,
/// which needs no `/proc`. Any other descriptor resolves through
/// `/proc/self/fd/<n>`. The base is a host path, so `guest_base` reverse-maps
/// it before the join, and `normalise` folds the result. Returns the length,
/// or `-1` where it cannot be answered. The output carries a NUL after it.
/// # Safety
/// `path` must be readable up to its NUL within `OUT` bytes, `out` must hold
/// `OUT` writable bytes, which is what every entry point passes.
pub unsafe fn absolutize(
    t: &Table,
    dirfd: c_int,
    path: *const c_char,
    out: &mut [u8; OUT],
) -> isize {
    let n = strnlen(path, OUT);
    if n == 0 || n >= OUT {
        return -1;
    }
    let path = unsafe { core::slice::from_raw_parts(path as *const u8, n) };
    if path[0] == b'/' {
        out[..n].copy_from_slice(path);
        let m = normalise(out, n);
        out[m] = 0;
        return m as isize;
    }
    // The base: the working directory, or the descriptor's target. Both
    // are host paths here.
    let mut base = [0u8; OUT];
    let base_n = if dirfd == AT_FDCWD {
        let Some(getcwd) = next_getcwd() else {
            return -1;
        };
        if unsafe { getcwd(base.as_mut_ptr() as *mut c_char, OUT) }.is_null() || base[0] != b'/' {
            return -1;
        }
        strnlen(base.as_ptr() as *const c_char, OUT) as isize
    } else {
        let Some(readlink) = next_readlink() else {
            return -1;
        };
        unsafe { fd_base(readlink, dirfd, &mut base) }
    };
    if base_n <= 0 {
        return -1;
    }
    // ⭐ T-1643: the input to `rewrite` is always a guest path. A base that
    // carries the rootfs prefix goes back to guest names first, or the
    // `/` pair adds that prefix a second time.
    let base_n = unsafe { guest_base(t, &mut base, base_n as usize) };
    if base_n <= 0 || base_n as usize + 1 + n >= OUT {
        return -1;
    }
    let base_n = base_n as usize;
    out[..base_n].copy_from_slice(&base[..base_n]);
    out[base_n] = b'/';
    out[base_n + 1..base_n + 1 + n].copy_from_slice(path);
    let m = normalise(out, base_n + 1 + n);
    out[m] = 0;
    m as isize
}

/// The pointer one path-taking call passes on.
///
/// ⭐ One funnel for every entry point in `lib.rs`: absolutize against the
/// table, and hand back either the rewritten buffer or the caller's own
/// pointer. A null return means the call does not happen: `errno` is set
/// and one line is said.
///
/// ⭐ A descriptor-relative path the table cannot be checked against goes
/// through UNCHANGED, not failed. The kernel resolves it through a directory
/// that is already real -- every directory descriptor the payload holds came
/// from a call this object already rewrote -- so forwarding is kernel-correct
/// and failing it would break every tool that walks with `openat`, like `tar`
/// and `ls -R`, on any run with a table set. Only `AT_FDCWD`, which resolves
/// through the working directory, can fail here, and only where `getcwd`
/// itself fails or its guest name does not fit. T-1643: that failure never
/// falls back to the unmapped relative path.
///
/// Three fast paths that never touch the table: a null pointer reaches the
/// real call (which answers `EFAULT` as it always did), an empty table
/// rewrites nothing (so an unmapped payload cannot fail here at all), and an
/// overlong path reaches the real call (which answers `ENAMETOOLONG`).
///
/// # Safety
/// As [`absolutize`]: a readable path and a writable `OUT`-byte buffer.
pub unsafe fn prepare(
    what: &[u8],
    dirfd: c_int,
    path: *const c_char,
    out: &mut [u8; OUT],
) -> *const c_char {
    unsafe { prepare_mode(what, dirfd, path, is_write_call(what), out) }
}

/// [`prepare`] where the caller says whether the call writes: `open` and
/// `fopen` write only with some flags. A write under a read-only prefix of
/// `PODVM_RO` answers `EROFS` (TODO/cli.md T-1655).
///
/// # Safety
/// As [`prepare`].
pub unsafe fn prepare_mode(
    what: &[u8],
    dirfd: c_int,
    path: *const c_char,
    writes: bool,
    out: &mut [u8; OUT],
) -> *const c_char {
    unsafe { prepare_follow(what, dirfd, path, writes, follows(what), out) }
}

/// [`prepare_mode`] where the caller says whether the call follows the
/// link of its last component: `O_NOFOLLOW`, `AT_SYMLINK_NOFOLLOW` and
/// `AT_SYMLINK_FOLLOW` are flags of the call (TODO/interpose.md T-1725).
///
/// # Safety
/// As [`prepare`].
pub unsafe fn prepare_follow(
    what: &[u8],
    dirfd: c_int,
    path: *const c_char,
    writes: bool,
    follow: bool,
    out: &mut [u8; OUT],
) -> *const c_char {
    if path.is_null() {
        return path;
    }
    let t = unsafe { table() };
    if t.n == 0 {
        return path;
    }
    // An empty path reaches the kernel, which answers ENOENT for it: the
    // .NET runtime asks `realpath("")` and `access("")` at each start, and
    // a line of podvm for each would be noise (TODO/cli.md T-1706).
    let n = strnlen(path, OUT);
    if n == 0 || n >= OUT {
        return path;
    }
    let relative = unsafe { *(path as *const u8) } != b'/';
    let mut abs = [0u8; OUT];
    if unsafe { absolutize(&t, dirfd, path, &mut abs) } < 0 {
        // ⭐ Unresolvable against a descriptor: forward, per above. Unresolvable
        // against the working directory: the process is genuinely lost, fail it.
        if relative && dirfd != AT_FDCWD {
            return path;
        }
        crate::say::line(&[what, b": podvm cannot resolve the path, so the call fails"]);
        crate::set_errno(ENOENT);
        return core::ptr::null();
    }
    let abs_n = strnlen(abs.as_ptr() as *const c_char, OUT);
    if writes && unsafe { denied(abs.as_ptr(), abs_n) } {
        crate::set_errno(EROFS);
        return core::ptr::null();
    }
    match unsafe { rewrite(&t, abs.as_ptr(), abs_n, out) } {
        // ⭐ Unmatched resolves back to the caller's own pointer, never the
        // absolutized copy: the bytes the kernel sees stay exactly the
        // payload's, including any symlinks in the working directory prefix.
        // The kernel then resolves a relative path against the real base.
        -1 => path,
        -2 => {
            crate::say::line(&[what, b": the mapped path does not fit, so the call fails"]);
            crate::set_errno(ENAMETOOLONG);
            core::ptr::null()
        }
        // ⭐ T-1725: a path in the table resolves its links in guest
        // names, so an absolute link target stays in the rootfs. Where the
        // walk cannot run (no real `readlink`), the rewrite stands.
        // The NUL ends the name here: a buffer used again (the per-thread
        // buffer of `dlopen`, T-1645) holds the tail of a longer name.
        n => match unsafe { resolve_links(&t, &abs[..abs_n], follow, out) } {
            Ok(m) => {
                out[m] = 0;
                out.as_ptr() as *const c_char
            }
            Err(EINVAL) => {
                let n = unsafe { rewrite(&t, abs.as_ptr(), abs_n, out) }.max(n);
                out[n as usize] = 0;
                out.as_ptr() as *const c_char
            }
            Err(e) => {
                crate::say::line(&[what, b": a link of the path does not resolve inside the rootfs, so the call fails (TODO/interpose.md T-1725)"]);
                crate::set_errno(e);
                core::ptr::null()
            }
        },
    }
}

