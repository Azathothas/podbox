//! The path table and the rewrite, for T-0703.
//!
//! [`TODO/interpose.md`](../../../TODO/interpose.md) T-0703. Every path-taking
//! entry point in `lib.rs` funnels through [`prepare`]: absolutize the path,
//! match it against the table, and hand back the pointer to call with.
//!
//! ⭐ **The table comes from the environment, as one variable.** `PODBOX_MAPS`
//! carries `FROM:TO[,FROM:TO...]` pairs, which mirrors pathmap's `PATH_MAPPING`
//! syntax without sharing its name: one word, one meaning, and podbox never
//! reads another tool's variable. No `-v` exists yet, so nothing sets it and
//! every call takes the empty-table fast path below.
//!
//! ⛔ **No allocation and no locks on this path.** The table borrows the
//! process's own environ memory as `(pointer, length)` pairs on the caller's
//! stack, so parsing costs a walk and no bytes. Rewriting writes into the
//! caller's 4 KiB stack buffer.

use core::ffi::{c_char, c_int};

extern "C" {
    static environ: *const *const c_char;
}

/// The variable that carries the table.
pub const MAPS_VAR: &[u8] = b"PODBOX_MAPS";

/// How many pairs a table holds. Bounded, because this runs on every path a
/// payload touches: a mapping with more entries than this is a configuration
/// error, and the excess pairs are ignored rather than allocated for.
pub const MAX_PAIRS: usize = 32;

/// How long one side of a pair may be. A longer side is malformed and its
/// pair is skipped: silently truncating a prefix would map the wrong tree.
pub const MAX_SIDE: usize = 4096;

/// The rewrite buffer every entry point owns on its stack.
pub const OUT: usize = 4096;

const ENOENT: c_int = 2;
const ENAMETOOLONG: c_int = 36;

pub const AT_FDCWD: c_int = -100;

crate::real!(fn next_getcwd = "getcwd"(*mut c_char, usize) -> *mut c_char);
crate::real!(fn next_readlink = "readlink"(*const c_char, *mut c_char, usize) -> isize);

/// The process's own table, parsed out of its environ.
///
/// # Safety
/// Reads the process's environ, which the kernel keeps valid for the
/// process's life.
pub unsafe fn table() -> Table {
    unsafe { parse(environ) }
}

/// One pair, borrowing environ memory. `Copy` so the table is a plain array.
#[derive(Clone, Copy)]
pub struct Pair {
    pub from_p: *const u8,
    pub from_n: usize,
    pub to_p: *const u8,
    pub to_n: usize,
}

/// The parsed table. Empty where the variable is absent, empty, or malformed
/// throughout: an empty table rewrites nothing, which is the safe direction.
pub struct Table {
    pub pairs: [Pair; MAX_PAIRS],
    pub n: usize,
}

const NO_PAIR: Pair = Pair {
    from_p: core::ptr::null(),
    from_n: 0,
    to_p: core::ptr::null(),
    to_n: 0,
};

/// Byte length of a NUL-terminated string, capped so a missing NUL cannot run
/// the scan off the end of anything.
pub(crate) fn strnlen(p: *const c_char, cap: usize) -> usize {
    let mut n = 0usize;
    while n < cap {
        if unsafe { *(p as *const u8).add(n) } == 0 {
            break;
        }
        n += 1;
    }
    n
}

/// Compare `len` bytes at `a` and `b`.
fn eq(a: *const u8, b: *const u8, len: usize) -> bool {
    for i in 0..len {
        if unsafe { *a.add(i) } != unsafe { *b.add(i) } {
            return false;
        }
    }
    true
}

/// The value of one `NAME=VALUE` entry, or `None` where the entry is some
/// other variable. The length check runs first so the compare never reads
/// past a short string's NUL.
fn match_var(e: *const c_char, name: &[u8]) -> Option<*const u8> {
    if strnlen(e, name.len()) == name.len()
        && eq(e as *const u8, name.as_ptr(), name.len())
        && unsafe { *(e as *const u8).add(name.len()) } == b'='
    {
        Some(unsafe { (e as *const u8).add(name.len() + 1) })
    } else {
        None
    }
}

/// The value of one variable, as pointer and length into environ memory.
///
/// `None` where the variable is absent. Used for the small podbox-owned
/// variables (`PODBOX_MAPS` goes through [`parse`]; this serves the rest)
///
/// # Safety
/// Reads the process's environ, which the kernel keeps valid.
pub unsafe fn lookup_env(name: &[u8]) -> Option<(*const u8, usize)> {
    let env = unsafe { environ };
    if env.is_null() {
        return None;
    }
    let mut i = 0usize;
    while i < 4096 {
        let e = unsafe { *env.add(i) };
        if e.is_null() {
            break;
        }
        if let Some(v) = match_var(e, name) {
            return Some((v, strnlen(v as *const c_char, 65536)));
        }
        i += 1;
    }
    None
}

