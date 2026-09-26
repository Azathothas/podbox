//! `/proc/self` emulation where no procfs is mounted, T-0413.
//!
//! The chroot entry mounts nothing, so `/proc/self/fd/N`, `/proc/self/exe`
//! and the mount-table files fail with `ENOENT`. Three of those have exact
//! answers without a kernel filesystem, and one has an allowed fixture:
//!
//! * `/proc/self/fd/N` (and the `/dev/fd/N` spelling bash passes around):
//!   a pipe the process holds is answered from `fstat`/`fstatfs` live, so a
//!   descriptor the kernel tracks is never described from a static file. A
//!   socket is answered with the kernel's own `ENXIO`, which is what opening
//!   one through these paths says on a live `/proc`. Anything else is passed
//!   through to the honest failure: emulation must be exact or refused.
//! * `/proc/self/exe`: the caller-resolved guest path ([`GUEST_EXE_VAR`]),
//!   set beside the exec by the side that owns the rootfs host path. Tried
//!   only after the real call fails, so a truth the kernel still answers
//!   (no chroot, host `/proc` mounted) always wins over this variable.
//! * The mount-table files (`/proc/mounts`, `/proc/self/mounts`,
//!   `/proc/self/mountinfo`, `/etc/mtab`): a generated fixture, which is the
//!   half `TOOL.md` section 10 allows ("a static fixture can satisfy a
//!   program that only reads a mount table, and nothing more"). The `/`
//!   line is generated from live `stat`/`statfs` of the payload's root --
//!   real device numbers, never shipped ones -- and the recorded T-0708
//!   emulated mounts render behind it with a `0:0` device, because an
//!   emulated mount has none. Anything needing a live interface beyond a
//!   table read gets the fixture's approximation declared as such, or the
//!   honest failure where no descriptor was handed.
//!
//! ⛔ **Counted like T-0708.** Every served answer tallies
//! [`crate::emulate::OP_PROC`] through the memo descriptor with what it
//! served in the detail word. No tally behind it, no emulation: the
//! wrappers fall back to the real call and its honest failure.
//!
//! ⛔ **No allocation and no locks on this path**, T-0701's constraint 2.
//! Every buffer here is on the caller's stack, including the fixture.

use core::ffi::{c_int, c_uint, c_void};

/// The variable carrying the payload's resolved guest path into the payload.
///
/// Set beside the exec by `podbox-cli` (which owns the rootfs host path),
/// read here with no libc call, like `map` and the memo do. A duplicate of
/// `podbox-enter`'s `ladder::GUEST_EXE_VAR` by value, the same way
/// `memo::MEMO_FD_VAR` duplicates the host's spelling: this object takes no
/// dependency on the rest of the tree.
pub const GUEST_EXE_VAR: &[u8] = b"PODBOX_GUEST_EXE";

extern "C" {
    fn fstatfs(fd: c_int, buf: *mut c_void) -> c_int;
    fn fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
}

/// `F_SETFD`, with `FD_CLOEXEC`: the one flag a fresh serve carries that the
/// request may not have asked for. UAPI `asm-generic/fcntl.h`, arch-generic.
const F_SETFD: c_int = 2;
const FD_CLOEXEC: c_int = 1;

/// The filesystem magic holding `fd`, or `None`.
///
/// Raw `fstatfs`, not the interposed `statfs` (which rewrites paths and
/// reports the memo): a descriptor needs no path and no report.
pub(crate) unsafe fn fs_magic(fd: c_int) -> Option<u64> {
    let mut fs = [0u8; STATFS_LEN];
    if unsafe { fstatfs(fd, fs.as_mut_ptr() as *mut c_void) } != 0 {
        return None;
    }
    Some(u64::from_ne_bytes(fs[0..8].try_into().unwrap_or([0u8; 8])))
}

/// The open flags `fd` was opened with, or `None`. Raw `fcntl`: the
/// interposer exports no `fcntl` wrapper, so there is nothing to recurse
/// into, and the number is the kernel's on every libc this object loads
/// against.
pub(crate) unsafe fn open_flags(fd: c_int) -> Option<c_int> {
    let r = unsafe { fcntl(fd, F_GETFL, 0) };
    if r < 0 {
        None
    } else {
        Some(r)
    }
}

/// Duplicate `fd`, close-on-exec where asked, or -1 with the kernel's errno.
pub(crate) unsafe fn dup_of(fd: c_int, cloexec: bool) -> c_int {
    unsafe { fcntl(fd, if cloexec { F_DUPFD_CLOEXEC } else { F_DUPFD }, 0) }
}

/// Set or clear close-on-exec on `fd`.
pub(crate) unsafe fn set_cloexec(fd: c_int, on: bool) -> bool {
    unsafe { fcntl(fd, F_SETFD, if on { FD_CLOEXEC } else { 0 }) == 0 }
}

/// Write the whole buffer, short writes retried. `false` on any error.
pub(crate) unsafe fn write_all(fd: c_int, buf: &[u8]) -> bool {
    let mut at = 0usize;
    while at < buf.len() {
        let n = unsafe { write(fd, buf[at..].as_ptr() as *const c_void, buf.len() - at) };
        if n <= 0 {
            return false;
        }
        at += n as usize;
    }
    true
}

/// Rewind to the start. `0` is `SEEK_SET` on every Linux architecture.
pub(crate) unsafe fn rewind(fd: c_int) -> bool {
    (unsafe { lseek(fd, 0, 0) }) == 0
}

