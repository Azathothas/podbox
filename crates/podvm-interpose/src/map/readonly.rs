//! The read-only layer of the path table. TODO/cli.md T-1655.
//!
//! `PODVM_RO` lists guest prefixes, each `ro:PREFIX` or `rw:PREFIX`, comma
//! separated: `run --read-only` gives `ro:/` with an `rw:` entry for each
//! writable volume and `--tmpfs`, and a `-v HOST:GUEST:ro` volume gives
//! `ro:GUEST`. The longest prefix decides; a path under no entry is
//! writable. A call that changes a path under a read-only prefix answers
//! `EROFS`, as a read-only mount does. ⚠ The interposer is not a boundary:
//! a raw system call passes, and a descriptor-relative path that the table
//! cannot resolve passes unchanged.

use super::*;

/// The variable of the read-only table.
pub const RO_VAR: &[u8] = b"PODVM_RO";

/// `EROFS`: a write to a read-only file system.
pub const EROFS: c_int = 30;

/// The calls that change the path they name, whatever their flags. `open`
/// and `fopen` change it only where their flags or mode write, so their
/// wrappers say so through [`prepare_mode`].
pub fn is_write_call(what: &[u8]) -> bool {
    matches!(
        what,
        b"unlink"
            | b"unlinkat"
            | b"rmdir"
            | b"remove"
            | b"mkdir"
            | b"mkdirat"
            | b"chmod"
            | b"fchmodat"
            | b"truncate"
            | b"truncate64"
            | b"utime"
            | b"utimes"
            | b"utimensat"
            | b"futimesat"
            | b"lutimes"
            | b"setxattr"
            | b"lsetxattr"
            | b"removexattr"
            | b"lremovexattr"
            | b"creat"
            | b"creat64"
            | b"mknod"
            | b"mknodat"
            | b"mkfifo"
            | b"mkfifoat"
            | b"symlink"
            | b"symlinkat"
            | b"link"
            | b"linkat"
            | b"rename"
            | b"renameat"
            | b"renameat2"
            | b"chown"
            | b"lchown"
            | b"fchownat"
    )
}

/// True where `open` flags write: an access mode other than read only, or
/// a create, a truncate or an append.
pub fn open_writes(flags: c_int) -> bool {
    const O_ACCMODE: c_int = 0o3;
    const O_CREAT: c_int = 0o100;
    const O_TRUNC: c_int = 0o1000;
    const O_APPEND: c_int = 0o2000;
    flags & O_ACCMODE != 0 || flags & (O_CREAT | O_TRUNC | O_APPEND) != 0
}

/// True where an `fopen` mode writes: `w`, `a` or `+`.
///
/// # Safety
/// `mode` is null or a C string.
pub unsafe fn mode_writes(mode: *const c_char) -> bool {
    if mode.is_null() {
        return false;
    }
    let n = strnlen(mode, 16);
    let bytes = unsafe { core::slice::from_raw_parts(mode as *const u8, n) };
    bytes.iter().any(|c| matches!(c, b'w' | b'a' | b'+'))
}

/// True where the table of `PODVM_RO` makes the guest path `path` read
/// only: the longest matching prefix is an `ro:` entry.
///
/// # Safety
/// `path` must be readable for `path_n` bytes.
pub unsafe fn denied(path: *const u8, path_n: usize) -> bool {
    let Some((v, v_n)) = (unsafe { lookup_env(RO_VAR) }) else {
        return false;
    };
    let table = unsafe { core::slice::from_raw_parts(v, v_n) };
    denied_in(table, unsafe { core::slice::from_raw_parts(path, path_n) })
}

/// [`denied`] over a given table, for the tests.
pub fn denied_in(table: &[u8], path: &[u8]) -> bool {
    let mut best: Option<(usize, bool)> = None;
    for entry in table.split(|c| *c == b',') {
        let (ro, prefix_bytes) = if let Some(p) = entry.strip_prefix(b"ro:") {
            (true, p)
        } else if let Some(p) = entry.strip_prefix(b"rw:") {
            (false, p)
        } else {
            continue;
        };
        let mut n = prefix_bytes.len();
        if n > 1 && prefix_bytes[n - 1] == b'/' {
            n -= 1;
        }
        if n == 0 || prefix_bytes[0] != b'/' {
            continue;
        }
        if prefix(prefix_bytes.as_ptr(), n, path.as_ptr(), path.len())
            && best.is_none_or(|(b, _)| n > b)
        {
            best = Some((n, ro));
        }
    }
    best.is_some_and(|(_, ro)| ro)
}
