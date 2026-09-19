//! The identity memo: who the payload believes it is.
//!
//! [`TODO/interpose.md`](../../../TODO/interpose.md) T-0711. A payload that
//! is already uid 0 still calls `setuid`, and on the runtimes podbox targets
//! that fails. The operator ruled answer 3: the call fails as the runtime
//! fails it by default, and `--user` turns on the fakeroot behaviour for a
//! payload that needs it.
//!
//! ⭐ **The memo is the environment, not a file.** `podbox-cli` sets
//! `PODBOX_IDENTITY` to `UID[:GID]` where the caller passed `--user`, and
//! the numbers are already resolved there (names never reach this object).
//! The per-process record is two atomics, initialized from the variable on
//! first use: inherited across `fork` by copying, re-read across `execve` by
//! the variable surviving it. No locks, no allocation, no daemon.
//!
//! ⛔ **The lie is marked everywhere it shows.** A faked call says one line
//! unconditionally (a behaviour change inside somebody else's process is a
//! refusal-shaped event), the banner names the flag, and the container is
//! degraded so `--strict` refuses it. Without the variable every function
//! below calls through and reports failure exactly as the runtime fails it.

use core::ffi::{c_int, c_uint};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// The variable that carries the requested identity.
pub const IDENTITY_VAR: &[u8] = b"PODBOX_IDENTITY";

/// `(uid_t)-1` leaves the id alone, which is every `*id` call's own
/// convention and the value the entry points test for before recording.
pub const NO_ID: u32 = u32::MAX;

/// The recorded identity: real, effective and saved user and group ids.
///
/// ⭐ Three each rather than one, because `setreuid` splits them and
/// `getresuid` reports all three: collapsing them would answer `id` wrong.
/// Initialized from the variable on first use; `fork` copies the values and
/// `execve` re-reads the variable, which matches except where the payload
/// moved one mid-process and then executed, and that divergence is written
/// down at the entry points.
static RUID: AtomicU32 = AtomicU32::new(0);
static EUID: AtomicU32 = AtomicU32::new(0);
static SUID: AtomicU32 = AtomicU32::new(0);
static RGID: AtomicU32 = AtomicU32::new(0);
static EGID: AtomicU32 = AtomicU32::new(0);
static SGID: AtomicU32 = AtomicU32::new(0);
static READ: AtomicBool = AtomicBool::new(false);

/// The recorded supplementary groups, and how many. Bounded: sixteen is
/// twice what `NGROUPS_MAX` guarantees anywhere (`sysconf` says 65536 on
/// Linux, and no payload passes that many through `setgroups` to have them
/// reported back).
static GROUPS: [AtomicU32; 16] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];
static NGROUPS: AtomicU32 = AtomicU32::new(0);

/// Parse `UID[:GID]` of decimal digits. A missing group means the user's own:
/// `docker --user` resolves it the same way, and inventing another group's id
/// here would be a second opinion about the image's `/etc/group`.
pub fn parse(spec: &[u8]) -> Option<(u32, u32)> {
    fn num(b: &[u8]) -> Option<u32> {
        if b.is_empty() || b.len() > 10 {
            return None;
        }
        let mut v: u32 = 0;
        for c in b {
            if !c.is_ascii_digit() {
                return None;
            }
            v = v.checked_mul(10)?.checked_add((c - b'0') as u32)?;
        }
        Some(v)
    }
    match spec.iter().position(|c| *c == b':') {
        None => {
            let u = num(spec)?;
            Some((u, u))
        }
        Some(at) => {
            let u = num(&spec[..at])?;
            let g = num(&spec[at + 1..])?;
            Some((u, g))
        }
    }
}

/// The requested identity, where the caller set one.
fn wanted() -> Option<(u32, u32)> {
    // ⭐ Read straight out of the process's own environ, the way `map`
    // reads its table: no libc call, no lock, no allocation.
    let (p, n) = unsafe { crate::map::lookup_env(IDENTITY_VAR) }?;
    if n == 0 {
        return None;
    }
    let bytes = unsafe { core::slice::from_raw_parts(p, n) };
    parse(bytes)
}

/// Is the fakeroot behaviour on for this process?
pub fn enabled() -> bool {
    wanted().is_some()
}

/// Read the variable once per process image. Racy by design, like the
/// `dlsym` cache: two threads initializing write the same numbers, so the
/// race costs a parse and is correct.
fn ensure() {
    if READ.load(Ordering::Acquire) {
        return;
    }
    if let Some((u, g)) = wanted() {
        RUID.store(u, Ordering::Release);
        EUID.store(u, Ordering::Release);
        SUID.store(u, Ordering::Release);
        RGID.store(g, Ordering::Release);
        EGID.store(g, Ordering::Release);
        SGID.store(g, Ordering::Release);
    }
    READ.store(true, Ordering::Release);
}

fn store(slot: &AtomicU32, v: u32) {
    slot.store(v, Ordering::Release);
}

