# Handoff

The state on 2026-11-11, in the session that resumed podvm. A fresh
session with no memory reads this file first, then `AGENTS.md`,
`TODO/PROGRESS.md`, `TODO/RESUME.md` and `docs/decisions.md`. Remove this
file when each step below is done.

## Where things are

* The repository is PodUtils/podvm on `main`. No work lives in a local
  worktree or a branch. The branch `archive/podbox-worktree` is an
  archive; never merge or delete it.
* The owner resumed podvm on 2026-11-11 and answered Q1 to Q7
  (`docs/decisions.md`, rows of 2026-11-11). No question is open.
* The newest release is `v0.2.0-beta.3` (2026-11-11, release run
  42980701550 on c62643d). A release starts from `release.yml`; dispatch
  `nightly.yml` with `trial=true` on `main` first.
* podvm takes the crates of PodUtils/podcrates at `v0.22.0` (`93de04a`,
  T-1732): `die_with_parent` in the launcher of `run -d`, and
  `pod-services` (T-1679).
* **The record is in the format of `pod-todo` since 2026-11-11** (T-1672,
  call C17). An entry is `# T-NNNN: Title`, the header fields
  `**Name:** value` (with `**Milestone:** none`), and the sections
  `## Problem` to `## Prove`, then `## Blocker`, `## Correction` and
  `## Done`. A row of `TODO/INDEX.md` has seven cells. `podvm-count --set
  T-NNNN STATUS` writes the `**Status:**` line and the row; a done entry
  needs `## Done` with a date. `podvm-gate` checks the record through a
  view of the old layout (`crates/podvm-gate/src/gate/legacy.rs`), and
  `podvm-todo check` (pod-todo's reader) reports the problems that are not
  of form: those are T-1680.
* PodUtils/podrunner has a lead session since 2026-11-11. It writes to
  podvm by a COORD issue titled `(to podvm)`; the watcher reads it.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`;
  the machine tier runs TCG there.

## Open COORD issues

* COORD-001 (#194, to podcrates): the takes of `pod-relay` (T-1678,
  blocked: no release), `pod-services` (T-1679, partial) and `pod-todo`
  (T-1680, partial).
* COORD-040 (#259): `pod-confine` for T-1658, in podcrates `v0.23.0`.
* COORD-041 (#260): `pod-netlab` and the reader of the guest's modloop
  for T-1662 (T-1663 and T-1664 wait with it).
* COORD-042 (#261): pod-extract and the root entry with an empty name
  (the lambda-al2023 clause of T-1659); the fix is in `v0.23.0`.
* COORD-043 (#262, to all): podvm took `v0.22.0`.
* COORD-044 (#263, to podcrates): `gcr.io`, `mirror.gcr.io` and the NIC
  module sums of the guest for `pod-services` (T-1679).
* COORD-038, COORD-039 and the takes before them stay open until their
  peer answers.

## Next steps, in order

1. At podcrates `v0.23.0`: take it, then finish T-1658 (`pod-confine`),
   the lambda clause of T-1659, `workerd-test` of T-1661, and T-1679
   (empty the `WAITING` list of `crates/podvm/src/hosts_tests.rs`).
2. The work order of `TODO/PROGRESS.md`, step 9: T-1680 (partial: the remaining
   problems of `podvm-todo check`, then the record checks move to
   pod-todo), T-1673, T-1683, T-1684, T-1685.
3. Done on 2026-11-11 after the first handoff: T-1659 (partial),
   T-1661 (partial), T-1660, T-1668, T-1672, T-1732, T-1679 (partial).
   Defects fixed on the way: the hard links of a container tree and of the
   guest's base archive, the removal of a read-only tree by a user that is
   not root, a partial tree of a killed run, `--argv0` on the first exec
   of the loader family, `lstat` of a volume's mount point.
4. Before an entry is set done, run `cargo test -p podvm`: the test
   `each_listed_gap_names_an_entry_that_is_not_done` reads the record.
5. CI: each push cancels the run before it. Let one run finish between
   pushes where it can.
