//! The syscall tier (TODO/enter.md T-1657): the answers of the handler
//! beyond the paths of T-1652.
//!
//! * `uname` gives a fixed kernel identity that names the tier, as gVisor's
//!   `4.19.0-gvisor` does: [`RELEASE`] and [`VERSION`].
//! * `/tmp` is a directory of the store (a volume) with a size cap. The
//!   handler counts the growth of each file that a descriptor opened under
//!   `/tmp` writes (`write`, `pwrite64`, `writev`, `pwritev`, `pwritev2`,
//!   `ftruncate`, `fallocate`), and a write over the cap fails with
//!   `ENOSPC`, as a full tmpfs answers. An `unlink` under `/tmp` gives the
//!   space back. The count lives in a shared page of a memfd, so each
//!   process of the payload reads one count, also after an `execve`.
//!
//! ⚠ The count is not a boundary: a write through `mmap`, `sendfile`,
//! `splice` or `copy_file_range`, and a rename into `/tmp`, are not
//! counted. The handler runs in the payload's address space (T-1652), so
//! the tier is a compatibility layer, as the banner says.

use core::sync::atomic::{AtomicBool, AtomicI64, AtomicPtr, AtomicU64, Ordering};

use crate::sys::{self, nr, sc, sc6};

/// The release that `uname` gives on the tier.
pub const RELEASE: &[u8] = crate::TIER_RELEASE.as_bytes();
/// The version that `uname` gives on the tier.
pub const VERSION: &[u8] = b"#1 SMP podvm syscall tier (TODO/enter.md T-1657)";
/// The guest path of the sized directory.
pub const TMP: &[u8] = b"/tmp";

const WRITE: i64 = 1;
const FSTAT: i64 = 5;
const LSEEK: i64 = 8;
const PWRITE64: i64 = 18;
const WRITEV: i64 = 20;
const DUP: i64 = 32;
const UNAME: i64 = 63;
const FTRUNCATE: i64 = 77;
const FALLOCATE: i64 = 285;
const PWRITEV: i64 = 296;
const MEMFD_CREATE: i64 = 319;
const PWRITEV2: i64 = 328;
const F_DUPFD: i64 = 0;
const F_GETFL: i64 = 3;
const F_DUPFD_CLOEXEC: i64 = 1030;
const O_APPEND: i64 = 0o2000;
const ENOSPC: i64 = 28;
const SEEK_CUR: i64 = 1;
const MAP_SHARED: i64 = 1;

static ON: AtomicBool = AtomicBool::new(false);
static CAP: AtomicI64 = AtomicI64::new(0);
static USED: AtomicPtr<AtomicI64> = AtomicPtr::new(core::ptr::null_mut());
/// The memfd of the count; the handler keeps it from the payload's close.
pub static FD: AtomicI64 = AtomicI64::new(-1);
/// The descriptors below 4096 that were opened under `/tmp`.
static MARK: [AtomicU64; 64] = [const { AtomicU64::new(0) }; 64];

/// True on the syscall tier.
pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// The cap in bytes, 0 for none.
pub fn cap() -> i64 {
    CAP.load(Ordering::Relaxed)
}

/// True where `g` (a guest path) is `/tmp` or under it.
pub fn under_tmp(g: &[u8]) -> bool {
    g == TMP || (g.starts_with(TMP) && g.get(TMP.len()) == Some(&b'/'))
}

/// The growth of a file of `size` bytes by a write of `len` bytes at
/// `start`.
pub fn growth(start: i64, len: i64, size: i64) -> i64 {
    (start.saturating_add(len) - size).max(0)
}

/// True where a growth fits under the cap with `used` bytes counted.
pub fn fits(used: i64, grow: i64, cap: i64) -> bool {
    cap <= 0 || grow <= 0 || used.saturating_add(grow) <= cap
}

/// Fill a `struct utsname` (6 fields of 65 bytes) with the tier's release
/// and version over the host's answer.
pub fn fill_uname(u: &mut [u8; 390]) {
    for (at, v) in [(130usize, RELEASE), (195usize, VERSION)] {
        let field = &mut u[at..at + 65];
        field.fill(0);
        field[..v.len().min(64)].copy_from_slice(&v[..v.len().min(64)]);
    }
}

