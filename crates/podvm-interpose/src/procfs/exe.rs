use super::*;

/// Should `path` (NUL-terminated, as the kernel took it) be answered from
/// the guest exe even where the real call succeeds?
///
/// Pure over the flag and the spelling, so the wrappers and the unit test
/// read the same words: exactly `/proc/self/exe` with the flag on, and
/// nothing else. [`is_pid_exe`] adds the spelling with the process's own
/// pid, which [`exe_override_hit`] reads. No mapping can win over this spelling: the `/proc` leaves
/// never rewrite (the builtin exclusion), so the wrappers ask before
/// consulting the table.
pub fn should_override_exe(override_on: bool, path_with_nul: &[u8]) -> bool {
    override_on && is_self_exe(path_with_nul)
}

/// Should the raw C `path` be answered from the guest exe?
///
/// The flag plus the exact spelling, length-capped like every other path
/// here. False on a null pointer, an uncapped path, or an empty one: none
/// of those is a spelling the kernel took.
///
/// # Safety
/// `path` must be readable up to its NUL within `OUT` bytes, which is what
/// every entry point passes.
pub(crate) unsafe fn exe_override_hit(path: *const c_char) -> bool {
    if path.is_null() {
        return false;
    }
    if !unsafe { exe_override_enabled() } {
        return false;
    }
    let n = crate::map::strnlen(path, crate::map::OUT);
    if n == 0 || n >= crate::map::OUT {
        return false;
    }
    let bytes = unsafe { core::slice::from_raw_parts(path as *const u8, n + 1) };
    unsafe extern "C" {
        fn getpid() -> c_int;
    }
    should_override_exe(true, bytes) || is_pid_exe(bytes, unsafe { getpid() })
}

unsafe extern "C" {
    fn fstatfs(fd: c_int, buf: *mut c_void) -> c_int;
    fn fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int;
    pub(super) fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    pub(super) fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
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
pub(super) const O_RDONLY: c_int = 0o0;
pub(super) const O_WRONLY: c_int = 0o1;
pub(super) const O_RDWR: c_int = 0o2;
pub(super) const O_CREAT: c_int = 0o100;
pub(super) const O_EXCL: c_int = 0o200;
pub(super) const O_NOCTTY: c_int = 0o400;
pub(super) const O_TRUNC: c_int = 0o1000;
pub(super) const O_APPEND: c_int = 0o2000;
pub(super) const O_NONBLOCK: c_int = 0o4000;
const O_DSYNC: c_int = 0o10000;
const O_DIRECT: c_int = 0o40000;
pub(super) const O_LARGEFILE: c_int = 0o100000;
pub(super) const O_DIRECTORY: c_int = 0o200000;
pub(super) const O_NOFOLLOW: c_int = 0o400000;
pub const O_CLOEXEC: c_int = 0o2000000;
pub(super) const O_SYNC: c_int = 0o4010000;
pub(super) const O_PATH: c_int = 0o10000000;
pub(super) const O_TMPFILE: c_int = 0o20000000 | O_DIRECTORY;

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
pub(super) const ST_INO: usize = 8;
#[cfg(target_arch = "x86_64")]
pub(super) const ST_MODE: usize = 24;
/// aarch64: the generic layout (experiments/439-stat-layout.sh).
#[cfg(target_arch = "aarch64")]
pub(super) const ST_MODE: usize = 16;

/// `struct statfs` on x86_64 is kernel ABI, identical under both libcs:
/// `f_type` a `long` at 0, `f_flags` a `long` at 80, 120 bytes whole.
pub const STATFS_LEN: usize = 120;
pub const STATFS_FLAGS: usize = 80;
pub const ST_RDONLY: u64 = 1;

/// `struct stat` on x86_64, 144 bytes whole. Only the header travels here:
/// the glue fills it with the real `fstat` and these read it back.
#[cfg(target_arch = "x86_64")]
pub const STAT_LEN: usize = 144;
/// `struct stat` on aarch64, 128 bytes whole.
#[cfg(target_arch = "aarch64")]
pub const STAT_LEN: usize = 128;

/// Split device numbers back out of `st_dev`.
///
/// ⛔ The inverse of the `makedev` this crate and `podvm-probe` share,
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

/// Exactly `/proc/PID/exe` for the pid `pid`, NUL-terminated.
///
/// ⛔ TODO/interpose.md T-1704. It is the same link as `/proc/self/exe` for
/// the process itself. GNU `realpath(1)` reads it: gnulib's canonicalize
/// follows `/proc/self` to `/proc/PID` and then reads `exe` there, so with
/// the self spelling alone it named the image's loader (measured with the
/// loader of the host and `realpath` of coreutils). Another pid is another
/// process, and its link stays the kernel's.
pub fn is_pid_exe(path: &[u8], pid: c_int) -> bool {
    let Some(rest) = path.strip_prefix(b"/proc/") else {
        return false;
    };
    let Some(digits) = rest.strip_suffix(b"/exe\0") else {
        return false;
    };
    if pid <= 0 || digits.is_empty() || digits[0] == b'0' || !digits.iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    let mut v: i64 = 0;
    for c in digits {
        v = v * 10 + (c - b'0') as i64;
        if v > c_int::MAX as i64 {
            return false;
        }
    }
    v == pid as i64
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
