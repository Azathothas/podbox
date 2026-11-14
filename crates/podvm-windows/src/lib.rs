//! The Windows guest driver: a disposable Validation OS machine driven
//! through a FAT mailbox, and the DOS flavor. `TODO/milestones.md` T-1112.
//!
//! The driver is `pod-guest` of PodUtils/podcrates: the boot plan on the
//! machine tier's own profile (`tcg` where `kvm` is missing), the per-run
//! overlay over a read-only base, the FAT16 mailbox built in process, the
//! `cmd.exe` agent that a scheduled task starts, the DOS console line, QMP,
//! the bounded stop of the emulator's process group, and the bounded store
//! of a base image with its pinned digest.
//!
//! This crate gives podvm's names and places to it ([`settings`]): the
//! per-run directory `podvm-windows-PID-TAG` under `$XDG_RUNTIME_DIR`,
//! `PODVM_WINDOWS_KEEP`, the marker `podvm` (`podvm-dos` for DOS) and the
//! OEM name `PODVM`. It keeps the base caches (`PODVM_WINDOWS_BASE`,
//! `PODVM_DOS_BASE`) and docker's range of exit codes ([`cap_status`]).
//! Each function keeps its podvm signature; a failure is a
//! `pod_core::Failure` whose text is the text of the old error.

// Edition 2024 makes `std::env::set_var` and `remove_var` unsafe. The tests
// change variables of the process, so a test build denies unsafe code in
// place of forbidding it, and only the test modules allow it.
#![cfg_attr(not(test), forbid(unsafe_code))]
#![cfg_attr(test, deny(unsafe_code))]

use std::path::{Path, PathBuf};
use std::time::Duration;

pub use pod_guest::{
    accel_for, agent, argv, convert_argv, emulator, fat16, fetch, outcome, overlay_argv, plan, qmp,
    scratch_of, stop, timeout_of, Accel, Ceiling, Code, Fat16, Outcome, Plan, Request, Result,
    Stop, Timeout,
};

/// The prefix every per-run directory carries, used both to create one and
/// to be sure of one before removing it.
pub const SCRATCH_PREFIX: &str = "podvm-windows-";

/// podvm's values for `pod-guest`: the per-run directories under
/// `$XDG_RUNTIME_DIR` (the temporary directory where it is not set), with
/// [`SCRATCH_PREFIX`]; `PODVM_WINDOWS_KEEP` keeps them; the marker `podvm`
/// and the OEM name `PODVM` on the mailbox.
pub fn settings() -> pod_guest::Settings {
    let root = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let mut s = pod_guest::Settings::new(root);
    s.scratch_prefix = SCRATCH_PREFIX.to_string();
    s.keep = std::env::var_os("PODVM_WINDOWS_KEEP").is_some();
    s.marker = b"podvm".to_vec();
    s.oem = "PODVM".to_string();
    s
}

/// The mailbox a run is handed: `pod_guest::mailbox` with [`settings`].
pub fn mailbox(agent_cmd: &str, request: &Request) -> Result<Fat16> {
    pod_guest::mailbox(&settings(), agent_cmd, request)
}

/// Stage one run: `pod_guest::stage` with [`settings`].
pub fn stage(
    qemu_img: &Path,
    base: &Path,
    plan: &Plan,
    agent_cmd: &str,
    request: &Request,
) -> Result<Fat16> {
    pod_guest::stage(&settings(), qemu_img, base, plan, agent_cmd, request)
}

/// Boot the guest and read the result: `pod_guest::run` with [`settings`].
/// A run that times out is a `Timeout` failure ([`timeout_of`]).
pub fn run(plan: &Plan, request: &Request) -> Result<Outcome> {
    pod_guest::run(&settings(), plan, request)
}

/// Provision a fresh image: `pod_guest::provision` with [`settings`].
pub fn provision(
    qemu_img: &Path,
    base: &Path,
    plan: &Plan,
    out: &Path,
    wait: Duration,
) -> Result<Vec<u8>> {
    // podcrates `v0.45.0`: the keys go to the console once the screen of
    // the boot settles (`Ready::screen`), within this bound (an estimate
    // of a slow boot under TCG), and not after a fixed sleep (TODO/machine.md
    // T-1873).
    let ready = pod_guest::ready::Ready::screen(PROVISION_READY);
    pod_guest::provision(&settings(), qemu_img, base, plan, out, &ready, wait)
}

/// The bound of the wait for a provision boot to be ready for the keys.
pub const PROVISION_READY: Duration = Duration::from_secs(600);

/// Remove a per-run directory unless `PODVM_WINDOWS_KEEP` is set.
pub fn cleanup(plan: &Plan) {
    pod_guest::cleanup(&settings(), plan)
}

