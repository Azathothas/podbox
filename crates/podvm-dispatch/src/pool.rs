//! Path buffers for the handler, out of the payload's stack.
//!
//! ⛔ The handler runs on whatever stack the payload's thread has: a child of
//! musl's `posix_spawn` has about 5 KiB. A buffer of `PATH_MAX` on that stack
//! wrote over the parent's frame (measured: the exec of such a child
//! faulted at address 59). The handler also cannot move to a stack of its
//! own: a nested signal with `SA_ONSTACK` would then start at the top of the
//! alternate stack, over the frame of the SIGSYS. So the buffers come from
//! this pool: a bitmap of 128 buffers, taken with a compare-and-swap, and an
//! anonymous map of its own for each buffer past that.

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::state::PATH_MAX;
use crate::sys::{nr, sc, sc6};

const SLOTS: usize = 128;
static BASE: AtomicUsize = AtomicUsize::new(0);
static USED: [AtomicU64; SLOTS / 64] = [const { AtomicU64::new(0) }; SLOTS / 64];

/// Reserve the pool. Call it before the payload runs.
pub fn init() -> bool {
    // SAFETY: an anonymous map with no reserve.
    let m = unsafe {
        sc6(
            nr::MMAP,
            0,
            (SLOTS * PATH_MAX) as i64,
            3,
            0x22 | 0x4000,
            -1,
            0,
        )
    };
    if m < 0 {
        return false;
    }
    BASE.store(m as usize, Ordering::Release);
    true
}

/// One buffer of `PATH_MAX` bytes, given back on drop.
pub struct Buf {
    at: *mut [u8; PATH_MAX],
    slot: usize,
}

impl Buf {
    /// A buffer, or None where no memory is left.
    pub fn get() -> Option<Buf> {
        let base = BASE.load(Ordering::Acquire);
        if base != 0 {
            for (w, word) in USED.iter().enumerate() {
                let mut cur = word.load(Ordering::Relaxed);
                while cur != u64::MAX {
                    let bit = (!cur).trailing_zeros() as usize;
                    match word.compare_exchange_weak(
                        cur,
                        cur | 1 << bit,
                        Ordering::Acquire,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => {
                            let slot = w * 64 + bit;
                            return Some(Buf {
                                at: (base + slot * PATH_MAX) as *mut [u8; PATH_MAX],
                                slot,
                            });
                        }
                        Err(now) => cur = now,
                    }
                }
            }
        }
        // SAFETY: an anonymous map of one buffer.
        let m = unsafe { sc6(nr::MMAP, 0, PATH_MAX as i64, 3, 0x22, -1, 0) };
        (m >= 0).then_some(Buf {
            at: m as *mut [u8; PATH_MAX],
            slot: usize::MAX,
        })
    }

    pub fn buf(&mut self) -> &mut [u8; PATH_MAX] {
        // SAFETY: the slot is this guard's alone until drop.
        unsafe { &mut *self.at }
    }
}

impl Drop for Buf {
    fn drop(&mut self) {
        if self.slot == usize::MAX {
            // SAFETY: the map that `get` made for this buffer.
            unsafe { sc(nr::MUNMAP, self.at as i64, PATH_MAX as i64, 0) };
            return;
        }
        USED[self.slot / 64].fetch_and(!(1 << (self.slot % 64)), Ordering::Release);
    }
}
