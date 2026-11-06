//! The emulated operations, T-0708.
//!
//! Four calls the runtime cannot honour: `mknod` becomes a regular file,
//! `mount` becomes a table plus success, `unshare` becomes success and a
//! memo, `clone` loses its namespace flags before the real call. Every one
//! is a lie the payload is told, so each is counted: the tally rides the
//! ownership memo file (its `O_APPEND` discipline, its descriptor, its
//! fixed widths), and `inspect` reads it back.
//!
//! ⛔ **No emulation without a tally behind it.** Where the memo descriptor
//! was not handed, the wrappers fall back to the real call and its honest
//! failure: a success reported with nothing counted is the lie this object
//! exists to avoid, which is the memo's own rule.
//!
//! ⛔ **No allocation and no locks on this path.** Headers and chains are
//! stack buffers, one `writev` carries a whole mount chain: under `O_APPEND`
//! the kernel places it contiguously, so concurrent payload processes
//! cannot interleave one.

use core::ffi::{c_int, c_void};

/// Tally record kinds, in the memo record's `ino` field beside [`TALLY_DEV`].
pub const OP_MKNOD: u64 = 1;
pub const OP_MOUNT: u64 = 2;
pub const OP_UNSHARE: u64 = 3;
pub const OP_CLONE: u64 = 4;
/// T-0413: one served `/proc/self` answer (a descriptor duplicate, a
/// `readlink` synthesis, an exe passthrough, a mount-table serve). The
/// detail word names which: 1 fd open, 2 fd readlink, 3 exe readlink, 4
/// exe open, 5 mount-table serve.
pub const OP_PROC: u64 = 5;

/// The device number tally records carry. No file carries it: `dev_t` values
/// the kernel hands out are small, so the ownership scan skips these records
/// by mismatch and no lookup ever answers one.
pub const TALLY_DEV: u64 = u64::MAX;

/// Continuation records (mount path chains) carry ino 0 with 16 path bytes
/// in the four trailing words. A header names how many follow it; a chain
/// that breaks (EOF, or anything but a continuation) is dropped by the
/// reader rather than half-read.
pub const CONT: u64 = 0;

/// Namespace flags, UAPI `linux/sched.h`. Arch-generic: these values are
/// identical on every Linux architecture, and no payload runs anywhere else
/// (this crate refuses non-x86_64 at compile).
pub const CLONE_NEWNS: u32 = 0x00020000;
pub const CLONE_NEWCGROUP: u32 = 0x02000000;
pub const CLONE_NEWUTS: u32 = 0x04000000;
pub const CLONE_NEWIPC: u32 = 0x08000000;
pub const CLONE_NEWUSER: u32 = 0x10000000;
pub const CLONE_NEWPID: u32 = 0x20000000;
pub const CLONE_NEWNET: u32 = 0x40000000;
pub const CLONE_NEWTIME: u32 = 0x00000080;

/// Every namespace flag the wrapper strips.
pub const CLONE_NS_MASK: u32 = CLONE_NEWNS
    | CLONE_NEWCGROUP
    | CLONE_NEWUTS
    | CLONE_NEWIPC
    | CLONE_NEWUSER
    | CLONE_NEWPID
    | CLONE_NEWNET
    | CLONE_NEWTIME;

/// Strip every namespace flag, leaving the rest (`SIGCHLD`, `VM`, `FS`,
/// `FILES`, `SIGHAND`, `THREAD`, `SETTLS` and the rest) exactly as the
/// caller set them: shells and build systems set those speculatively, and
/// failing them stops payloads that would have worked with less isolation.
pub fn strip_ns(flags: u32) -> u32 {
    flags & !CLONE_NS_MASK
}

#[repr(C)]
#[derive(Clone, Copy)]
struct IoVec {
    base: *const c_void,
    len: usize,
}

extern "C" {
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    fn writev(fd: c_int, iov: *const IoVec, iovcnt: c_int) -> isize;
}

