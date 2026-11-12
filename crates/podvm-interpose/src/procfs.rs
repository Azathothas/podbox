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
//! wrappers fall back to the real call and its honest failure. The
//! exception is `/proc/self/exe` under the override (T-1739): there the
//! real call names the loader, a wrong answer, so the answer stays and the
//! tally is best effort.
//!
//! ⛔ **No allocation and no locks on this path**, T-0701's constraint 2.
//! Every buffer here is on the caller's stack, including the fixture.

use core::ffi::{c_char, c_int, c_uint, c_void};

/// The variable carrying the payload's resolved guest path into the payload.
///
/// Set beside the exec by `podvm` (which owns the rootfs host path),
/// read here with no libc call, like `map` and the memo do. A duplicate of
/// `podvm-enter`'s `ladder::GUEST_EXE_VAR` by value, the same way
/// `memo::MEMO_FD_VAR` duplicates the host's spelling: this object takes no
/// dependency on the rest of the tree.
pub const GUEST_EXE_VAR: &[u8] = b"PODVM_GUEST_EXE";

/// The flag gating the loader-family exe override, T-1409.
///
/// Set to `1` by `podvm` on loader-family userland entries (and only
/// there): entering through the loader leaves `/proc/self/exe` pointing at
/// the loader, so the payload would see the loader's path instead of its
/// own. Where set, the `/proc/self/exe` shapes answer from [`GUEST_EXE_VAR`]
/// even where the real call succeeds (host `/proc` mounted). Where absent
/// everything behaves as before: the emulation answers only where the real
/// call fails, so a stale or foreign value can never poison a truth the
/// kernel still answers.
///
/// ⛔ The substitution itself says nothing here: it fires on every matching
/// call, and a line per call would flood the payload's stderr. The CLI names
/// it once on the banner, which is the banner-visible half T-1409 requires.
pub const GUEST_EXE_FORCE_VAR: &[u8] = b"PODVM_GUEST_EXE_FORCE";

/// Whether the flag value asks for the override: exactly `1`.
///
/// Pure over the value, so the wrappers and the unit test read the same
/// words. Anything else (absent, empty, another spelling) is off: an
/// accidental value must never reroute what the payload sees itself as.
pub fn parse_exe_override(value: &[u8]) -> bool {
    value == b"1"
}

/// Is the loader-family override on for this process?
///
/// Read straight out of the process's own environ, the way `map` reads its
/// table: no libc call, no lock, no allocation.
///
/// # Safety
/// Reads the process's environ, which the kernel keeps valid for the
/// process's life.
pub(crate) unsafe fn exe_override_enabled() -> bool {
    let Some((p, n)) = (unsafe { crate::map::lookup_env(GUEST_EXE_FORCE_VAR) }) else {
        return false;
    };
    parse_exe_override(unsafe { core::slice::from_raw_parts(p, n) })
}

mod exe;
pub use exe::*;

mod mounts;
pub use mounts::*;

#[cfg(test)]
mod tests;
