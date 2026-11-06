//! `TODO/probe.md` T-0107: mode selection, and one switch that turns every
//! degradation into a refusal.
//!
//! `pod_hostprobe::select` chooses the rung. This module re-exports it and
//! keeps podvm's banner cells, which [`crate::report`] prints.

pub use pod_hostprobe::select::*;

use crate::verdict::Verdict;
use crate::Findings;

/// The probe rows the banner's `probes:` line quotes, in `TOOL.md`
/// section 6.8's order.
pub const BANNER_ROWS: &[&str] = &[
    "clone(CLONE_NEWNS)",
    "mount(tmpfs,/mnt)",
    "fsmount(tmpfs)",
    "move_mount(-> /tmp/mm-probe)",
    "clone(CLONE_NEWUTS)",
    "sethostname(in NEWUTS)",
    "chroot(/tmp)",
    "ptrace(PTRACE_TRACEME)",
];

/// `clone(CLONE_NEWNS)=ok`, `mount=EPERM`, `chroot=skip`: the banner's
/// shorthand, which is the verdict for `ok` and the errno name otherwise.
pub fn banner_cell(f: &Findings, name: &str) -> String {
    let short = name
        .strip_prefix("clone(CLONE_")
        .and_then(|r| r.strip_suffix(')'))
        .map(|r| format!("clone({r})"))
        .unwrap_or_else(|| name.split(['(', ' ']).next().unwrap_or(name).to_string());
    match f.get(name) {
        None => format!("{short}=(not probed)"),
        Some(o) => match (o.verdict, o.errno) {
            (Verdict::Ok, _) => format!("{short}=ok"),
            (Verdict::Denied, Some(e)) => format!("{short}={}", e.name()),
            (Verdict::Denied, None) => format!("{short}=denied"),
            (Verdict::Skip, Some(e)) => format!("{short}=skip({})", e.name()),
            (Verdict::Skip, None) => format!("{short}=skip"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys;
    use crate::verdict::Outcome;

    #[test]
    fn the_banner_cell_names_the_errno_rather_than_the_word_denied() {
        let f = Findings {
            rows: vec![
                ("clone(CLONE_NEWNS)", Outcome::ok()),
                ("ptrace(PTRACE_TRACEME)", Outcome::denied(sys::EPERM)),
            ],
            ..Findings::empty()
        };
        assert_eq!(banner_cell(&f, "clone(CLONE_NEWNS)"), "clone(NEWNS)=ok");
        assert_eq!(banner_cell(&f, "ptrace(PTRACE_TRACEME)"), "ptrace=EPERM");
    }
}
