//! Serve `open` of a `--device` guest path, TODO/enter.md T-0501.
//!
//! The entering process opens the host path before the `chroot` and hands
//! the child the descriptor at a fixed number, with the guest spelling
//! beside it in `PODBOX_DEVICE_FDS` (`guest:childfd:perms` rows joined by
//! `\x1e`). An open that names the exact guest spelling and fails `ENOENT`
//! (the image holds no node there, because none can be made) is answered
//! here with a duplicate of that descriptor, after the access-mode check
//! against the row's perms. Anything the row does not name exactly falls
//! through to the kernel's failure: a creation or truncation flag, a status
//! flag the shared description does not hold, or a malformed row is never
//! approximated, because a duplicate that is almost the open asked for is
//! the finding in T-0413's shape wearing a new flag.
//!
//! The access answer is exact where a duplicate is one: a read the row
//! does not grant is `EACCES`, exactly as a node with those bits would
//! answer. A malformed row is skipped rather than failed: the hook must
//! never fail a path it cannot exactly serve, and the table is built by
//! tested code (`podbox-enter`'s `device` module) whose own tests pin both
//! directions.

use core::ffi::c_int;

/// Linux `O_*`, UAPI `asm-generic/fcntl.h`. Duplicated as literals rather
/// than depended on: this object takes no dependency on the rest of the
/// tree, and the values are kernel ABI, not project logic.
const O_ACCMODE: c_int = 0o3;
const O_RDONLY: c_int = 0o0;
const O_WRONLY: c_int = 0o1;
const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_EXCL: c_int = 0o200;
const O_TRUNC: c_int = 0o1000;
const O_APPEND: c_int = 0o2000;
const O_NONBLOCK: c_int = 0o4000;
const O_DSYNC: c_int = 0o10000;
const O_DIRECT: c_int = 0o40000;
const O_LARGEFILE: c_int = 0o100000;
const O_DIRECTORY: c_int = 0o200000;
const O_NOFOLLOW: c_int = 0o400000;
const O_CLOEXEC: c_int = 0o2000000;
const O_SYNC: c_int = 0o4010000;
const O_PATH: c_int = 0o10000000;
const O_TMPFILE: c_int = 0o20000000 | O_DIRECTORY;
const O_NOCTTY: c_int = 0o400;

/// File-type bits, UAPI `linux/stat.h`.
const S_IFCHR: u32 = 0o20000;
const S_IFBLK: u32 = 0o60000;

/// `EEXIST`, UAPI `asm-generic/errno.h`. Creating what the mapping already
/// names fails exactly as a kernel creating over an existing node does.
const EEXIST: c_int = 17;

/// Record separator between serve rows.
const SEP_ENTRY: u8 = 0x1e;

/// The serve variable the entering process builds.
pub const SERVE_VAR: &[u8] = b"PODBOX_DEVICE_FDS";

/// What answering an open may do. `Dup(fd)` duplicates the descriptor;
/// `Fail(e)` is the kernel's exact answer where no duplicate would be one;
/// `Pass` falls through to the real call and its honest failure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ServeAnswer {
    Dup(c_int),
    Fail(c_int),
    Pass,
}

/// The row naming `guest` (NUL-terminated caller bytes), with its
/// descriptor and perms, or `None`. Malformed rows are skipped: see the
/// module note for why the hook never fails what it cannot serve.
pub fn find(table: &[u8], guest: &[u8]) -> Option<(c_int, [u8; 2], usize)> {
    let want = strip_nul(guest)?;
    for row in table.split(|b| *b == SEP_ENTRY) {
        if row.is_empty() {
            continue;
        }
        let Some((g, rest)) = split_colon(row) else {
            continue;
        };
        if g != want {
            continue;
        }
        let Some((fds, perms)) = split_colon(rest) else {
            continue;
        };
        let Ok(fd) = parse_fd(fds) else {
            continue;
        };
        if perms.is_empty() || perms.len() > 2 || !perms.iter().all(|b| *b == b'r' || *b == b'w') {
            continue;
        }
        let mut p = [0u8; 2];
        p[..perms.len()].copy_from_slice(perms);
        return Some((fd, p, perms.len()));
    }
    None
}