pub const ENOENT: c_int = 2;
pub const EACCES: c_int = 13;
pub const ENXIO: c_int = 6;
pub const ENOTDIR: c_int = 20;

const O_ACCMODE: c_int = 0o3;
const O_RDONLY: c_int = 0o0;
const O_WRONLY: c_int = 0o1;
const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_EXCL: c_int = 0o200;
const O_NOCTTY: c_int = 0o400;
const O_TRUNC: c_int = 0o1000;
const O_APPEND: c_int = 0o2000;
const O_NONBLOCK: c_int = 0o4000;
const O_DSYNC: c_int = 0o10000;
const O_DIRECT: c_int = 0o40000;
const O_LARGEFILE: c_int = 0o100000;
const O_DIRECTORY: c_int = 0o200000;
const O_NOFOLLOW: c_int = 0o400000;
pub const O_CLOEXEC: c_int = 0o2000000;
const O_SYNC: c_int = 0o4010000;
const O_PATH: c_int = 0o10000000;
const O_TMPFILE: c_int = 0o20000000 | O_DIRECTORY;

pub const F_DUPFD: c_int = 0;
pub const F_DUPFD_CLOEXEC: c_int = 1030;
pub const F_GETFL: c_int = 3;

pub const S_IFMT: c_uint = 0o170000;
pub const S_IFIFO: c_uint = 0o10000;
pub const S_IFSOCK: c_uint = 0o140000;

/// `PIPEFS_MAGIC`, UAPI `linux/magic.h`. A fifo `fstat` cannot tell a pipe
/// from a named fifo (both are `S_IFIFO`), and answering `pipe:[ino]` for a
/// named fifo would be wrong where it looks right. The filesystem type
/// tells them apart: only pipefs inodes render as `pipe:[ino]`.
pub const PIPEFS_MAGIC: u64 = 0x5049_5045;

/// `struct stat` field offsets on x86_64, the same measurement `lib.rs`
/// carries: `dev=0 ino=8 mode=24`, identical under both libcs.
const ST_INO: usize = 8;
const ST_MODE: usize = 24;

/// `struct statfs` on x86_64 is kernel ABI, identical under both libcs:
/// `f_type` a `long` at 0, `f_flags` a `long` at 80, 120 bytes whole.
pub const STATFS_LEN: usize = 120;
pub const STATFS_FLAGS: usize = 80;
pub const ST_RDONLY: u64 = 1;

/// `struct stat` on x86_64, 144 bytes whole. Only the header travels here:
/// the glue fills it with the real `fstat` and these read it back.
pub const STAT_LEN: usize = 144;

/// Split device numbers back out of `st_dev`.
///
/// ⛔ The inverse of the `makedev` this crate and `podbox-probe` share,
/// duplicated as three lines rather than depended on: this object takes no
/// dependency on the rest of the tree, and the formula is kernel ABI, not
/// project logic.
pub fn major_of(dev: u64) -> u32 {
    (((dev >> 8) & 0xfff) | ((dev >> 32) & !0xfff)) as u32
}

pub fn minor_of(dev: u64) -> u32 {
    ((dev & 0xff) | ((dev >> 12) & !0xff)) as u32
}

pub fn mode_of(st: &[u8; STAT_LEN]) -> u32 {
    u32::from_ne_bytes([
        st[ST_MODE],
        st[ST_MODE + 1],
        st[ST_MODE + 2],
        st[ST_MODE + 3],
    ])
}

pub fn ino_of(st: &[u8; STAT_LEN]) -> u64 {
    u64::from_ne_bytes([
        st[ST_INO],
        st[ST_INO + 1],
        st[ST_INO + 2],
        st[ST_INO + 3],
        st[ST_INO + 4],
        st[ST_INO + 5],
        st[ST_INO + 6],
        st[ST_INO + 7],
    ])
}

/// A `/proc/self/fd/N` or `/dev/fd/N` spelling with its descriptor number.
///
/// Absolute only, digits only, NUL-terminated right after the digits: a
/// trailing slash is `ENOTDIR` on a live `/proc`, not a descriptor, so it
/// never matches here and falls through. Relative spellings cannot name
/// these leaves without a `/proc` descriptor the payload cannot hold where
/// no procfs is mounted, so they fall through too. `u32::MAX` is past any
/// real descriptor table and is refused rather than wrapped.
///
/// The `/dev/stdin`, `/dev/stdout` and `/dev/stderr` spellings name
/// descriptors 0, 1 and 2 by definition, so they match exactly: resolving
/// them through the `/proc/self/fd` symlink they name needs no procfs
/// once the number is known.
pub fn fd_number(path: &[u8]) -> Option<u32> {
    const A: &[u8] = b"/proc/self/fd/";
    const B: &[u8] = b"/dev/fd/";
    if path == b"/dev/stdin\0" {
        return Some(0);
    }
    if path == b"/dev/stdout\0" {
        return Some(1);
    }
    if path == b"/dev/stderr\0" {
        return Some(2);
    }
    let rest = if path.starts_with(A) {
        &path[A.len()..]
    } else if path.starts_with(B) {
        &path[B.len()..]
    } else {
        return None;
    };
    if rest.is_empty() {
        return None;
    }
    let mut v: u64 = 0;
    for (i, c) in rest.iter().enumerate() {
        if *c == 0 {
            if i == 0 {
                return None;
            }
            if v > c_int::MAX as u64 {
                return None;
            }
            return Some(v as u32);
        }
        if !c.is_ascii_digit() {
            return None;
        }
        v = v.checked_mul(10)?.checked_add((c - b'0') as u64)?;
        if v > c_int::MAX as u64 {
            return None;
        }
    }
    None
}

