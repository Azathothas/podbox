# Progress

This page gives the state of the record and the only work order
(`AGENTS.md`, section 11). [INDEX](INDEX.md) lists the entries.
[RULES](RULES.md) gives the procedure. [RESUME](RESUME.md) gives the cold
start.

## State

383 entries: 7 open, 6 partial, 5 blocked, 365 done.

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

The goal is a complete release for the beta testers (the owner's ruling
"Complete release first", 2026-11-13). A tool that the cloud host lacks
is installed, not a blocker ("Missing tools", the same day). Take the
first entry that is not done. Read its Prove before you start. Check
each claim that an entry waits against the code and the crates. Set its
status with `podvm-count` in the same commit as the work.

1. The defects of the QA pass (experiment 443, 2026-11-13: 518 rows pass,
   64 fail; the rerun: 566 pass, 17 fail), then the release
   `0.2.0-beta.8` for the beta testers (`docs/HANDOFF.md`). Open:
   [T-1757](cli.md), [T-1754](interpose.md) and [T-1755](interpose.md)
   (fixed; their rows wait for the next rerun), [T-1760](cli.md)
   (partial: `tty` on the namespace rung waits for the /dev/pts half of
   COORD-073), [T-1770](net.md) (blocked on COORD-075) and
   [T-1774](image.md) (P3). The copy of the rootfs on each run (11 s for
   golang) is T-1749.
2. The scope entries that wait for a peer: [T-1662](net.md) (partial:
   `podvm lab scenarios` and `plan` run; the lab guest, its probes and the
   judge are podcrates@T-345 to podcrates@T-347), [T-1667](cli.md)
   (partial: its check of labs comes with T-1662), [T-1663](net.md) and
   [T-1664](net.md) (blocked: they run in the lab guest of T-1662).
   [T-1714](probe.md) waits for the published schema of podcrates@T-107;
   podcrates `v0.36.0` holds its first part, the report with its reader.
3. [T-1740](enter.md) (partial: a cheaper syscall tier) and
   [T-1350](gate.md) (partial: the wrapper of the parked Windows lane).
4. Low priority, the asks of podswarm (PodUtils/podcrates#124):
   [T-1746](probe.md) (`pod-hostprobe`'s reader is in `v0.36.0`; the
   probe costs about 50 ms on the cloud host), [T-1749](enter.md)
   (partial; COORD-071), [T-1747](cli.md), [T-1750](machine.md) and
   [T-1751](machine.md). V-1, V-4 and V-8 are done.

The entries done since the resume of 2026-11-11 are in the record; the
handoff (`docs/HANDOFF.md`) names them.

## History

The layers of this page before 2026-11-10 (the CI remainder of
2026-10-01, the host failure of 2026-09-30, the publication of beta.10 to
beta.12, the reconciliation, the standing decisions and the session tables
of 2026-10-02) are in
[progress-before-2026-11-10.txt](../docs/history/progress-before-2026-11-10.txt).
