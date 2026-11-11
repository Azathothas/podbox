//! What the handler reads: written one time before the payload runs, read
//! after that by each thread with no lock.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

pub const PATH_MAX: usize = 4096;

/// A path of at most `PATH_MAX - 1` bytes, written one time.
pub struct Fixed {
    buf: UnsafeCell<[u8; PATH_MAX]>,
    len: AtomicUsize,
}

// SAFETY: written before the payload starts (one thread), then only read.
unsafe impl Sync for Fixed {}

impl Fixed {
    pub const fn new() -> Fixed {
        Fixed {
            buf: UnsafeCell::new([0; PATH_MAX]),
            len: AtomicUsize::new(0),
        }
    }

    /// Set the value. Call it before any payload code runs.
    pub fn set(&self, v: &[u8]) -> bool {
        if v.len() >= PATH_MAX {
            return false;
        }
        // SAFETY: no reader runs yet (the contract of `set`).
        let b = unsafe { &mut *self.buf.get() };
        b[..v.len()].copy_from_slice(v);
        b[v.len()] = 0;
        self.len.store(v.len(), Ordering::Release);
        true
    }

    /// The value with no NUL.
    pub fn get(&self) -> &[u8] {
        let n = self.len.load(Ordering::Acquire);
        // SAFETY: the bytes before len are fixed after `set`.
        let b = unsafe { &*self.buf.get() };
        b.get(..n).unwrap_or(&[])
    }

    /// The value with its NUL.
    pub fn get_z(&self) -> &[u8] {
        let n = self.len.load(Ordering::Acquire);
        // SAFETY: as in `get`; `set` wrote the NUL at n.
        let b = unsafe { &*self.buf.get() };
        b.get(..n + 1).unwrap_or(&[0])
    }
}

/// The host path of the root, canonical, with no trailing slash.
pub static ROOT: Fixed = Fixed::new();
/// The guest path of the payload (`/proc/self/exe` answers it).
pub static EXE: Fixed = Fixed::new();
/// The host path of podvm itself, for the re-exec of `execve`.
pub static SELF: Fixed = Fixed::new();
/// An `O_PATH` descriptor of the root, above the payload's usual range.
pub static ROOT_FD: AtomicI64 = AtomicI64::new(-1);
/// The region of podvm's own code: its system calls do not dispatch.
pub static REGION_START: AtomicUsize = AtomicUsize::new(0);
pub static REGION_LEN: AtomicUsize = AtomicUsize::new(0);
/// The payload's program break: the base, the current end and the end of
/// the reserved range.
pub static BRK_BASE: AtomicUsize = AtomicUsize::new(0);
pub static BRK_CUR: AtomicUsize = AtomicUsize::new(0);
pub static BRK_END: AtomicUsize = AtomicUsize::new(0);

pub fn root_fd() -> i64 {
    ROOT_FD.load(Ordering::Relaxed)
}

/// TODO/enter.md T-1724. The volumes of the run: a guest prefix, the host
/// directory, and an `O_PATH` descriptor of it, from `PODVM_MAPS`.
pub const MAX_VOLS: usize = 16;

pub struct Vol {
    pub guest: Fixed,
    pub host: Fixed,
    pub fd: AtomicI64,
}

#[allow(clippy::declare_interior_mutable_const)]
const NO_VOL: Vol = Vol {
    guest: Fixed::new(),
    host: Fixed::new(),
    fd: AtomicI64::new(-1),
};
pub static VOLS: [Vol; MAX_VOLS] = [NO_VOL; MAX_VOLS];
pub static NVOLS: AtomicUsize = AtomicUsize::new(0);

/// The volumes that are set.
pub fn vols() -> &'static [Vol] {
    &VOLS[..NVOLS.load(Ordering::Acquire).min(MAX_VOLS)]
}