/// Exactly `/proc/self/exe`, NUL-terminated. Anything longer (`exe/`, a
/// sibling leaf) falls through to the honest failure.
pub fn is_self_exe(path: &[u8]) -> bool {
    path == b"/proc/self/exe\0"
}

/// Which mount-table file a path names, if any.
///
/// `/proc/self/mounts` is the kernel's own alias of `/proc/mounts`, so the
/// two share a rendering. A user table that rewrote one of these wins over
/// this emulation: the wrappers only call here where the path went through
/// untouched, so a deliberate mapping is never second-guessed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MountFile {
    Mounts,
    MountInfo,
}

pub fn mount_file(path: &[u8]) -> Option<MountFile> {
    if path == b"/proc/mounts\0" || path == b"/proc/self/mounts\0" || path == b"/etc/mtab\0" {
        Some(MountFile::Mounts)
    } else if path == b"/proc/self/mountinfo\0" {
        Some(MountFile::MountInfo)
    } else {
        None
    }
}

/// What opening the descriptor path may do.
///
/// `Dup` duplicates the descriptor: the kernel's own reopen of a pipe
/// through these leaves is a new descriptor on the same description, which
/// is what a duplicate is. `Fail(e)` is the kernel's exact answer where no
/// duplicate would be one (a socket opens as `ENXIO`, a directory flag on a
/// non-directory as `ENOTDIR`, an incompatible access mode as `EACCES`).
/// `Pass` falls through to the real call and its honest failure: anything
/// the rules below do not name exactly is refused, never approximated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OpenAnswer {
    Dup,
    Fail(c_int),
    Pass,
}

/// The flag transformation a duplicate may carry, T-0413.
///
/// A duplicate shares the file description, so every status flag the
/// request sets beyond the original would either mutate the shared
/// description (`O_NONBLOCK`, `O_APPEND`) or describe a new one
/// (`O_SYNC`, `O_DIRECT`): where either side disagrees the answer is the
/// fall-through, not a half-carried flag. Creation and truncation flags
/// have no meaning on an existing descriptor and always fall through, as
/// do `O_PATH`, `O_TMPFILE` and `O_NOFOLLOW`, whose semantics no duplicate
/// reproduces. `O_NOCTTY` and `O_LARGEFILE` are masked out first, as the
/// kernel ignores them where they do not apply (a pipe is no tty, and
/// offsets are 64-bit throughout).
pub fn open_answer(orig_flags: c_int, req_flags: c_int) -> OpenAnswer {
    // ⚠ `O_TMPFILE` is tested by its distinguishing bit, never by the
    // whole constant: the constant embeds `O_DIRECTORY`, so matching the
    // whole thing would refuse a plain directory flag as a tmpfile
    // instead of answering `ENOTDIR` for it (caught by the unit test).
    const TMPFILE_BIT: c_int = O_TMPFILE & !O_DIRECTORY;
    const REFUSED: c_int = O_CREAT | O_EXCL | O_TRUNC | O_PATH | TMPFILE_BIT | O_NOFOLLOW;
    if req_flags & REFUSED != 0 {
        return OpenAnswer::Pass;
    }
    // Masked first so every check below reads the significant flags: the
    // ignored set stays named here, where extending it is a claim the
    // kernel ignores the added flag on this path.
    let req_flags = req_flags & !(O_NOCTTY | O_LARGEFILE);
    if req_flags & O_DIRECTORY != 0 {
        return OpenAnswer::Fail(ENOTDIR);
    }
    // The shared description's flags: the request must carry what the
    // description holds, neither adding nor dropping.
    const SHARED: c_int = O_APPEND | O_NONBLOCK | O_DIRECT | O_ASYNC | O_SYNC | O_DSYNC;
    const O_ASYNC: c_int = 0o20000;
    if req_flags & SHARED != orig_flags & SHARED {
        return OpenAnswer::Pass;
    }
    let (orig_acc, req_acc) = (orig_flags & O_ACCMODE, req_flags & O_ACCMODE);
    let ok = match req_acc {
        O_RDONLY => orig_acc == O_RDONLY || orig_acc == O_RDWR,
        O_WRONLY => orig_acc == O_WRONLY || orig_acc == O_RDWR,
        O_RDWR => orig_acc == O_RDWR,
        _ => false,
    };
    if !ok {
        return OpenAnswer::Fail(EACCES);
    }
    OpenAnswer::Dup
}

/// Write `pipe:[ino]` / `socket:[ino]` into `out`, returning the length.
///
/// The kernel's exact rendering, prefix, brackets and digits included: a
/// `readlink` that answered anything else would read as a different file.
pub fn fd_link(prefix: &[u8], ino: u64, out: &mut [u8; 32]) -> usize {
    let mut at = 0usize;
    for c in prefix {
        if at >= out.len() {
            return at;
        }
        out[at] = *c;
        at += 1;
    }
    if at >= out.len() {
        return at;
    }
    out[at] = b'[';
    at += 1;
    let mut digits = [0u8; 20];
    let mut v = ino;
    let mut d = 0usize;
    loop {
        digits[d] = b'0' + (v % 10) as u8;
        v /= 10;
        d += 1;
        if v == 0 {
            break;
        }
    }
    while d > 0 {
        d -= 1;
        if at >= out.len() {
            return at;
        }
        out[at] = digits[d];
        at += 1;
    }
    if at >= out.len() {
        return at;
    }
    out[at] = b']';
    at + 1
}

