//! The guest names of the sockets bound through the long form, T-1710.
//!
//! A socket bound as `/proc/self/fd/N/NAME` keeps that name in the
//! kernel, and the descriptor `N` is closed after the bind. So the
//! interposer keeps each such bind here: the kernel's name and the guest
//! name. `getsockname` and the other answers look the kernel's name up.
//!
//! ⛔ No allocation and no lock: a fixed ring of slots, each guarded by a
//! sequence number; each byte is an atomic, so a reader that races a
//! writer reads a torn copy and drops it, never undefined memory. The
//! table belongs to one process: a `fork` copies it, an `exec` starts it
//! empty.

use core::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering};

use super::SUN_PATH;

const SLOTS: usize = 32;

struct Slot {
    /// Odd while a writer writes.
    seq: AtomicU32,
    kernel_n: AtomicU8,
    guest_n: AtomicU8,
    kernel: [AtomicU8; SUN_PATH],
    guest: [AtomicU8; SUN_PATH],
}

#[allow(clippy::declare_interior_mutable_const)]
const SLOT: Slot = Slot {
    seq: AtomicU32::new(0),
    kernel_n: AtomicU8::new(0),
    guest_n: AtomicU8::new(0),
    kernel: [const { AtomicU8::new(0) }; SUN_PATH],
    guest: [const { AtomicU8::new(0) }; SUN_PATH],
};

static TABLE: [Slot; SLOTS] = [SLOT; SLOTS];
static NEXT: AtomicUsize = AtomicUsize::new(0);

/// Keep `kernel` (the bound name) and `guest` (the name the payload gave).
/// The oldest slot goes first.
pub fn keep(kernel: &[u8], guest: &[u8]) {
    if kernel.len() >= SUN_PATH || guest.len() >= SUN_PATH {
        return;
    }
    let s = &TABLE[NEXT.fetch_add(1, Ordering::Relaxed) % SLOTS];
    let start = s.seq.load(Ordering::Acquire);
    if start % 2 == 1
        || s.seq
            .compare_exchange(start, start + 1, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
    {
        return; // another writer has the slot: this bind stays unnamed
    }
    for (d, b) in s.kernel.iter().zip(kernel) {
        d.store(*b, Ordering::Relaxed);
    }
    for (d, b) in s.guest.iter().zip(guest) {
        d.store(*b, Ordering::Relaxed);
    }
    s.kernel_n.store(kernel.len() as u8, Ordering::Relaxed);
    s.guest_n.store(guest.len() as u8, Ordering::Relaxed);
    s.seq.store(start + 2, Ordering::Release);
}

/// The guest name of the kernel's name `kernel` into `out`; its length.
pub fn find(kernel: &[u8], out: &mut [u8; SUN_PATH]) -> Option<usize> {
    for s in &TABLE {
        let start = s.seq.load(Ordering::Acquire);
        if start == 0 || start % 2 == 1 {
            continue;
        }
        let kn = s.kernel_n.load(Ordering::Relaxed) as usize;
        if kn != kernel.len()
            || s.kernel[..kn]
                .iter()
                .zip(kernel)
                .any(|(a, b)| a.load(Ordering::Relaxed) != *b)
        {
            continue;
        }
        let gn = s.guest_n.load(Ordering::Relaxed) as usize;
        for (o, a) in out.iter_mut().zip(&s.guest[..gn]) {
            *o = a.load(Ordering::Relaxed);
        }
        if s.seq.load(Ordering::Acquire) == start {
            return Some(gn);
        }
    }
    None
}