/// The flag transformation a duplicate may carry: T-0413's `open_answer`
/// rule, restated for a descriptor rather than a reopen. A duplicate
/// shares the file description, so a path flag (`O_PATH`, `O_TMPFILE`,
/// `O_NOFOLLOW`) has no meaning on it and always falls through, the
/// ignored set is masked first, a directory flag on it is `ENOTDIR`, a
/// status flag the description does not hold falls through (setting it
/// would mutate the shared description), and an access mode the row does
/// not grant is `EACCES`, checked before anything below: the kernel
/// answers access before creation too.
///
/// Creation is then the node's own semantics, not the filesystem's. The
/// mapping names a node that exists, so creating over it is a no-op where
/// the kernel makes it one: `O_CREAT` without `O_EXCL` duplicates, and
/// `O_EXCL` with `O_CREAT` fails `EEXIST`, exactly. `O_TRUNC` duplicates
/// where truncating is a no-op (a device), and falls through where no
/// duplicate reproduces it (anything else: truncating by duplicate would
/// leave the host bytes whole while reporting success). Where the row
/// serves nothing the call falls through to the kernel's own answer for
/// the image path, which a creation then makes rather than the device.
pub fn answer(
    fd: c_int,
    orig_flags: c_int,
    req_flags: c_int,
    perms: &[u8],
    is_dev: bool,
) -> ServeAnswer {
    use crate::procfs::{EACCES, ENOTDIR};
    // `O_TMPFILE` embeds `O_DIRECTORY`, so it is tested by its
    // distinguishing bit first: matching the directory arm would answer
    // `ENOTDIR` for a shape the kernel creates unnamed in the parent.
    const TMPFILE_BIT: c_int = O_TMPFILE & !O_DIRECTORY;
    const REFUSED: c_int = O_PATH | TMPFILE_BIT | O_NOFOLLOW;
    if req_flags & REFUSED != 0 {
        return ServeAnswer::Pass;
    }
    let req_flags = req_flags & !(O_NOCTTY | O_LARGEFILE | O_CLOEXEC);
    if req_flags & O_DIRECTORY != 0 {
        return ServeAnswer::Fail(ENOTDIR);
    }
    const SHARED: c_int = O_APPEND | O_NONBLOCK | O_DIRECT | O_SYNC | O_DSYNC;
    if req_flags & SHARED != orig_flags & SHARED {
        return ServeAnswer::Pass;
    }
    let req_acc = req_flags & O_ACCMODE;
    let need_read = req_acc == O_RDONLY || req_acc == O_RDWR;
    let need_write = req_acc == O_WRONLY || req_acc == O_RDWR;
    if need_read && !perms.contains(&b'r') {
        return ServeAnswer::Fail(EACCES);
    }
    if need_write && !perms.contains(&b'w') {
        return ServeAnswer::Fail(EACCES);
    }
    if req_flags & O_EXCL != 0 && req_flags & O_CREAT != 0 {
        return ServeAnswer::Fail(EEXIST);
    }
    if req_flags & O_CREAT != 0 && !is_dev {
        return ServeAnswer::Pass;
    }
    if req_flags & O_TRUNC != 0 && !is_dev {
        return ServeAnswer::Pass;
    }
    ServeAnswer::Dup(fd)
}

fn strip_nul(path: &[u8]) -> Option<&[u8]> {
    if path.len() < 2 || path[path.len() - 1] != 0 {
        return None;
    }
    Some(&path[..path.len() - 1])
}

fn split_colon(row: &[u8]) -> Option<(&[u8], &[u8])> {
    let i = row.iter().position(|b| *b == b':')?;
    Some((&row[..i], &row[i + 1..]))
}

fn parse_fd(fds: &[u8]) -> Result<c_int, ()> {
    if fds.is_empty() || fds.len() > 6 {
        return Err(());
    }
    let mut v: c_int = 0;
    for c in fds {
        if !c.is_ascii_digit() {
            return Err(());
        }
        v = v
            .checked_mul(10)
            .and_then(|v| v.checked_add((c - b'0') as c_int))
            .ok_or(())?;
    }
    Ok(v)
}

