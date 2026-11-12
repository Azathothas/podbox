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
* The newest release is `v0.2.0-beta.5` (2026-11-12, release run
  43140490892 on b41e0175; 42 assets with sums and sigstore bundles). A
  release starts from `release.yml`; dispatch `nightly.yml` with
  `trial=true` on `main` first.
* Commit signing is off for this repository (`.claude/session-start.sh`,
  `docs/decisions.md`, "Commit signing"), so the stop hook of a cloud
  session asks for no rewrite of the author.
* podvm takes the crates of PodUtils/podcrates at `v0.27.0`:
  `pod-confine` and `Plan.confine` (T-1658), and the hosts and module sums
  of `pod-services` (T-1679, at `v0.24.0`).
* **The record is in the format of `pod-todo` since 2026-11-11** (T-1672,
  call C17). An entry is `# T-NNNN: Title`, the header fields
  `**Name:** value` (with `**Milestone:** none`), and the sections
  `## Problem` to `## Prove`, then `## Blocker`, `## Correction` and
  `## Done`. A row of `TODO/INDEX.md` has seven cells. `podvm-count --set
  T-NNNN STATUS` writes the `**Status:**` line and the row; a done entry
  needs `## Done` with a date. `podvm-gate` checks the record through a
  view of the old layout (`crates/podvm-gate/src/gate/legacy.rs`), and
  check 35 runs `podvm-todo check` (pod-todo's reader), which holds the
  record checks of form since T-1680.
* PodUtils/podrunner has a lead session since 2026-11-11. It writes to
  podvm by a COORD issue titled `(to podvm)`; the watcher reads it.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`;
  the machine tier runs TCG there.

## Open COORD issues

* COORD-001 (#194, to podcrates): the takes of `pod-relay` (T-1678,
  blocked: no release), `pod-services` (T-1679, partial) and `pod-todo`
  (T-1680, done 2026-11-11).
* COORD-040 (#259): `pod-confine` for T-1658, in podcrates `v0.23.0`.
* COORD-041 (#260): `pod-netlab` and the reader of the guest's modloop
  for T-1662 (T-1663 and T-1664 wait with it).
* COORD-042 (#261): pod-extract and the root entry with an empty name
  (the lambda-al2023 clause of T-1659); the fix is in `v0.23.0`.
* COORD-044 (#263, to podcrates): `gcr.io`, `mirror.gcr.io` and the NIC
  module sums of the guest for `pod-services` (T-1679).
* COORD-033 (#252, to podrunner): the aarch64 interposer, T-1721 (open).
* COORD-039 (#258, to podrunner): T-1709 is on `main`, not in
  `v0.2.0-beta.3`; it stays open until podrunner reads it.
* COORD-024, COORD-025, COORD-028, COORD-031, COORD-032, COORD-034,
  COORD-035, COORD-038 and COORD-043 are closed on 2026-11-11: each peer
  closed the issue that they answer, and each mapped entry is done.

## Next steps, in order

1. podvm takes podcrates `v0.24.0` (T-1679, T-1659 done; T-1658 partial).
   Next: the rest of T-1658 (the rungs with no chroot, `run -d`, `exec`),
   then `workerd-test` of T-1661, which needs the layer.
2. podrunner and cfb read COORD-052 (beta.5); #271 tracks cfb's asks.
3. Done on 2026-11-11 after the first handoff: T-1660, T-1668, T-1672,
   T-1659, T-1665, T-1673, T-1679, T-1680, T-1683, T-1684, T-1685, T-1735, T-1737, T-1732, T-1733, T-1734, T-1736; partial:
   T-1659, T-1661, T-1679. Defects fixed on the way: the hard links of a
   container tree and of the guest's base archive, the removal of a
   read-only tree by a user that is not root, a partial tree of a killed
   run, `--argv0` on the first exec of the loader family, `lstat` of a
   volume's mount point.
4. Before an entry is set done, run `cargo test -p podvm`: the test
   `each_listed_gap_names_an_entry_that_is_not_done` reads the record.
   pod-todo's reader reads the files on disk: an untracked directory can
   hide a citation that a clone of CI refuses (CI run 400).
5. CI: each push cancels the run before it. Let one run finish between
   pushes where it can.