/// One tally header over `(op, d1, d2, d3)`, so the layout is built and
/// tested in one place and both writers below share it.
fn header(op: u64, d1: u32, d2: u32, d3: u32) -> [u8; crate::memo::RECORD] {
    let mut rec = [0u8; crate::memo::RECORD];
    rec[0..8].copy_from_slice(&TALLY_DEV.to_le_bytes());
    rec[8..16].copy_from_slice(&op.to_le_bytes());
    rec[16..20].copy_from_slice(&d1.to_le_bytes());
    rec[20..24].copy_from_slice(&d2.to_le_bytes());
    rec[24..28].copy_from_slice(&d3.to_le_bytes());
    rec
}

/// 16-byte chunks in `n` bytes, rounding up. An empty path takes none, and
/// the reader expects none where the header says none.
fn chunks(n: usize) -> usize {
    n.div_ceil(16)
}

/// Continuation records staged on the stack, T-0708.
///
/// Each continuation is a full 32-byte record (the tally words plus 16
/// path bytes), so the file stays fixed-width and the reader scans it
/// without parsing. A 16-byte-slice chain was tried first and died in the
/// drive: the reader walks 32-byte records and dropped every mount chain
/// it could not align to one.
const MAX_CONT: usize = 2 * crate::map::OUT.div_ceil(16);

/// Append one tally header `(op, d1, d2, d3)`, no chains.
///
/// Returns false where the memo descriptor was not handed: the caller then
/// falls back to the real call rather than reporting an uncounted success.
///
/// # Safety
/// None beyond the memo descriptor, which the host hands the payload.
pub fn tally(op: u64, d1: u32, d2: u32, d3: u32) -> bool {
    let Some(fd) = crate::memo::memo_fd() else {
        return false;
    };
    let rec = header(op, d1, d2, d3);
    let n = unsafe { write(fd, rec.as_ptr() as *const c_void, crate::memo::RECORD) };
    n == crate::memo::RECORD as isize
}

/// Stage `n` bytes at `p` as 32-byte continuation records in `area` from
/// `at`, returning the records used. Every byte of every record is
/// written, so no stale stack reads whatever the caller passed before.
///
/// # Safety
/// `p` must be readable for `n` bytes, `area` writable past the records
/// used. Past `MAX_CONT` the function stops, and the writer compares
/// against `chunks` and refuses the tally rather than logging it half.
unsafe fn stage_chain(area: &mut [[u8; 32]], at: usize, p: *const u8, n: usize) -> usize {
    let mut used = 0usize;
    let mut off = 0usize;
    while off < n && at + used < MAX_CONT {
        let mut rec = [0u8; 32];
        rec[0..8].copy_from_slice(&TALLY_DEV.to_le_bytes());
        // rec[8..16] stays zero: a continuation.
        let take = core::cmp::min(16, n - off);
        rec[16..16 + take]
            .copy_from_slice(unsafe { core::slice::from_raw_parts(p.add(off), take) });
        area[at + used] = rec;
        used += 1;
        off += take;
    }
    if off < n {
        return 0;
    }
    used
}