/// The two link prefixes the kernel renders.
pub const PIPE_PREFIX: &[u8] = b"pipe:";
pub const SOCKET_PREFIX: &[u8] = b"socket:";

/// Filesystem names for the fixture's `/` line, from `statfs` magic.
///
/// Six the payload is likely to stand on, then `unknown`: a made-up name
/// would be someone else's topology, and a missing one is a word that says
/// the fixture does not know.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fstype {
    Ext4,
    Tmpfs,
    Overlay,
    Btrfs,
    Xfs,
    Unknown,
}

impl Fstype {
    pub fn of_magic(m: u64) -> Fstype {
        match m {
            0xEF53 => Fstype::Ext4,
            0x0102_1994 => Fstype::Tmpfs,
            0x794C_7630 => Fstype::Overlay,
            0x9126_800E => Fstype::Btrfs,
            0x5846_5342 => Fstype::Xfs,
            _ => Fstype::Unknown,
        }
    }

    pub fn word(self) -> &'static [u8] {
        match self {
            Fstype::Ext4 => b"ext4",
            Fstype::Tmpfs => b"tmpfs",
            Fstype::Overlay => b"overlay",
            Fstype::Btrfs => b"btrfs",
            Fstype::Xfs => b"xfs",
            Fstype::Unknown => b"unknown",
        }
    }
}

/// Escape a path for a mount-table line: space, tab, newline and backslash
/// take the octal escapes the kernel's own table uses, so a path with a
/// space parses back to one field rather than two.
pub fn escape_mount(s: &[u8], out: &mut [u8; 256]) -> usize {
    let mut at = 0usize;
    for c in s {
        let esc: Option<&[u8; 4]> = match c {
            b' ' => Some(b"\\040"),
            b'\t' => Some(b"\\011"),
            b'\n' => Some(b"\\012"),
            b'\\' => Some(b"\\134"),
            _ => None,
        };
        if let Some(e) = esc {
            if at + 4 > out.len() {
                return at;
            }
            out[at..at + 4].copy_from_slice(e);
            at += 4;
        } else {
            if at + 1 > out.len() {
                return at;
            }
            out[at] = *c;
            at += 1;
        }
    }
    at
}

/// One recorded T-0708 emulated mount, as offsets into the scan's staging
/// area.
///
/// ⛔ Offsets, not borrows: the scan keeps staging path bytes while it
/// takes records, and a borrow handed out early would forbid every later
/// write. The renderers slice these against the area they already hold.
#[derive(Clone, Copy)]
pub struct RecMount {
    pub src_start: usize,
    pub src_len: usize,
    pub tgt_start: usize,
    pub tgt_len: usize,
    pub ro: bool,
}

impl RecMount {
    /// The source bytes staged for this record.
    pub fn src<'a>(&self, area: &'a [u8]) -> &'a [u8] {
        &area[self.src_start..self.src_start + self.src_len]
    }

    /// The target bytes staged for this record.
    pub fn tgt<'a>(&self, area: &'a [u8]) -> &'a [u8] {
        &area[self.tgt_start..self.tgt_start + self.tgt_len]
    }
}

/// Append one line (no trailing newline accounting beyond it) to the
/// fixture, stopping whole lines at the buffer's end: a mount table that
/// ends mid-line is a corrupt table, and a shorter honest one beats it.
fn push_line(out: &mut [u8; FIXTURE_MAX], at: &mut usize, line: &[u8]) {
    if *at + line.len() + 1 > out.len() {
        return;
    }
    out[*at..*at + line.len()].copy_from_slice(line);
    *at += line.len();
    out[*at] = b'\n';
    *at += 1;
}

/// The fixture's ceiling: 8 KiB of stack. A payload with more emulated
/// mounts than fit keeps the root line and the first ones; the fixture is
/// an approximation TOOL.md section 10 licenses, and a bounded one.
pub const FIXTURE_MAX: usize = 8192;

/// Render the `mounts`/`mtab` fixture: the `/` line from live topology,
/// then the recorded emulated mounts.
pub fn render_mounts(
    fstype: Fstype,
    rw: bool,
    area: &[u8; 4096],
    mounts: &[RecMount],
    out: &mut [u8; FIXTURE_MAX],
) -> usize {
    let mut at = 0usize;
    let mut line = [0u8; 512];
    let opts = if rw { &b"rw"[..] } else { &b"ro"[..] };
    {
        let n = line_of_mounts(&mut line, b"podbox", b"/", fstype.word(), opts);
        push_line(out, &mut at, &line[..n]);
    }
    for m in mounts {
        let mopts = if m.ro { &b"ro"[..] } else { &b"rw"[..] };
        let n = line_of_mounts(&mut line, m.src(area), m.tgt(area), b"none", mopts);
        push_line(out, &mut at, &line[..n]);
    }
    at
}

fn line_of_mounts(
    out: &mut [u8; 512],
    src: &[u8],
    tgt: &[u8],
    fstype: &[u8],
    opts: &[u8],
) -> usize {
    let mut at = 0usize;
    let mut esc = [0u8; 256];
    let mut field = |bytes: &[u8]| {
        for c in bytes {
            if at < out.len() {
                out[at] = *c;
                at += 1;
            }
        }
    };
    let n = escape_mount(src, &mut esc);
    field(&esc[..n]);
    field(b" ");
    let n = escape_mount(tgt, &mut esc);
    field(&esc[..n]);
    field(b" ");
    field(fstype);
    field(b" ");
    field(opts);
    field(b" 0 0");
    at
}