/// Record what the payload asked to become: each of real, effective and
/// saved user id, where the call sets it. `NO_ID` leaves one alone, which is
/// the call's own `-1` convention.
pub fn set_user(ruid: u32, euid: u32, saved: u32) {
    ensure();
    if ruid != NO_ID {
        store(&RUID, ruid);
    }
    if euid != NO_ID {
        store(&EUID, euid);
    }
    if saved != NO_ID {
        store(&SUID, saved);
    }
}

/// Record what the payload asked its groups to become, same convention.
pub fn set_group(rgid: u32, egid: u32, saved: u32) {
    ensure();
    if rgid != NO_ID {
        store(&RGID, rgid);
    }
    if egid != NO_ID {
        store(&EGID, egid);
    }
    if saved != NO_ID {
        store(&SGID, saved);
    }
}

/// Record a supplementary list, truncated at sixteen with the count kept
/// honest: `getgroups` below reports how many there are, not how many fit.
/// `count` is the caller's own number, read beside the list.
pub fn set_groups(list: &[u32], count: u32) {
    ensure();
    for (i, g) in list.iter().enumerate() {
        if i >= GROUPS.len() {
            break;
        }
        GROUPS[i].store(*g, Ordering::Relaxed);
    }
    NGROUPS.store(count, Ordering::Release);
}

/// What `getuid` answers under the flag.
pub fn ruid() -> u32 {
    ensure();
    RUID.load(Ordering::Acquire)
}

/// What `geteuid` answers under the flag.
pub fn euid() -> u32 {
    ensure();
    EUID.load(Ordering::Acquire)
}

/// What `getresuid` reports as saved under the flag.
pub fn saved_uid() -> u32 {
    ensure();
    SUID.load(Ordering::Acquire)
}

/// What `getgid` answers under the flag.
pub fn rgid() -> u32 {
    ensure();
    RGID.load(Ordering::Acquire)
}

/// What `getegid` answers under the flag.
pub fn egid() -> u32 {
    ensure();
    EGID.load(Ordering::Acquire)
}

/// What `getresgid` reports as saved under the flag.
pub fn saved_gid() -> u32 {
    ensure();
    SGID.load(Ordering::Acquire)
}

/// How many supplementary groups the record holds.
pub fn ngroups() -> u32 {
    ensure();
    NGROUPS.load(Ordering::Acquire)
}

/// Copy up to `len` recorded groups into `out`, returning how many there
/// are. That is `getgroups(2)`'s own contract: the return is the count even
/// where the buffer holds fewer.
///
/// # Safety
/// `out` must point to room for at least `len` `c_uint`, or be null where
/// `len` is not positive, which is the payload's own contract for
/// `getgroups(2)`.
pub unsafe fn groups(out: *mut c_uint, len: c_int) -> c_int {
    ensure();
    let have = NGROUPS.load(Ordering::Acquire);
    if len > 0 && !out.is_null() {
        let mut i = 0u32;
        while i < have && i < GROUPS.len() as u32 && i < len as u32 {
            unsafe { *out.add(i as usize) = GROUPS[i as usize].load(Ordering::Relaxed) };
            i += 1;
        }
    }
    have as c_int
}

extern "C" {
    fn __errno_location() -> *mut c_int;
}

/// Try the real call first: where the runtime grants it, the kernel's answer
/// and the record agree, which is more honest than a lie that happens to
/// match. Where it fails with the wall's errno, record the intent and answer
/// success, saying so unconditionally. `record` runs on both paths so the
/// record tracks what was asked.
///
/// # Safety
/// `call` must uphold the real syscall's own contract, and `what` must stay
/// alive for the diagnostic: both are the entry point's own arguments.
pub unsafe fn attempt(what: &[u8], call: impl FnOnce() -> c_int, record: impl FnOnce()) -> c_int {
    let rc = call();
    if rc == 0 {
        record();
        return 0;
    }
    let e = unsafe { *__errno_location() };
    if !crate::is_the_wall(e) {
        return rc;
    }
    record();
    crate::say::line(&[
        what,
        b": the runtime refused it, and --user is faking the identity, so \
          podbox recorded the requested id and reported success \
          (TODO/interpose.md T-0711)",
    ]);
    unsafe { *__errno_location() = 0 };
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uid_alone_means_its_own_group() {
        assert_eq!(parse(b"1000"), Some((1000, 1000)));
        assert_eq!(parse(b"0"), Some((0, 0)));
    }

    #[test]
    fn uid_and_gid_parse() {
        assert_eq!(parse(b"1000:100"), Some((1000, 100)));
        assert_eq!(parse(b"0:0"), Some((0, 0)));
    }

    #[test]
    fn anything_else_is_not_an_identity() {
        assert_eq!(parse(b""), None);
        assert_eq!(parse(b"nobody"), None);
        assert_eq!(parse(b"1000:"), None);
        assert_eq!(parse(b":100"), None);
        assert_eq!(parse(b"1000:100:10"), None);
        assert_eq!(parse(b"4294967296"), None);
        assert_eq!(parse(b"-1"), None);
        assert_eq!(parse(b"0x10"), None);
    }
}
