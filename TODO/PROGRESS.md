# Progress

This page gives the state of the record and the only work order
(`AGENTS.md`, section 11). [INDEX](INDEX.md) lists the entries.
[RULES](RULES.md) gives the procedure. [RESUME](RESUME.md) gives the cold
start.

## State

388 entries: 2 open, 5 partial, 3 blocked, 378 done.

The state on 2026-11-13 (`docs/HANDOFF.md` has the detail):

* The newest release is `v0.2.0-beta.12` (2026-11-13, release run
  43474394015 on `c1a8d3e3`; the gate 43468572134, `arm.yml` 43471435973
  and the nightly trial 43472240628 passed first, through issue 315): the
  sealed run of the machine tier (T-1751, ask V-7 of podswarm) and no NIC
  with a host channel on the incus tier (T-1779, PodUtils/podrunner#88).
  `v0.2.0-beta.8` (release run 43399714037, issue 302) is the complete
  beta of the QA pass. `release.yml` needs a successful gate run on its
  commit; dispatch `nightly.yml` with `trial=true` on `main` first. A
  request asks for one dispatch of the gate: two runs of it cancel each
  other (issue 301).
* GitHub Actions is disabled for the account that a cloud session pushes
  and dispatches with (2026-11-13): a push starts no run, and
  `actions_run_trigger` answers "Actions has been disabled for this user".
  Each run of a cloud session is an issue `[gh action] requested`
  (`docs/decisions.md`, 2026-11-13, "Workflow fallback").
* podvm takes the crates of PodUtils/podcrates at `v0.40.0` (`1335e9a`).
  A change of moved code goes to podcrates as a COORD issue.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`;
  `kvm.yml` (T-1682) gives a hosted runner with it, and `arm.yml` a
  hosted arm64 runner (T-1721).
* PodUtils/podrunner takes the betas for its kinds. COORD-079 (issue 307,
  T-1777), COORD-081 (issue 311, T-1778) and COORD-082 (issue 316,
  T-1779) wait for its take of `0.2.0-beta.12`; COORD-077 (issue 303,
  T-1776) is closed.

Each partial entry names its remaining clauses. Each blocked entry names
its blocker and the condition that clears it.

## Work order

The goal is a complete release for the beta testers (the owner's ruling
"Complete release first", 2026-11-13). A tool that the cloud host lacks
is installed, not a blocker ("Missing tools", the same day). Take the
first entry that is not done. Read its Prove before you start. Check
each claim that an entry waits against the code and the crates. Set its
status with `podvm-count` in the same commit as the work.

1. podrunner takes `0.2.0-beta.12` (COORD-079, issue 307; COORD-081,
   issue 311; COORD-082, issue 316). The QA pass of
   experiment 443 is worked, and each of its entries is done. The copy
   of the rootfs on each run (11 s for golang) is T-1749.
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
   [T-1749](enter.md) (partial; COORD-071: the overlay root is
   podcrates@T-486, open), [T-1747](cli.md) and [T-1750](machine.md).
   V-1, V-2 (the probe cache of `run`), V-4, V-7 (`--podvm-sealed`) and
   V-8 are done.

The entries done since the resume of 2026-11-11 are in the record; the
handoff (`docs/HANDOFF.md`) names them.

## History

The layers of this page before 2026-11-10 (the CI remainder of
2026-10-01, the host failure of 2026-09-30, the publication of beta.10 to
beta.12, the reconciliation, the standing decisions and the session tables
of 2026-10-02) are in
[progress-before-2026-11-10.txt](../docs/history/progress-before-2026-11-10.txt).
