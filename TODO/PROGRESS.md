# Progress

This page gives the state of the record and the only work order
(`AGENTS.md`, section 11). [INDEX](INDEX.md) lists the entries.
[RULES](RULES.md) gives the procedure. [RESUME](RESUME.md) gives the cold
start.

## State

361 entries: 4 open, 8 partial, 4 blocked, 345 done.

The state on 2026-11-13 (`docs/HANDOFF.md` has the detail):

* The newest release is `v0.2.0-beta.7` (2026-11-13, release run
  43348676353 on `2edace91`; 42 assets; the gate 43345851281, `arm.yml`
  43347185943 and the nightly trial 43347752049 passed first, through
  issue 287): T-1743 for PodUtils/podrunner@T-096, and podcrates
  `v0.34.0`. Experiment 441 exits 0 with its `podvm-x86_64`.
  `release.yml` needs a successful gate run on its commit; dispatch
  `nightly.yml` with `trial=true` on `main` first.
* GitHub Actions is disabled for the account that a cloud session pushes
  and dispatches with (2026-11-13): a push starts no run, and
  `actions_run_trigger` answers "Actions has been disabled for this user".
  Each run of a cloud session is an issue `[gh action] requested`
  (`docs/decisions.md`, 2026-11-13, "Workflow fallback").
* podvm takes the crates of PodUtils/podcrates at `v0.35.0` (`311c6bb`).
  A change of moved code goes to podcrates as a COORD issue.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`;
  `kvm.yml` (T-1682) gives a hosted runner with it, and `arm.yml` a
  hosted arm64 runner (T-1721).
* PodUtils/podrunner pins `v0.2.0-beta.6`; PodUtils/podrunner@T-096 takes
  beta.7 (COORD-067, issue 289).

Each partial entry names its remaining clauses. Each blocked entry names
its blocker and the condition that clears it.

## Work order

The owner resumed podvm's own work on 2026-11-11
([decisions](../docs/decisions.md), "podvm resumes"), and put the needs of
podrunner first ("podrunner first", the same day). Take the first entry
that is not done. Read its Prove before you start. Set its status with
`podvm-count` in the same commit as the work.

1. The KVM legs passed on the hosted runner of `kvm.yml` on 2026-11-12;
   [T-1350](gate.md) keeps only the wrapper of the parked Windows lane.
   [T-1722](machine.md) (partial: the other verbs of incus, the ladder)
   comes after the scope entries of step 2.
2. The scope entries (issues 166 to 189, index issue 193), in this order:
   [T-1667](cli.md) (partial:
   the check of labs in a guest waits for the network entries); the network entries
   [T-1662](net.md) (partial: `podvm lab scenarios` and `plan` run; the
   guest, the probes and the judge wait for podcrates@T-345 to
   podcrates@T-347), [T-1663](net.md) and [T-1664](net.md) (blocked: they
   run in the lab guest of T-1662).
3. The record and the tree: [T-1744](interpose.md) (partial: the open flags of the
   interposer on aarch64; its proof needs a run of `arm.yml`).
4. [T-1406](podssh.md), partial: live-relay pairing over `pod-relay`.
   Blocked on podcrates: [T-1714](probe.md) waits for
   the schema of podcrates@T-107.
5. After podvm's own order (the owner's ruling "Priority",
   2026-11-09): the podrunner part (issues 229 to 240) and the clauses
   of PodUtils/podcrates#39; then the asks of podswarm
   (PodUtils/podcrates#124): [T-1745](cli.md), [T-1747](cli.md) and
   [T-1749](enter.md) to [T-1751](machine.md); V-4 and V-8 are done, and
   [T-1746](probe.md) (V-2) is blocked on a reader
   of the probe report in `pod-hostprobe` (COORD-069, issue 291). Each ask has a
   fallback in podswarm.

The entries done since the resume of 2026-11-11 are in the record; the
handoff (`docs/HANDOFF.md`) names them.

## History

The layers of this page before 2026-11-10 (the CI remainder of
2026-10-01, the host failure of 2026-09-30, the publication of beta.10 to
beta.12, the reconciliation, the standing decisions and the session tables
of 2026-10-02) are in
[progress-before-2026-11-10.txt](../docs/history/progress-before-2026-11-10.txt).
