//! The path table and the rewrite, for T-0703.
//!
//! [`TODO/interpose.md`](../../../TODO/interpose.md) T-0703. Every path-taking
//! entry point in `lib.rs` funnels through [`prepare`]: absolutize the path,
//! match it against the table, and hand back the pointer to call with.
//!
//! ⭐ **The table comes from the environment, as one variable.** `PODVM_MAPS`
//! carries `FROM:TO[,FROM:TO...]` pairs, which mirrors pathmap's `PATH_MAPPING`
//! syntax without sharing its name: one word, one meaning, and podvm never
//! reads another tool's variable. The userland rung sets it: the root pair
//! of T-1407, then one pair for each `-v HOST:GUEST` (TODO/enter.md T-1653),
//! which the longest match below prefers to the root pair. Where it is
//! unset, every call takes the empty-table fast path below. T-0707's
//! never-rewrite paths ride beside it: the built-in `/proc` tree in code,
//! plus a colon-separated user list in `PODVM_EXCLUDE_PATH`.
//!
//! ⛔ **No allocation and no locks on this path.** The table borrows the
//! process's own environ memory as `(pointer, length)` pairs on the caller's
//! stack, so parsing costs a walk and no bytes. Rewriting writes into the
//! caller's 4 KiB stack buffer.

use core::ffi::{c_char, c_int};

unsafe extern "C" {
    static environ: *const *const c_char;
}

/// The variable that carries the table.
pub const MAPS_VAR: &[u8] = b"PODVM_MAPS";

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

crate::real!(pub(crate) fn next_getcwd = "getcwd"(*mut c_char, usize) -> *mut c_char);
crate::real!(pub(crate) fn next_readlink = "readlink"(*const c_char, *mut c_char, usize) -> isize);

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
pub(crate) fn match_var(e: *const c_char, name: &[u8]) -> Option<*const u8> {
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
/// `None` where the variable is absent. Used for the small podvm-owned
/// variables (`PODVM_MAPS` goes through [`parse`]; this serves the rest)
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
        // `PODVM_MAPS=` then the pairs.
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

mod absolute;
pub use absolute::normalise;
use absolute::{fd_base, guest_base};
mod rewrite;
pub use rewrite::*;
mod readonly;
pub use readonly::*;

#[cfg(test)]
mod tests;