/// Append a mount tally: one header naming both chain lengths, then the
/// source and target as continuation records, in a single `writev` so
/// concurrent payload processes cannot interleave a chain.
///
/// Returns false where the memo descriptor was not handed, or where the
/// chains would overflow the staging area (unreachable under the rewrite
/// buffers, refused rather than truncated).
///
/// # Safety
/// `src_p` must be readable for `src_n` bytes, `tgt_p` for `tgt_n`.
pub unsafe fn tally_mount(
    flags: u32,
    src_p: *const u8,
    src_n: usize,
    tgt_p: *const u8,
    tgt_n: usize,
) -> bool {
    let Some(fd) = crate::memo::memo_fd() else {
        return false;
    };
    let cs = chunks(src_n);
    let ct = chunks(tgt_n);
    if 1 + cs + ct > 1 + MAX_CONT {
        return false;
    }
    let hdr = header(OP_MOUNT, flags, cs as u32, ct as u32);
    let mut area = [[0u8; 32]; MAX_CONT];
    let us = unsafe { stage_chain(&mut area, 0, src_p, src_n) };
    if us != cs {
        return false;
    }
    let ut = unsafe { stage_chain(&mut area, us, tgt_p, tgt_n) };
    if ut != ct {
        return false;
    }
    let mut iov = [IoVec {
        base: core::ptr::null(),
        len: 0,
    }; 1 + MAX_CONT];
    iov[0] = IoVec {
        base: hdr.as_ptr() as *const c_void,
        len: crate::memo::RECORD,
    };
    let mut k = 0usize;
    while k < us + ut {
        iov[1 + k] = IoVec {
            base: area[k].as_ptr() as *const c_void,
            len: 32,
        };
        k += 1;
    }
    let total = (1 + us + ut) as c_int;
    let want = crate::memo::RECORD + (us + ut) * 32;
    let n = unsafe { writev(fd, iov.as_ptr(), total) };
    n == want as isize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strip_set_is_the_eight_namespace_flags() {
        assert_eq!(CLONE_NS_MASK, 0x7E020080);
        assert_eq!(CLONE_NS_MASK & CLONE_NEWNET, CLONE_NEWNET);
    }

    #[test]
    fn stripping_keeps_everything_but_namespaces() {
        // SIGCHLD (17) plus a namespace flag: only the flag goes.
        assert_eq!(strip_ns(0x00020011), 0x00000011);
        // All eight at once leave nothing.
        assert_eq!(strip_ns(CLONE_NS_MASK), 0);
        // Nothing in, nothing out.
        assert_eq!(strip_ns(0x00000011), 0x00000011);
        // High bits the mask does not name survive the strip.
        assert_eq!(strip_ns(0x80000000), 0x80000000);
    }

    #[test]
    fn the_header_is_a_fixed_32_byte_record() {
        let h = header(OP_MOUNT, 0x20, 2, 3);
        assert_eq!(u64::from_le_bytes(h[0..8].try_into().unwrap()), u64::MAX);
        assert_eq!(u64::from_le_bytes(h[8..16].try_into().unwrap()), OP_MOUNT);
        assert_eq!(u32::from_le_bytes(h[16..20].try_into().unwrap()), 0x20);
        assert_eq!(u32::from_le_bytes(h[20..24].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(h[24..28].try_into().unwrap()), 3);
        assert_eq!(&h[28..32], &[0, 0, 0, 0]);
    }

    #[test]
    fn chunk_math_rounds_up_and_empties_take_none() {
        assert_eq!(chunks(0), 0);
        assert_eq!(chunks(1), 1);
        assert_eq!(chunks(16), 1);
        assert_eq!(chunks(17), 2);
        assert_eq!(chunks(4096), 256);
    }

    #[test]
    fn chains_stage_as_full_records_with_tally_words() {
        let path = b"/some/mount/source";
        let mut area = [[0u8; 32]; 4];
        let used = unsafe { stage_chain(&mut area, 0, path.as_ptr(), path.len()) };
        // 18 bytes: two 32-byte records, each carrying the tally words.
        assert_eq!(used, 2);
        assert_eq!(
            u64::from_le_bytes(area[0][0..8].try_into().unwrap()),
            u64::MAX
        );
        assert_eq!(u64::from_le_bytes(area[0][8..16].try_into().unwrap()), 0);
        assert_eq!(&area[0][16..32], b"/some/mount/sour");
        assert_eq!(&area[1][16..18], b"ce");
        assert_eq!(&area[1][18..32], &[0u8; 14]);
        // An empty path stages nothing, and the header claims none.
        assert_eq!(chunks(0), 0);
    }
}
