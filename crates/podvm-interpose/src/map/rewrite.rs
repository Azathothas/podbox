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

/// Absolutize `path` for `dirfd` into `out`, T-0703's `*at` rule.
///
/// An absolute path is copied. `AT_FDCWD` resolves through the real `getcwd`,
/// which needs no `/proc`. Any other descriptor resolves through
/// `/proc/self/fd/<n>`; where that read fails there is no answer, only an
/// absence. Returns the length, or `-1` where it cannot be answered.
///
/// # Safety
/// `path` must be readable up to its NUL within `OUT` bytes and `out` must
/// hold `OUT` writable bytes, which is what every entry point passes.
pub unsafe fn absolutize(dirfd: c_int, path: *const c_char, out: &mut [u8; OUT]) -> isize {
    let getcwd = match next_getcwd() {
        Some(f) => f,
        None => return -1,
    };
    let readlink = match next_readlink() {
        Some(f) => f,
        None => return -1,
    };
    let n = strnlen(path, OUT);
    if n == 0 || n >= OUT {
        return -1;
    }
    if unsafe { *(path as *const u8) } == b'/' {
        unsafe {
            core::ptr::copy_nonoverlapping(path as *const u8, out.as_mut_ptr(), n);
        }
        return n as isize;
    }
    // The base: the working directory, or the descriptor's target.
    let mut base = [0u8; OUT];
    let base_n: usize = if dirfd == AT_FDCWD {
        if unsafe { getcwd(base.as_mut_ptr() as *mut c_char, OUT) }.is_null() {
            return -1;
        }
        strnlen(base.as_ptr() as *const c_char, OUT)
    } else {
        // `/proc/self/fd/<n>` in a 64-byte stack buffer: the longest `c_int`
        // is 11 digits plus the sign.
        let mut link = [0u8; 64];
        let head = b"/proc/self/fd/";
        unsafe {
            core::ptr::copy_nonoverlapping(head.as_ptr(), link.as_mut_ptr(), head.len());
        }
        let mut num = [0u8; 12];
        let neg = dirfd < 0;
        let mut v = (dirfd as i64).unsigned_abs();
        let mut d = 0usize;
        loop {
            num[d] = b'0' + (v % 10) as u8;
            v /= 10;
            d += 1;
            if v == 0 {
                break;
            }
        }
        let mut at = head.len();
        if neg {
            link[at] = b'-';
            at += 1;
        }
        while d > 0 {
            d -= 1;
            link[at] = num[d];
            at += 1;
        }
        link[at] = 0;
        let r = unsafe {
            readlink(
                link.as_ptr() as *const c_char,
                base.as_mut_ptr() as *mut c_char,
                OUT - 1,
            )
        };
        if r <= 0 || r as usize >= OUT {
            return -1;
        }
        r as usize
    };
    if base_n == 0 || base_n + 1 + n >= OUT {
        return -1;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(base.as_ptr(), out.as_mut_ptr(), base_n);
        let mut at = base_n;
        if out[at - 1] != b'/' {
            out[at] = b'/';
            at += 1;
        }
        core::ptr::copy_nonoverlapping(path as *const u8, out.as_mut_ptr().add(at), n);
        (at + n) as isize
    }
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
/// itself fails.
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
    if path.is_null() {
        return path;
    }
    let t = unsafe { table() };
    if t.n == 0 {
        return path;
    }
    if strnlen(path, OUT) >= OUT {
        return path;
    }
    let relative = unsafe { *(path as *const u8) } != b'/';
    let mut abs = [0u8; OUT];
    if unsafe { absolutize(dirfd, path, &mut abs) } < 0 {
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
    match unsafe { rewrite(&t, abs.as_ptr(), abs_n, out) } {
        // ⭐ Unmatched resolves back to the caller's own pointer, never the
        // absolutized copy: the bytes the kernel sees stay exactly the
        // payload's, including any symlinks in the working directory prefix.
        -1 => path,
        -2 => {
            crate::say::line(&[what, b": the mapped path does not fit, so the call fails"]);
            crate::set_errno(ENAMETOOLONG);
            core::ptr::null()
        }
        _ => out.as_ptr() as *const c_char,
    }
}

/// Match and rewrite a path that is already absolute-or-literal, without any
/// `dirfd` resolution. `symlink`'s target takes this shape: the content is
/// resolved when the link is used, not when it is made, so resolving it now
/// would answer about the wrong directory.
///
/// # Safety
/// As [`absolutize`]: a readable path and a writable `OUT`-byte buffer.
pub unsafe fn prepare_literal(
    what: &[u8],
    path: *const c_char,
    out: &mut [u8; OUT],
) -> *const c_char {
    if path.is_null() {
        return path;
    }
    let t = unsafe { table() };
    if t.n == 0 {
        return path;
    }
    let n = strnlen(path, OUT);
    if n >= OUT {
        return path;
    }
    match unsafe { rewrite(&t, path as *const u8, n, out) } {
        -1 => path,
        -2 => {
            crate::say::line(&[what, b": the mapped path does not fit, so the call fails"]);
            crate::set_errno(ENAMETOOLONG);
            core::ptr::null()
        }
        _ => out.as_ptr() as *const c_char,
    }
}
