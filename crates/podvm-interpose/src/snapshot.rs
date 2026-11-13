//! podvm's variables as the process started, TODO/interpose.md T-1756.
//!
//! ⛔ The table lives in the environment, and a program can drop it from
//! its own process: `env -i`, `clearenv` and `unsetenv` empty `environ`
//! before the exec. Before this copy, the table was then gone for the rest
//! of the process: its exec of `/bin/sh` passed to the host's shell. A
//! constructor of this object copies each `PODVM_*` entry and
//! `LD_PRELOAD` into static storage when the object loads; [`env`] gives
//! the copy as an `environ` image, and the table, [`crate::map::lookup_env`]
//! and the exec read it where `environ` no longer has the name.
//!
//! No allocation: one static buffer, filled once, before `main`.

use core::cell::UnsafeCell;
use core::ffi::c_char;
use core::sync::atomic::{AtomicBool, Ordering};

const BYTES: usize = 32 * 1024;
const MAX: usize = 32;

struct Snap {
    bytes: [u8; BYTES],
    ptrs: [*const c_char; MAX + 1],
}

struct Cell(UnsafeCell<Snap>);
// SAFETY: written once by the constructor, before any thread of the
// process exists; read only after `READY`.
unsafe impl Sync for Cell {}

static SNAP: Cell = Cell(UnsafeCell::new(Snap {
    bytes: [0; BYTES],
    ptrs: [core::ptr::null(); MAX + 1],
}));
static READY: AtomicBool = AtomicBool::new(false);

/// Whether `name` is a variable of podvm that the copy keeps.
pub fn ours(name: &[u8]) -> bool {
    name.starts_with(b"PODVM_") || name == b"LD_PRELOAD"
}

extern "C" fn take() {
    let env = crate::map::environ_ptr();
    if env.is_null() {
        return;
    }
    // SAFETY: the constructor runs once, before any reader.
    let s = unsafe { &mut *SNAP.0.get() };
    let (mut at, mut n, mut i) = (0usize, 0usize, 0usize);
    while i < 4096 && n < MAX {
        let e = unsafe { *env.add(i) };
        if e.is_null() {
            break;
        }
        i += 1;
        let len = crate::map::strnlen(e, BYTES);
        let entry = unsafe { core::slice::from_raw_parts(e as *const u8, len) };
        let Some(eq) = entry.iter().position(|&b| b == b'=') else {
            continue;
        };
        if !ours(&entry[..eq]) || at + len + 1 > BYTES {
            continue;
        }
        s.bytes[at..at + len].copy_from_slice(entry);
        s.bytes[at + len] = 0;
        s.ptrs[n] = unsafe { s.bytes.as_ptr().add(at) } as *const c_char;
        at += len + 1;
        n += 1;
    }
    READY.store(true, Ordering::Release);
}

#[used]
#[unsafe(link_section = ".init_array")]
static INIT: extern "C" fn() = take;

/// The copy as a null-terminated `environ` image, or null before the
/// constructor ran.
pub fn env() -> *const *const c_char {
    if !READY.load(Ordering::Acquire) {
        return core::ptr::null();
    }
    unsafe { (*SNAP.0.get()).ptrs.as_ptr() }
}