/// Render the `mountinfo` fixture: ids are this table's own sequence from
/// 1, never anyone else's mount ids. The emulated mounts carry `0:0`
/// because an emulated mount has no device; the `/` line carries the live
/// one, which is the real topology the fixture is generated from.
pub fn render_mountinfo(
    major: u32,
    minor: u32,
    fstype: Fstype,
    rw: bool,
    area: &[u8; 4096],
    mounts: &[RecMount],
    out: &mut [u8; FIXTURE_MAX],
) -> usize {
    let mut at = 0usize;
    let mut line = [0u8; 768];
    let opts = if rw { &b"rw"[..] } else { &b"ro"[..] };
    {
        let n = line_of_mountinfo(
            &mut line,
            1,
            1,
            major,
            minor,
            b"/",
            b"/",
            opts,
            fstype.word(),
            b"podbox",
            opts,
        );
        push_line(out, &mut at, &line[..n]);
    }
    for (id, m) in (2u32..).zip(mounts.iter()) {
        let mopts = if m.ro { &b"ro"[..] } else { &b"rw"[..] };
        let n = line_of_mountinfo(
            &mut line,
            id,
            1,
            0,
            0,
            b"/",
            m.tgt(area),
            mopts,
            b"none",
            m.src(area),
            mopts,
        );
        push_line(out, &mut at, &line[..n]);
    }
    at
}

