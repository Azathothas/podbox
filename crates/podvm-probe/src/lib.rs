//! `TOOL.md` section 6.1: the probe set, disposable children, mode selection.
//! The work is in [`TODO/probe.md`](../../../TODO/probe.md), T-0101 to T-0110.
//!
//! `pod-hostprobe` (PodUtils/podcrates) runs the probes: the raw system
//! calls, the disposable children, the probe table, the selection of the
//! rung, the mount, machine and supervise legs, the identity and the write
//! probe. This crate gives podvm's settings to it, and keeps podvm's report,
//! exit codes, JSON writer and non-goals.
//!
//! ⭐ **Four rules decide whether the probes are right rather than merely
//! finished**, and `pod-hostprobe` keeps each one:
//!
//! 1. every probe runs in a disposable child ([`child`], T-0101);
//! 2. the errno is recorded, never a boolean ([`sys::Errno`], T-0101);
//! 3. "could not run" is a third state and never reads as "denied"
//!    ([`verdict::Verdict::Skip`], T-0109);
//! 4. the controls travel with the bogus-argument discriminator, so a probe
//!    that has stopped discriminating says so ([`select::Selection`], T-0102).
#![forbid(unsafe_op_in_unsafe_fn)]

pub mod exit;
pub mod inside;
pub mod json;
pub mod nongoals;
pub mod report;
pub mod select;

pub use pod_hostprobe::{
    binfmt, child, identity, interp, machine, mounts, probes, supervise, sys, verdict, writable,
    Findings, Settings,
};

/// The private argument a probe child is re-executed with.
///
/// ⚠ It is not a public verb. `podvm probe --help` names it as internal, and
/// the CLI dispatches it before anything else so a child cannot fall into the
/// ordinary argument surface.
pub const CHILD_FLAG: &str = "--probe-child";

/// podvm's values for `pod-hostprobe`: the child flag, the `argv[0]` of a
/// probe child, and the variable that the space and exec messages name.
pub fn settings() -> Settings {
    Settings {
        child_flag: CHILD_FLAG.to_string(),
        child_name: "podvm".to_string(),
        directory_setting: Some("$PODVM_STORE".to_string()),
    }
}

/// The directories of the write probe: the fixed list, then `$PODVM_STORE`
/// and each of [`writable::VARIABLES`] that is set, each with the variable
/// that named it, then the working directory.
pub fn candidates() -> Vec<(String, String)> {
    candidates_from(|var| std::env::var_os(var))
}

fn candidates_from(get: impl Fn(&str) -> Option<std::ffi::OsString>) -> Vec<(String, String)> {
    let named: Vec<(String, String)> = std::iter::once("PODVM_STORE")
        .chain(writable::VARIABLES.iter().copied())
        .filter_map(|var| get(var).map(|v| (v.to_string_lossy().to_string(), format!("${var}"))))
        .collect();
    writable::candidates(&named)
}

/// Is an emulator in the way, from the variables of this process.
pub fn interpreter() -> interp::Interpreter {
    interp::detect(&|var| std::env::var_os(var).is_some())
}

/// The cache key of `id` with the emulator check of this process.
pub fn confinement_key(id: &identity::Identity) -> identity::ConfinementKey {
    identity::confinement_key(id, &interpreter())
}

/// Run the whole set with podvm's settings. One disposable child per probe,
/// in a fixed order.
pub fn run() -> Findings {
    pod_hostprobe::run(&settings(), &candidates())
}

/// What a probe child does: run exactly one probe, in this process, and report
/// it on stdout and in the exit status.
///
/// ⛔ Both, and they must agree. [`child::Spawner`] refuses a child whose line
/// and exit status disagree, because a child that contradicts itself has
/// established no verdict. `TODO/probe.md` T-0109 rule 1.
pub fn run_child(name: &str) -> i32 {
    if probes::find(name).is_none() {
        eprintln!("podvm: no probe named {name:?}");
        return 2;
    }
    pod_hostprobe::run_child(name, &mut std::io::stdout().lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_probe_name_that_does_not_exist_exits_two() {
        assert_eq!(run_child("no such probe"), 2);
    }

    #[test]
    fn the_settings_are_podvms() {
        let s = settings();
        assert_eq!(s.child_flag, "--probe-child");
        assert_eq!(s.child_name, "podvm");
        assert_eq!(s.directory_setting.as_deref(), Some("$PODVM_STORE"));
    }

    #[test]
    fn the_store_is_the_first_named_candidate() {
        let env = |var: &str| match var {
            "PODVM_STORE" => Some("/podvm-store".into()),
            "TMPDIR" => Some("/podvm-tmp".into()),
            _ => None,
        };
        let got = candidates_from(env);
        let store = got
            .iter()
            .position(|c| *c == ("/podvm-store".into(), "$PODVM_STORE".into()));
        let tmp = got
            .iter()
            .position(|c| *c == ("/podvm-tmp".into(), "$TMPDIR".into()));
        assert!(store.is_some() && store < tmp, "{got:?}");
        assert_eq!(got[0].0, "/tmp", "the fixed list comes first: {got:?}");
    }
}