/// Parse the table out of one environ image.
///
/// `env` is the NUL-terminated `NAME=VALUE` pointers; the tests hand it a
/// static array and production hands it the process's own environ.
///
/// # Safety
/// `env` must be null or point at a NUL-terminated pointer array whose
/// entries are NUL-terminated within a page the process can read, which is
/// what the kernel builds and what the tests hand in.
pub unsafe fn parse(env: *const *const c_char) -> Table {
    let mut t = Table {
        pairs: [NO_PAIR; MAX_PAIRS],
        n: 0,
    };
    if env.is_null() {
        return t;
    }
    let var = MAPS_VAR;
    let mut i = 0usize;
    while i < 4096 {
        let e = unsafe { *env.add(i) };
        if e.is_null() {
            break;
        }
        // `PODBOX_MAPS=` then the pairs.
        if let Some(v) = match_var(e, var) {
            parse_pairs(v, &mut t);
            break;
        }
        i += 1;
    }
    t
}

/// Parse `FROM:TO[,FROM:TO...]` at `s` (NUL-terminated) into `t`.
///
/// ⚠ Malformed pairs are SKIPPED, not fatal: one bad pair must not drop the
/// good ones beside it, and failing the payload's call for a configuration
/// typo would be the wrong layer reporting it. An empty side is malformed.
fn parse_pairs(s: *const u8, t: &mut Table) {
    let mut at = s;
    unsafe {
        loop {
            if *at == 0 || t.n >= MAX_PAIRS {
                break;
            }
            // FROM runs to `:` or `,` or NUL.
            let from_p = at;
            while *at != 0 && *at != b':' && *at != b',' {
                at = at.add(1);
            }
            let from_n = at as usize - from_p as usize;
            if *at != b':' {
                // No `:`, so no pair here. Skip to the next comma.
                while *at != 0 && *at != b',' {
                    at = at.add(1);
                }
                if *at == b',' {
                    at = at.add(1);
                }
                continue;
            }
            at = at.add(1);
            let to_p = at;
            while *at != 0 && *at != b',' {
                at = at.add(1);
            }
            let to_n = at as usize - to_p as usize;
            if *at == b',' {
                at = at.add(1);
            }
            if from_n == 0 || to_n == 0 || from_n > MAX_SIDE || to_n > MAX_SIDE {
                continue;
            }
            // A trailing slash on FROM would deaden the pair: `/mapped/` only
            // matches the path `/mapped/` itself, never anything under it.
            // Strip one, so the pair means what its author meant.
            let mut from_n = from_n;
            if from_n > 1 && *from_p.add(from_n - 1) == b'/' {
                from_n -= 1;
            }
            // A FROM of `/` alone would match everything; that is what a
            // caller asking for it means, so it is kept, and the longest
            // match below still prefers anything more specific.
            t.pairs[t.n] = Pair {
                from_p,
                from_n,
                to_p,
                to_n,
            };
            t.n += 1;
        }
    }
}

/// Does `from` match `path` at a component boundary, and how many bytes of
/// `path` does it cover?
fn prefix(from_p: *const u8, from_n: usize, path: *const u8, path_n: usize) -> bool {
    if from_n > path_n || !eq(from_p, path, from_n) {
        return false;
    }
    // ⭐ The boundary: `/mapped` must not match `/mapped2`. A prefix of `/`
    // matches everything under the root, which is what it says.
    if from_n == 1 {
        return unsafe { *from_p } == b'/';
    }
    path_n == from_n || unsafe { *path.add(from_n) } == b'/'
}