fn num(mut v: u32, out: &mut [u8; 768], at: &mut usize) {
    let mut digits = [0u8; 10];
    let mut d = 0usize;
    loop {
        digits[d] = b'0' + (v % 10) as u8;
        v /= 10;
        d += 1;
        if v == 0 {
            break;
        }
    }
    while d > 0 {
        d -= 1;
        if *at < out.len() {
            out[*at] = digits[d];
            *at += 1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn line_of_mountinfo(
    out: &mut [u8; 768],
    id: u32,
    parent: u32,
    major: u32,
    minor: u32,
    root: &[u8],
    mp: &[u8],
    opts: &[u8],
    fstype: &[u8],
    source: &[u8],
    sup: &[u8],
) -> usize {
    let mut at = 0usize;
    let mut esc = [0u8; 256];
    num(id, out, &mut at);
    let mut field = |bytes: &[u8]| {
        if at < out.len() {
            out[at] = b' ';
            at += 1;
        }
        for c in bytes {
            if at < out.len() {
                out[at] = *c;
                at += 1;
            }
        }
    };
    // parent
    {
        let mut tmp = [0u8; 768];
        let mut t = 0usize;
        num(parent, &mut tmp, &mut t);
        field(&tmp[..t]);
    }
    // major:minor
    {
        let mut tmp = [0u8; 768];
        let mut t = 0usize;
        num(major, &mut tmp, &mut t);
        if t < tmp.len() {
            tmp[t] = b':';
            t += 1;
        }
        let mut t2 = t;
        // minor appended after the colon
        let mut digits = [0u8; 10];
        let mut d = 0usize;
        let mut v = minor;
        loop {
            digits[d] = b'0' + (v % 10) as u8;
            v /= 10;
            d += 1;
            if v == 0 {
                break;
            }
        }
        while d > 0 {
            d -= 1;
            if t2 < tmp.len() {
                tmp[t2] = digits[d];
                t2 += 1;
            }
        }
        field(&tmp[..t2]);
    }
    let n = escape_mount(root, &mut esc);
    field(&esc[..n]);
    let n = escape_mount(mp, &mut esc);
    field(&esc[..n]);
    field(opts);
    field(b"-");
    field(fstype);
    let n = escape_mount(source, &mut esc);
    field(&esc[..n]);
    field(sup);
    at
}

/// Scan the memo for complete T-0708 mount records, T-0413.
///
/// The fixed 32-byte layout is the tally's own: a header (`u64::MAX`,
/// op 2, flags, source chains, target chains) with its continuation
/// records contiguous behind it, exactly as `emulate::tally_mount` wrote
/// them. Anything else -- ownership records, another header, EOF, a short
/// read -- drops the open chain rather than half-reading it, which is the
/// host-side scan's own rule. Paths stage into `area`; records name
/// offsets into it, so the scan never borrows what it keeps writing.
/// Past either bound the scan stops taking records.
///
/// Returns the records taken.
///
/// # Safety
/// `fd` must be a readable descriptor on the memo file.
pub unsafe fn scan_mounts(fd: c_int, area: &mut [u8; 4096], recs: &mut [RecMount; 64]) -> usize {
    const REC: usize = crate::memo::RECORD;
    if unsafe { lseek(fd, 0, 0) } < 0 {
        return 0;
    }
    let mut buf = [0u8; 128 * REC];
    let mut taken = 0usize;
    let mut used = 0usize;
    // The open chain, if any: which side is arriving, how many records of
    // it are still owed, and the header's words behind it.
    let mut src_left = 0u32;
    let mut tgt_total = 0u32;
    let mut tgt_left = 0u32;
    let mut s_start = 0usize;
    let mut s_len = 0usize;
    let mut t_start = 0usize;
    let mut t_len = 0usize;
    let mut flags = 0u32;
    let drop_chain = |src_left: &mut u32, tgt_left: &mut u32| {
        *src_left = 0;
        *tgt_left = 0;
    };
    // Records arrive whole: the writer appends 32 bytes at a time under
    // O_APPEND, so a short read is EOF, never a torn record.
    loop {
        let n = unsafe { read(fd, buf.as_mut_ptr() as *mut c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        let mut at = 0usize;
        while at + REC <= n as usize {
            let r = &buf[at..at + REC];
            at += REC;
            let dev = u64::from_le_bytes(r[0..8].try_into().unwrap_or([0u8; 8]));
            let op = u64::from_le_bytes(r[8..16].try_into().unwrap_or([0u8; 8]));
            if dev != u64::MAX {
                // An ownership record: it also breaks any open chain, for
                // the writer's reason (a chain is contiguous or it is torn).
                drop_chain(&mut src_left, &mut tgt_left);
                continue;
            }
            if op == crate::emulate::OP_MOUNT {
                let cs = u32::from_le_bytes(r[20..24].try_into().unwrap_or([0u8; 4]));
                let ct = u32::from_le_bytes(r[24..28].try_into().unwrap_or([0u8; 4]));
                flags = u32::from_le_bytes(r[16..20].try_into().unwrap_or([0u8; 4]));
                s_start = used;
                s_len = 0;
                t_start = used;
                t_len = 0;
                src_left = cs;
                tgt_total = ct;
                tgt_left = 0;
                if cs == 0 {
                    // No source side: the target side arrives next, or an
                    // empty mount completes at once.
                    tgt_left = ct;
                    if ct == 0 {
                        push_rec(recs, &mut taken, s_start, 0, t_start, 0, flags);
                    }
                }
                continue;
            }
            if op != crate::emulate::CONT {
                drop_chain(&mut src_left, &mut tgt_left);
                continue;
            }
            let in_src = src_left > 0;
            let in_tgt = src_left == 0 && tgt_left > 0;
            if !in_src && !in_tgt {
                continue;
            }
            let piece = &r[16..32];
            if used + 16 > area.len() || taken >= recs.len() {
                drop_chain(&mut src_left, &mut tgt_left);
                continue;
            }
            // The last record of a side may be NUL-padded; the padding is
            // not path.
            let mut k = 16usize;
            while k > 0 && piece[k - 1] == 0 {
                k -= 1;
            }
            if in_src && s_len == 0 {
                s_start = used;
            }
            if in_tgt && t_len == 0 {
                t_start = used;
            }
            area[used..used + k].copy_from_slice(&piece[..k]);
            // ⚠ `k == 0` still advances nothing and counts the record: an
            // all-NUL continuation is padding the writer never emits, and
            // counting it keeps the chain aligned with the header.
            used += k;
            if in_src {
                s_len += k;
                src_left -= 1;
                if src_left == 0 {
                    tgt_left = tgt_total;
                    if tgt_left == 0 {
                        push_rec(recs, &mut taken, s_start, s_len, t_start, t_len, flags);
                    }
                }
            } else {
                t_len += k;
                tgt_left -= 1;
                if tgt_left == 0 {
                    push_rec(recs, &mut taken, s_start, s_len, t_start, t_len, flags);
                }
            }
        }
        if n as usize != buf.len() {
            break;
        }
    }
    taken
}

fn push_rec(
    recs: &mut [RecMount; 64],
    taken: &mut usize,
    s_start: usize,
    s_len: usize,
    t_start: usize,
    t_len: usize,
    flags: u32,
) {
    if *taken >= recs.len() {
        return;
    }
    const MS_RDONLY: u32 = 1;
    recs[*taken] = RecMount {
        src_start: s_start,
        src_len: s_len,
        tgt_start: t_start,
        tgt_len: t_len,
        ro: flags & MS_RDONLY != 0,
    };
    *taken += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z(s: &[u8]) -> Vec<u8> {
        let mut v = s.to_vec();
        v.push(0);
        v
    }

    #[test]
    fn fd_spellings_match_exactly() {
        assert_eq!(fd_number(&z(b"/proc/self/fd/0")), Some(0));
        assert_eq!(fd_number(&z(b"/proc/self/fd/63")), Some(63));
        assert_eq!(fd_number(&z(b"/dev/fd/63")), Some(63));
        // A trailing slash is ENOTDIR on a live /proc, never a descriptor.
        assert_eq!(fd_number(&z(b"/proc/self/fd/3/")), None);
        assert_eq!(fd_number(&z(b"/proc/self/fd/")), None);
        assert_eq!(fd_number(&z(b"/proc/self/fd/x")), None);
        assert_eq!(fd_number(&z(b"/proc/self/fd/3x")), None);
        assert_eq!(fd_number(&z(b"/proc/self/fd/-1")), None);
        // No NUL terminator is not a path the kernel took.
        assert_eq!(fd_number(b"/proc/self/fd/3"), None);
        // Past any descriptor table: refused, never wrapped.
        assert_eq!(fd_number(&z(b"/proc/self/fd/2147483648")), None);
        assert_eq!(fd_number(&z(b"/proc/self/fd/99999999999999999999")), None);
        // Other pids' descriptors are not answerable without their tables.
        assert_eq!(fd_number(&z(b"/proc/1234/fd/3")), None);
        assert_eq!(fd_number(&z(b"proc/self/fd/3")), None);
        // The standard stream spellings name their descriptors by definition.
        assert_eq!(fd_number(&z(b"/dev/stdin")), Some(0));
        assert_eq!(fd_number(&z(b"/dev/stdout")), Some(1));
        assert_eq!(fd_number(&z(b"/dev/stderr")), Some(2));
        assert_eq!(fd_number(&z(b"/dev/stdin/")), None);
    }

    #[test]
    fn exe_matches_only_itself() {
        assert!(is_self_exe(&z(b"/proc/self/exe")));
        assert!(!is_self_exe(&z(b"/proc/self/exe/")));
        assert!(!is_self_exe(&z(b"/proc/self/cwd")));
        assert!(!is_self_exe(b"/proc/self/exe"));
    }

    #[test]
    fn mount_files_sort_into_two_renderings() {
        assert_eq!(mount_file(&z(b"/proc/mounts")), Some(MountFile::Mounts));
        assert_eq!(
            mount_file(&z(b"/proc/self/mounts")),
            Some(MountFile::Mounts)
        );
        assert_eq!(mount_file(&z(b"/etc/mtab")), Some(MountFile::Mounts));
        assert_eq!(
            mount_file(&z(b"/proc/self/mountinfo")),
            Some(MountFile::MountInfo)
        );
        assert_eq!(mount_file(&z(b"/proc/cpuinfo")), None);
        assert_eq!(mount_file(&z(b"/proc/self/mountinfo/")), None);
    }

    #[test]
    fn compatible_flags_duplicate_and_the_rest_fall_through() {
        // Same mode both sides, no status flags: the duplicate.
        assert_eq!(open_answer(O_RDONLY, O_RDONLY), OpenAnswer::Dup);
        assert_eq!(open_answer(O_RDWR, O_RDWR), OpenAnswer::Dup);
        // CLOEXEC rides the dup variant; NOCTTY and LARGEFILE are ignored.
        assert_eq!(open_answer(O_RDONLY, O_RDONLY | O_CLOEXEC), OpenAnswer::Dup);
        assert_eq!(
            open_answer(O_RDONLY, O_RDONLY | O_NOCTTY | O_LARGEFILE),
            OpenAnswer::Dup
        );
        // A weaker request than the description holds is fine.
        assert_eq!(open_answer(O_RDWR, O_RDONLY), OpenAnswer::Dup);
        assert_eq!(open_answer(O_RDWR, O_WRONLY), OpenAnswer::Dup);
        // A stronger one is the kernel's EACCES.
        assert_eq!(open_answer(O_WRONLY, O_RDONLY), OpenAnswer::Fail(EACCES));
        assert_eq!(open_answer(O_RDONLY, O_WRONLY), OpenAnswer::Fail(EACCES));
        assert_eq!(open_answer(O_RDONLY, O_RDWR), OpenAnswer::Fail(EACCES));
        // A directory flag on a non-directory is ENOTDIR, exactly.
        assert_eq!(
            open_answer(O_RDONLY, O_RDONLY | O_DIRECTORY),
            OpenAnswer::Fail(ENOTDIR)
        );
        // Creation, truncation, path-only, tmpfile and nofollow have no
        // duplicate semantics: the fall-through, never an approximation.
        for f in [O_CREAT, O_TRUNC, O_EXCL, O_PATH, O_TMPFILE, O_NOFOLLOW] {
            assert_eq!(
                open_answer(O_RDONLY, O_RDONLY | f),
                OpenAnswer::Pass,
                "{f:o}"
            );
        }
        // A status flag either side does not carry is a different
        // description, not a dup with a tweak.
        assert_eq!(
            open_answer(O_RDONLY, O_RDONLY | O_NONBLOCK),
            OpenAnswer::Pass
        );
        assert_eq!(open_answer(O_RDONLY | O_APPEND, O_RDONLY), OpenAnswer::Pass);
        assert_eq!(open_answer(O_RDONLY, O_RDONLY | O_SYNC), OpenAnswer::Pass);
    }

    #[test]
    fn links_render_as_the_kernel_renders_them() {
        let mut out = [0u8; 32];
        let n = fd_link(PIPE_PREFIX, 12345, &mut out);
        assert_eq!(&out[..n], b"pipe:[12345]");
        let n = fd_link(SOCKET_PREFIX, 7, &mut out);
        assert_eq!(&out[..n], b"socket:[7]");
        let n = fd_link(PIPE_PREFIX, 0, &mut out);
        assert_eq!(&out[..n], b"pipe:[0]");
    }

    #[test]
    fn fstype_names_what_it_knows_and_says_the_rest() {
        assert_eq!(Fstype::of_magic(0xEF53), Fstype::Ext4);
        assert_eq!(Fstype::of_magic(0x0102_1994), Fstype::Tmpfs);
        assert_eq!(Fstype::of_magic(0x794C_7630), Fstype::Overlay);
        assert_eq!(Fstype::of_magic(0x9126_800E), Fstype::Btrfs);
        assert_eq!(Fstype::of_magic(0x5846_5342), Fstype::Xfs);
        assert_eq!(Fstype::of_magic(0x6464_6F63), Fstype::Unknown);
        assert_eq!(Fstype::Unknown.word(), b"unknown");
    }

    #[test]
    fn mount_escapes_keep_one_path_one_field() {
        let mut out = [0u8; 256];
        let n = escape_mount(b"/mnt/my drive", &mut out);
        assert_eq!(&out[..n], b"/mnt/my\\040drive");
        let n = escape_mount(b"/plain", &mut out);
        assert_eq!(&out[..n], b"/plain");
    }

    #[test]
    fn the_mounts_fixture_names_root_and_the_recorded_mounts() {
        let mut area = [0u8; 4096];
        area[0..4].copy_from_slice(b"none");
        area[4..10].copy_from_slice(b"/mnt/x");
        let mounts = [RecMount {
            src_start: 0,
            src_len: 4,
            tgt_start: 4,
            tgt_len: 6,
            ro: false,
        }];
        let mut out = [0u8; FIXTURE_MAX];
        let n = render_mounts(Fstype::Ext4, true, &area, &mounts, &mut out);
        let text = std::str::from_utf8(&out[..n]).unwrap();
        let mut lines = text.lines();
        assert_eq!(lines.next().unwrap(), "podbox / ext4 rw 0 0");
        assert_eq!(lines.next().unwrap(), "none /mnt/x none rw 0 0");
        assert_eq!(lines.next(), None);
    }

    #[test]
    fn the_mountinfo_fixture_sequences_its_own_ids() {
        let mut area = [0u8; 4096];
        area[0..4].copy_from_slice(b"none");
        area[4..10].copy_from_slice(b"/mnt/x");
        let mounts = [RecMount {
            src_start: 0,
            src_len: 4,
            tgt_start: 4,
            tgt_len: 6,
            ro: true,
        }];
        let mut out = [0u8; FIXTURE_MAX];
        let n = render_mountinfo(8, 1, Fstype::Ext4, true, &area, &mounts, &mut out);
        let text = std::str::from_utf8(&out[..n]).unwrap();
        let mut lines = text.lines();
        assert_eq!(lines.next().unwrap(), "1 1 8:1 / / rw - ext4 podbox rw");
        assert_eq!(lines.next().unwrap(), "2 1 0:0 / /mnt/x ro - none none ro");
        assert_eq!(lines.next(), None);
    }

    #[test]
    fn mode_ino_and_device_words_read_back() {
        let mut st = [0u8; STAT_LEN];
        st[ST_MODE..ST_MODE + 4].copy_from_slice(&0o10600u32.to_ne_bytes());
        st[ST_INO..ST_INO + 8].copy_from_slice(&123456u64.to_ne_bytes());
        assert_eq!(mode_of(&st) & S_IFMT, S_IFIFO);
        assert_eq!(ino_of(&st), 123456);
        // makedev(8, 1) is 0x801: the inverse names both halves.
        assert_eq!(major_of(0x801), 8);
        assert_eq!(minor_of(0x801), 1);
    }

    /// A memo file with one complete mount, one torn chain and one
    /// ownership record: the scan takes the complete mount with its
    /// read-only flag and nothing else.
    #[test]
    fn the_scan_takes_complete_mounts_and_drops_the_rest() {
        use std::io::Write;
        use std::os::unix::io::AsRawFd;

        fn header(flags: u32, cs: u32, ct: u32) -> [u8; 32] {
            let mut r = [0u8; 32];
            r[0..8].copy_from_slice(&u64::MAX.to_le_bytes());
            r[8..16].copy_from_slice(&crate::emulate::OP_MOUNT.to_le_bytes());
            r[16..20].copy_from_slice(&flags.to_le_bytes());
            r[20..24].copy_from_slice(&cs.to_le_bytes());
            r[24..28].copy_from_slice(&ct.to_le_bytes());
            r
        }
        fn cont(piece: &[u8]) -> [u8; 32] {
            let mut r = [0u8; 32];
            r[0..8].copy_from_slice(&u64::MAX.to_le_bytes());
            r[8..16].copy_from_slice(&crate::emulate::CONT.to_le_bytes());
            r[16..16 + piece.len()].copy_from_slice(piece);
            r
        }
        fn owner() -> [u8; 32] {
            let mut r = [0u8; 32];
            r[0..8].copy_from_slice(&8u64.to_le_bytes());
            r[8..16].copy_from_slice(&9u64.to_le_bytes());
            r
        }

        let d = std::env::temp_dir().join(format!("podbox-procfs-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let path = d.join("memo");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            // Complete: flags ro, source "none" (one record), target
            // "/mnt/x" (one record).
            f.write_all(&header(1, 1, 1)).unwrap();
            f.write_all(&cont(b"none")).unwrap();
            f.write_all(&cont(b"/mnt/x")).unwrap();
            // Torn: a header whose target chain never arrives, broken by
            // an ownership record.
            f.write_all(&header(0, 1, 1)).unwrap();
            f.write_all(&cont(b"src")).unwrap();
            f.write_all(&owner()).unwrap();
            // Complete read-write mount behind the tear.
            f.write_all(&header(0, 1, 1)).unwrap();
            f.write_all(&cont(b"tmpfs")).unwrap();
            f.write_all(&cont(b"/mnt/y")).unwrap();
        }
        let f = std::fs::File::open(&path).unwrap();
        let mut area = [0u8; 4096];
        let mut recs: [RecMount; 64] = [RecMount {
            src_start: 0,
            src_len: 0,
            tgt_start: 0,
            tgt_len: 0,
            ro: false,
        }; 64];
        let taken = unsafe { scan_mounts(f.as_raw_fd(), &mut area, &mut recs) };
        assert_eq!(taken, 2);
        assert_eq!(recs[0].src(&area), b"none");
        assert_eq!(recs[0].tgt(&area), b"/mnt/x");
        assert!(recs[0].ro);
        assert_eq!(recs[1].src(&area), b"tmpfs");
        assert_eq!(recs[1].tgt(&area), b"/mnt/y");
        assert!(!recs[1].ro);
        let _ = std::fs::remove_dir_all(&d);
    }
}