/// Serve `open` of a device guest path after the real call failed
/// `ENOENT`, T-0501. Same gate as the `/proc/self` emulation: only the
/// exact spelling, only where the mapping table left the path untouched,
/// and a deliberate mapping always wins.
///
/// # Safety
/// The payload's own contract for this entry point: `path` is either null
/// or points at a NUL-terminated byte string this process may read, and
/// the process's environ is the kernel's.
pub unsafe fn serve_open(path: *const core::ffi::c_char, flags: c_int) -> Option<c_int> {
    if path.is_null() {
        return None;
    }
    let n = crate::map::strnlen(path, crate::map::OUT);
    if n == 0 || n >= crate::map::OUT {
        return None;
    }
    let bytes = unsafe { core::slice::from_raw_parts(path as *const u8, n + 1) };
    let (table, tlen) = unsafe { crate::map::lookup_env(SERVE_VAR) }?;
    if tlen == 0 {
        return None;
    }
    let table = unsafe { core::slice::from_raw_parts(table, tlen) };
    let (fd, perms_buf, perms_len) = find(table, bytes)?;
    let perms = &perms_buf[..perms_len];
    let Some(orig) = (unsafe { crate::procfs::open_flags(fd) }) else {
        // The descriptor is gone: fall through, and the kernel's `ENOENT`
        // stands. A stale table fails like an absent path, never louder.
        return None;
    };
    // Creation over the mapping is the node's own no-op, but only where
    // the descriptor IS a device: truncating or creating over anything
    // else by duplicate would report success while leaving the host bytes
    // whole, so those fall through. A failed `fstat` is the conservative
    // answer: not a device, and creation falls through with it.
    let mut st = [0u8; crate::procfs::STAT_LEN];
    let is_dev = unsafe { crate::proc_fstat(fd as u32, &mut st) } && {
        let m = crate::procfs::mode_of(&st);
        m & crate::procfs::S_IFMT == S_IFCHR || m & crate::procfs::S_IFMT == S_IFBLK
    };
    match answer(fd, orig, flags, perms, is_dev) {
        ServeAnswer::Dup(source) => {
            let dup = unsafe { crate::procfs::dup_of(source, flags & O_CLOEXEC != 0) };
            if dup < 0 {
                return Some(-1);
            }
            Some(dup)
        }
        ServeAnswer::Fail(e) => {
            crate::set_errno(e);
            Some(-1)
        }
        ServeAnswer::Pass => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Vec<u8> {
        b"/dev/hostzero:41:r\x1e/dev/hostnull:42:rw".to_vec()
    }

    #[test]
    fn find_matches_exact_spelling() {
        let (fd, p, n) = find(&table(), b"/dev/hostzero\0").unwrap();
        assert_eq!(fd, 41);
        assert_eq!(&p[..n], b"r");
        let (fd, p, n) = find(&table(), b"/dev/hostnull\0").unwrap();
        assert_eq!(fd, 42);
        assert_eq!(&p[..n], b"rw");
    }

    #[test]
    fn find_rejects_near_spellings() {
        for bad in [
            b"/dev/hostzero/\0".as_slice(),
            b"/dev/hostzer\0",
            b"/dev/hostzero\0extra",
            b"dev/hostzero\0",
            b"/dev/other\0",
            b"\0",
        ] {
            assert!(find(&table(), bad).is_none(), "{bad:?} must not serve");
        }
    }

    #[test]
    fn find_skips_malformed_rows() {
        let t = b"no-colons\x1e/dev/g:xy:r\x1e/dev/g2:notanfd:r\x1e/dev/ok:43:w".to_vec();
        let (fd, _, _) = find(&t, b"/dev/ok\0").unwrap();
        assert_eq!(fd, 43);
        assert!(find(&t, b"no-colons\0").is_none());
    }

    #[test]
    fn answer_checks_perms_by_access_mode() {
        use crate::procfs::EACCES;
        // A read-only row serves reads and refuses writes with EACCES.
        assert_eq!(answer(41, 0, O_RDONLY, b"r", false), ServeAnswer::Dup(41));
        assert_eq!(
            answer(41, 0, O_WRONLY, b"r", false),
            ServeAnswer::Fail(EACCES)
        );
        assert_eq!(
            answer(41, 0, O_RDWR, b"r", false),
            ServeAnswer::Fail(EACCES)
        );
        assert_eq!(answer(41, 0, O_RDONLY, b"rw", false), ServeAnswer::Dup(41));
        assert_eq!(answer(41, 0, O_RDWR, b"rw", false), ServeAnswer::Dup(41));
        // Access is answered before creation: a write the row does not
        // grant fails even where creating would be a no-op.
        assert_eq!(
            answer(41, 0, O_WRONLY | O_CREAT, b"r", true),
            ServeAnswer::Fail(EACCES)
        );
    }

    #[test]
    fn answer_falls_through_where_no_duplicate_reproduces() {
        // Path and tmpfile shapes fall through.
        for f in [O_PATH, O_TMPFILE, O_NOFOLLOW] {
            assert_eq!(answer(41, 0, O_RDONLY | f, b"rw", true), ServeAnswer::Pass);
        }
        // A status flag the description does not hold falls through;
        // one it holds passes the flag check.
        assert_eq!(
            answer(41, 0, O_RDONLY | O_NONBLOCK, b"rw", true),
            ServeAnswer::Pass
        );
        assert_eq!(
            answer(41, O_NONBLOCK, O_RDONLY | O_NONBLOCK, b"rw", true),
            ServeAnswer::Dup(41)
        );
    }

    #[test]
    fn answer_serves_creation_as_the_node_itself() {
        // The mapping names a node that exists: creating over it is a
        // no-op where the kernel makes it one (a device), exclusive
        // creation fails EEXIST, and anywhere else the call falls
        // through to the kernel's own answer for the image path.
        assert_eq!(
            answer(41, 0, O_WRONLY | O_CREAT, b"rw", true),
            ServeAnswer::Dup(41)
        );
        assert_eq!(
            answer(41, 0, O_WRONLY | O_CREAT, b"rw", false),
            ServeAnswer::Pass
        );
        assert_eq!(
            answer(41, 0, O_WRONLY | O_CREAT | O_EXCL, b"rw", true),
            ServeAnswer::Fail(EEXIST)
        );
        assert_eq!(
            answer(41, 0, O_WRONLY | O_TRUNC, b"rw", true),
            ServeAnswer::Dup(41)
        );
        assert_eq!(
            answer(41, 0, O_WRONLY | O_TRUNC, b"rw", false),
            ServeAnswer::Pass
        );
    }
}