/// The longest matching pair for an absolute `path`, or `None`.
///
/// ⭐ Longest wins rather than first, so the answer does not depend on the
/// order pairs were written in. ⛔ Paths under `/.podbox/` never match: the
/// memo and the object itself live there, and a mapping that moved them
/// would silently disarm the ownership wall.
///
/// # Safety
/// `path` must be readable for `path_n` bytes.
pub unsafe fn longest(t: &Table, path: *const u8, path_n: usize) -> Option<(usize, usize)> {
    const GUARD: &[u8] = b"/.podbox/";
    if path_n >= GUARD.len() && eq(path, GUARD.as_ptr(), GUARD.len()) {
        return None;
    }
    if path_n == b"/.podbox".len() && eq(path, b"/.podbox".as_ptr(), path_n) {
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
        core::ptr::copy_nonoverlapping(path.add(from_n), out.as_mut_ptr().add(at), rest_n);
        (at + rest_n) as isize
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
        crate::say::line(&[what, b": podbox cannot resolve the path, so the call fails"]);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake environ image: static, NUL-terminated, process-stable like the
    /// real one, so the table borrows it the same way.
    fn env_of<const N: usize>(entries: [&'static [u8]; N]) -> Vec<*const c_char> {
        let mut v: Vec<*const c_char> = entries
            .iter()
            .map(|e| e.as_ptr() as *const c_char)
            .collect();
        v.push(core::ptr::null());
        v
    }

    #[test]
    fn an_absent_variable_is_an_empty_table() {
        let env = env_of([b"PATH=/bin\0", b"HOME=/root\0"]);
        let t = unsafe { parse(env.as_ptr()) };
        assert_eq!(t.n, 0);
        let env2: Vec<*const c_char> = vec![core::ptr::null()];
        let t2 = unsafe { parse(env2.as_ptr()) };
        assert_eq!(t2.n, 0);
        let t3 = unsafe { parse(core::ptr::null()) };
        assert_eq!(t3.n, 0);
    }

    /// `rewrite` through one unsafe block, so the cases read as cases.
    fn rw(t: &Table, path: &[u8], out: &mut [u8; OUT]) -> isize {
        unsafe { rewrite(t, path.as_ptr(), path.len(), out) }
    }

    #[test]
    fn pairs_parse_and_match_at_a_boundary() {
        let env = env_of([b"PODBOX_MAPS=/mapped:/real,/m2:/r2\0"]);
        let t = unsafe { parse(env.as_ptr()) };
        assert_eq!(t.n, 2);
        // Matches, with the rest carried over.
        let mut out = [0u8; OUT];
        let n = rw(&t, b"/mapped/a", &mut out);
        assert_eq!(&out[..n as usize], b"/real/a");
        // ⛔ `/mapped2` is not under `/mapped`.
        assert_eq!(rw(&t, b"/mapped2/a", &mut out), -1);
        // Unmatched is untouched.
        assert_eq!(rw(&t, b"/other/a", &mut out), -1);
    }

    #[test]
    fn the_longest_prefix_wins_whatever_the_order() {
        let env = env_of([b"PODBOX_MAPS=/:/root-fallback,/mapped:/real\0"]);
        let t = unsafe { parse(env.as_ptr()) };
        assert_eq!(t.n, 2);
        let mut out = [0u8; OUT];
        let n = rw(&t, b"/mapped/a", &mut out);
        assert_eq!(&out[..n as usize], b"/real/a");
        let n = rw(&t, b"/etc/hostname", &mut out);
        assert_eq!(&out[..n as usize], b"/root-fallback/etc/hostname");
    }

    #[test]
    fn malformed_pairs_are_skipped_not_fatal() {
        let env = env_of([b"PODBOX_MAPS=/good:/r,nocolon,/:/,:/empty-from,empty-to:\0"]);
        let t = unsafe { parse(env.as_ptr()) };
        // `/good:/r` and `/:/` parse; the other three do not.
        assert_eq!(t.n, 2);
    }

    #[test]
    fn the_memo_directory_is_never_rewritten() {
        let env = env_of([b"PODBOX_MAPS=/:/anywhere\0"]);
        let t = unsafe { parse(env.as_ptr()) };
        let mut out = [0u8; OUT];
        assert_eq!(rw(&t, b"/.podbox/ownership.memo", &mut out), -1);
        assert_eq!(rw(&t, b"/.podbox", &mut out), -1);
        // ⭐ But its siblings still map: the guard is the directory, not the
        // prefix string.
        let n = rw(&t, b"/.podbox2/x", &mut out);
        assert_eq!(&out[..n as usize], b"/anywhere/.podbox2/x");
    }

    #[test]
    fn absolutize_copies_absolute_paths_and_joins_the_cwd() {
        let mut out = [0u8; OUT];
        let n = unsafe { absolutize(AT_FDCWD, c"/x/y".as_ptr(), &mut out) };
        assert_eq!(&out[..n as usize], b"/x/y");
        // Relative joins the working directory this test runs in.
        let n = unsafe { absolutize(AT_FDCWD, c"rel".as_ptr(), &mut out) };
        assert!(n > 4, "{n}");
        assert_eq!(&out[n as usize - 4..n as usize], b"/rel");
        // A descriptor with nothing behind it has no answer.
        let n = unsafe { absolutize(9173, c"rel".as_ptr(), &mut out) };
        assert_eq!(n, -1);
    }
}