/// A per-run directory, mode 0700, `podvm-windows-PID-TAG`.
pub fn scratch_dir(tag: &str) -> Result<PathBuf> {
    pod_guest::scratch_dir(&settings(), tag)
}

/// A per-run directory that removes itself when it falls out of scope.
pub struct RunGuard<'a> {
    _guard: pod_guest::RunGuard<'a>,
}

impl<'a> RunGuard<'a> {
    /// Arm the guard for `plan`. Name the binding so it lives to the end
    /// of the run: a guard dropped at once guards nothing.
    pub fn arm(plan: &'a Plan) -> Self {
        RunGuard {
            _guard: pod_guest::RunGuard::arm(&settings(), plan),
        }
    }
}

/// ⭐ Docker's exit range is 0-124, and 125 is `podvm` itself failing. A
/// guest status of 125 or above would therefore read as a podvm failure,
/// so it is capped rather than passed through: `None` is "this status
/// cannot be returned as-is", and the caller names it.
pub fn cap_status(code: i32) -> Option<i32> {
    (0..=124).contains(&code).then_some(code)
}

/// The cached base image: `PODVM_WINDOWS_BASE` where it is set, so an
/// experiment isolates with its own copy, else under the home directory so
/// it survives between runs the way an image cache should.
pub fn base_cache() -> PathBuf {
    if let Some(p) = std::env::var_os("PODVM_WINDOWS_BASE")
        && !p.is_empty()
    {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME").unwrap_or_else(|| "/tmp".into());
    Path::new(&home).join(".local/share/podvm/windows/base.vhdx")
}

/// The DOS flavor: `pod_guest::dos` with podvm's marker and base cache.
pub mod answer; // TODO/machine.md T-1866
pub mod catalog; // TODO/machine.md T-1861
pub mod media; // TODO/machine.md T-1866
pub mod options; // TODO/machine.md T-1865

pub mod dos {
    use std::path::{Path, PathBuf};

    pub use pod_guest::dos::{
        argv, batch_from_argv, run, BAT, CPUS, DEFAULT_MEM_MIB, DEFAULT_TIMEOUT_SECS, MBOX_DRIVE,
        TO_MENU_MS, TO_PROCEED_MS, TO_PROMPT_MS,
    };

    use crate::{Plan, Result};

    /// Stage one DOS run: `pod_guest::dos::stage` with podvm's settings and
    /// the marker `podvm-dos`.
    pub fn stage(
        qemu_img: &Path,
        base: &Path,
        plan: &Plan,
        batch: &str,
        token: &str,
    ) -> Result<()> {
        let mut s = crate::settings();
        s.marker = b"podvm-dos".to_vec();
        pod_guest::dos::stage(&s, qemu_img, base, plan, batch, token)
    }

    /// The DOS base image: `PODVM_DOS_BASE` where it is set, so an experiment
    /// isolates with its own copy, else under the home directory beside the
    /// other flavor's cache so both persist between runs the same way.
    ///
    /// ⛔ **Not `PODVM_WINDOWS_BASE`.** That name is the other flavor's cache,
    /// and one variable naming two different disk images is the variant-keyed
    /// cache defect `docs/conventions/forbidden-patterns.md` refuses: a DOS
    /// run would boot a Windows disk and report its output as DOS's.
    pub fn base_cache() -> PathBuf {
        if let Some(p) = std::env::var_os("PODVM_DOS_BASE")
            && !p.is_empty()
        {
            return PathBuf::from(p);
        }
        let home = std::env::var_os("HOME").unwrap_or_else(|| "/tmp".into());
        Path::new(&home).join(".local/share/podvm/windows/freedos-base.img")
    }

    #[cfg(test)]
    #[allow(unsafe_code)]
    mod tests {
        use super::*;

        #[test]
        fn the_dos_cache_is_not_the_windows_cache() {
            // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
            unsafe { std::env::set_var("PODVM_DOS_BASE", "/tmp/dos-base.img") };
            // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
            unsafe { std::env::set_var("PODVM_WINDOWS_BASE", "/tmp/win-base.vhdx") };
            assert_eq!(base_cache(), PathBuf::from("/tmp/dos-base.img"));
            assert_eq!(crate::base_cache(), PathBuf::from("/tmp/win-base.vhdx"));
            assert_ne!(base_cache(), crate::base_cache());
            // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
            unsafe { std::env::remove_var("PODVM_DOS_BASE") };
            // SAFETY: only a test changes this variable; edition 2024 marks the change unsafe because another thread can read the environment at the same time.
            unsafe { std::env::remove_var("PODVM_WINDOWS_BASE") };
            let p = base_cache();
            assert!(p.to_string_lossy().ends_with("freedos-base.img"), "{p:?}");
        }
    }
}

#[cfg(test)]
mod download_tests;
#[cfg(test)]
#[allow(unsafe_code)]
mod tests; // TODO/machine.md T-1862