/// Turn the tier on. `spec` is `CAP` or `CAP:FD` (bytes, and the memfd of
/// a count that an earlier process of the payload made). False where the
/// spec does not parse or the count cannot be mapped.
pub fn set(spec: &[u8]) -> bool {
    let mut parts = spec.splitn(2, |&c| c == b':');
    let Some(cap) = parts.next().and_then(num) else {
        return false;
    };
    let fd = match parts.next() {
        Some(f) => match num(f) {
            Some(n) => n,
            None => return false,
        },
        None => {
            // SAFETY: memfd_create reads the NUL-ended name.
            let fd = unsafe { sc(MEMFD_CREATE, c"podvm-tmp".as_ptr() as i64, 0, 0) };
            if fd < 0 || unsafe { sc(FTRUNCATE, fd, 4096, 0) } < 0 {
                return false;
            }
            // Above the payload's usual range, as the root's descriptor.
            let high = unsafe { sc(nr::FCNTL, fd, F_DUPFD, 1001) };
            sys::close(fd);
            if high < 0 {
                return false;
            }
            high
        }
    };
    // SAFETY: a shared map of the count's page.
    let m = unsafe { sc6(nr::MMAP, 0, 4096, 3, MAP_SHARED, fd, 0) };
    if m < 0 {
        return false;
    }
    USED.store(m as *mut AtomicI64, Ordering::Relaxed);
    FD.store(fd, Ordering::Relaxed);
    CAP.store(cap, Ordering::Relaxed);
    ON.store(true, Ordering::Relaxed);
    true
}

/// `CAP:FD` for the next `execve`, into `out`; its length.
pub fn spec(out: &mut [u8; 48]) -> usize {
    let mut n = put(out, 0, cap());
    out[n] = b':';
    n += 1;
    put(out, n, FD.load(Ordering::Relaxed))
}

fn put(out: &mut [u8; 48], at: usize, mut v: i64) -> usize {
    let mut d = [0u8; 20];
    let mut k = 0;
    loop {
        d[k] = b'0' + (v % 10) as u8;
        k += 1;
        v /= 10;
        if v == 0 || k == d.len() {
            break;
        }
    }
    let mut n = at;
    while k > 0 {
        k -= 1;
        out[n] = d[k];
        n += 1;
    }
    n
}

fn num(s: &[u8]) -> Option<i64> {
    if s.is_empty() || s.len() > 18 || !s.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(s.iter().fold(0i64, |v, &c| v * 10 + (c - b'0') as i64))
}

fn used() -> Option<&'static AtomicI64> {
    let p = USED.load(Ordering::Relaxed);
    // SAFETY: a page of the count, mapped by `set` for the process's life.
    (!p.is_null()).then(|| unsafe { &*p })
}

/// Mark or unmark a descriptor as one of `/tmp`.
pub fn mark(fd: i64, on: bool) {
    if !(0..4096).contains(&fd) {
        return;
    }
    let (w, b) = ((fd / 64) as usize, 1u64 << (fd % 64));
    if on {
        MARK[w].fetch_or(b, Ordering::Relaxed);
    } else {
        MARK[w].fetch_and(!b, Ordering::Relaxed);
    }
}

/// True where `fd` was opened under `/tmp`.
pub fn marked(fd: i64) -> bool {
    (0..4096).contains(&fd)
        && MARK[(fd / 64) as usize].load(Ordering::Relaxed) & (1 << (fd % 64)) != 0
}

/// The size of the file of `fd`, from `fstat`.
fn size_of(fd: i64) -> Option<i64> {
    let mut st = [0u8; 144];
    // SAFETY: fstat writes a struct stat of 144 bytes.
    let r = unsafe { sc(FSTAT, fd, st.as_mut_ptr() as i64, 0) };
    (r >= 0).then(|| i64::from_ne_bytes(st[48..56].try_into().unwrap_or([0; 8])))
}

/// The bytes of an iovec array.
fn iov_len(iov: i64, n: i64) -> i64 {
    let mut t = 0i64;
    for i in 0..n.clamp(0, 1024) {
        // SAFETY: the payload gave an array of n iovecs at iov.
        t = t.saturating_add(unsafe { *((iov + i * 16 + 8) as *const i64) });
    }
    t
}

