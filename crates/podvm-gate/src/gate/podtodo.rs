//! Check 35: pod-todo's reader agrees with the record (TODO/deps.md
//! T-1680).
//!
//! The record is in the format of podcrates' `pod-todo` (T-1672). Its reader
//! checks the form of each entry and row, the counts, the citations and
//! their lines, the ids, the work order and the words, with the facts of
//! `TODO/todo.toml`. Each problem that it finds is a problem of the gate.
//! The record checks of this gate that pod-todo holds go next (T-1680).

use super::*;

pub(super) fn check_pod_todo(gate: &mut Gate, root: &Path) {
    // No base: the check of citations that an edit left behind needs git and
    // a change; `podvm-todo check` in a session runs it.
    let report = pod_todo::check::check_with(root, Some(Vec::new()), &|_: &str| Ok(None), "HEAD");
    for _ in 0..report.counts.total() {
        gate.bump("pod_todo");
    }
    for p in &report.problems {
        gate.err(&p.at, format!("{} (pod-todo, TODO/deps.md T-1680)", p.what));
    }
}