/// A call of the tier, or None for a call that runs as the payload gave it.
pub fn call(n: i64, a: [i64; 6]) -> Option<i64> {
    if !on() {
        return None;
    }
    match n {
        UNAME => Some(uname(a[0])),
        WRITE | PWRITE64 | WRITEV | PWRITEV | PWRITEV2 | FTRUNCATE | FALLOCATE if marked(a[0]) => {
            Some(write(n, a))
        }
        nr::CLOSE => {
            mark(a[0], false);
            None
        }
        DUP => Some(dup_of(a[0], unsafe { sc(DUP, a[0], 0, 0) })),
        nr::DUP2 | nr::DUP3 => {
            let r = unsafe { sc6(n, a[0], a[1], a[2], 0, 0, 0) };
            if r >= 0 {
                mark(r, marked(a[0]));
            }
            Some(r)
        }
        nr::FCNTL if a[1] == F_DUPFD || a[1] == F_DUPFD_CLOEXEC => {
            Some(dup_of(a[0], unsafe { sc(nr::FCNTL, a[0], a[1], a[2]) }))
        }
        _ => None,
    }
}

fn dup_of(old: i64, r: i64) -> i64 {
    if r >= 0 {
        mark(r, marked(old));
    }
    r
}

fn uname(out: i64) -> i64 {
    let mut u = [0u8; 390];
    // SAFETY: uname writes 390 bytes.
    let r = unsafe { sc(UNAME, u.as_mut_ptr() as i64, 0, 0) };
    if r < 0 {
        return r;
    }
    fill_uname(&mut u);
    if out == 0 {
        return -sys::EFAULT;
    }
    // SAFETY: the payload gave out as a struct utsname.
    unsafe { core::ptr::copy_nonoverlapping(u.as_ptr(), out as *mut u8, u.len()) };
    0
}

fn write(n: i64, a: [i64; 6]) -> i64 {
    let fd = a[0];
    let Some(size) = size_of(fd) else {
        // SAFETY: the payload's own call.
        return unsafe { sc6(n, a[0], a[1], a[2], a[3], a[4], a[5]) };
    };
    let grow = match n {
        FTRUNCATE => a[1] - size,
        FALLOCATE if a[1] & 1 == 0 => growth(a[2], a[3], size),
        FALLOCATE => 0,
        PWRITE64 => growth(a[3], a[2], size),
        PWRITEV | PWRITEV2 => growth(a[3], iov_len(a[1], a[2]), size),
        _ => {
            let len = if n == WRITEV {
                iov_len(a[1], a[2])
            } else {
                a[2]
            };
            // SAFETY: two queries of the descriptor.
            let append = unsafe { sc(nr::FCNTL, fd, F_GETFL, 0) } & O_APPEND != 0;
            let at = if append {
                size
            } else {
                unsafe { sc(LSEEK, fd, 0, SEEK_CUR) }.max(0)
            };
            growth(at, len, size)
        }
    };
    let count = used();
    let now = count.map_or(0, |c| c.load(Ordering::Relaxed));
    if !fits(now, grow, cap()) {
        return -ENOSPC;
    }
    // SAFETY: the payload's own call.
    let r = unsafe { sc6(n, a[0], a[1], a[2], a[3], a[4], a[5]) };
    if r >= 0
        && let (Some(c), Some(after)) = (count, size_of(fd))
    {
        c.fetch_add(after - size, Ordering::Relaxed);
    }
    r
}

/// Give back the bytes of a file that an `unlink` under `/tmp` removed.
pub fn unlinked(bytes: i64) {
    if let Some(c) = used() {
        c.fetch_sub(bytes, Ordering::Relaxed);
    }
}

/// The bytes of the regular file at the `O_PATH` descriptor `fd` where it
/// has one link, else 0: what its `unlink` gives back.
pub fn freed_by_unlink(fd: i64) -> i64 {
    let mut st = [0u8; 144];
    // SAFETY: fstat writes a struct stat of 144 bytes.
    if unsafe { sc(FSTAT, fd, st.as_mut_ptr() as i64, 0) } < 0 {
        return 0;
    }
    let nlink = u64::from_ne_bytes(st[16..24].try_into().unwrap_or([0; 8]));
    let mode = u32::from_ne_bytes(st[24..28].try_into().unwrap_or([0; 4]));
    if nlink == 1 && mode & 0o170000 == 0o100000 {
        i64::from_ne_bytes(st[48..56].try_into().unwrap_or([0; 8]))
    } else {
        0
    }
}
