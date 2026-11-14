# Progress

This page gives the state of the record and the only work order
(`AGENTS.md`, section 11). [INDEX](INDEX.md) lists the entries.
[RULES](RULES.md) gives the procedure. [RESUME](RESUME.md) gives the cold
start.

## State

480 entries: 67 open, 6 partial, 6 blocked, 401 done.

The state on 2026-11-14 (`docs/HANDOFF.md` has the detail):

* The newest release is `v0.2.0-beta.17` (2026-11-14, release run
  43580765158 on `0f13267c`; the gate 43576720202, `arm.yml` 43577573447
  and the nightly trial 43578086980 passed first, through issue 355): a
  second glibc object for glibc 2.34 images with no `libdl.so.2`
  (T-1880, PodUtils/podrunner#100), a full stdout exits 125 (T-1816),
  and a missing container name exits 1 (T-1805). beta.16 (issue 349)
  gave the eight P0 findings of the external QA report. `release.yml`
  needs a successful gate run on its commit; dispatch `nightly.yml` with
  `trial=true` on `main` first. A request asks for one dispatch of the
  gate: two runs of it cancel each other (issue 301).
* GitHub Actions is disabled for the account that a cloud session pushes
  and dispatches with (2026-11-13): a push starts no run, and
  `actions_run_trigger` answers "Actions has been disabled for this user".
  Each run of a cloud session is an issue `[gh action] requested`
  (`docs/decisions.md`, 2026-11-13, "Workflow fallback").
* podvm takes the crates of PodUtils/podcrates at `v0.43.0` (`ccbea49`,
  PodUtils/podcrates#149): it carries podcrates@T-567 (the completion of
  two layers) and podcrates@T-568 to podcrates@T-570.
  A change of moved code goes to podcrates as a COORD issue.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`;
  `kvm.yml` (T-1682) gives a hosted runner with it, and `arm.yml` a
  hosted arm64 runner (T-1721).
* PodUtils/podrunner takes the betas for its kinds. It closed each of its
  asks to podvm (PodUtils/podrunner#85, #87, #88, #90); podvm closed its
  answers (issues 303, 307, 311, 316, 319) on 2026-11-13.

Each partial entry names its remaining clauses. Each blocked entry names
its blocker and the condition that clears it.

## Work order

The goal is a complete release for the beta testers (the owner's ruling
"Complete release first", 2026-11-13). A tool that the cloud host lacks
is installed, not a blocker ("Missing tools", the same day). Take the
first entry that is not done. Read its Prove before you start. Check
each claim that an entry waits against the code and the crates. Set its
status with `podvm-count` in the same commit as the work.

1. A new ask of a peer session comes first (the watcher of `AGENTS.md`,
   section 9). The QA pass of
   experiment 443 is worked, and each of its entries is done. The copy
   of the rootfs on each run is T-1749: since 2026-11-14 a spare tree
   takes it off each run after the first, and podcrates `v0.42.0` carries the
   overlay root of the namespace rung (podcrates@T-486), which podvm takes
   next.
2. The external QA report of `v0.2.0-beta.14` (2026-11-14): the open
   entries from T-1803 to T-1860. Each P0 entry is done. Take the P1
   entries, each with the guard of its class ([T-1852](gate.md) to
   [T-1857](gate.md)), then P2 and P3.
2a. The Windows guest plan of the review of 2026-11-14: T-1861 to T-1879
   (`podvm run windows:NAME`). The owner accepts the vendor terms
   (`docs/decisions.md`, "Windows media"). Take the P1 entries first:
   [T-1861](machine.md), [T-1862](machine.md), [T-1863](machine.md),
   [T-1864](machine.md), [T-1865](machine.md), [T-1866](machine.md),
   [T-1867](machine.md) (the first KVM proof, on `kvm.yml`) and
   [T-1871](machine.md); then the P2 entries and T-1878 (P3). T-1862 to
   T-1865 wait for podcrates tags; T-1879 is a text fix that needs no tag.
3. The scope entries that wait for a peer: [T-1662](net.md) (partial:
   `podvm lab scenarios` and `plan` run; the lab guest, its probes and the
   judge are podcrates@T-345 to podcrates@T-347), [T-1667](cli.md)
   (partial: its check of labs comes with T-1662), [T-1663](net.md) and
   [T-1664](net.md) (blocked: they run in the lab guest of T-1662).
   podcrates names them after podcrates@T-486 and @T-107, which `v0.42.0`
   carries (PodUtils/podcrates#142).
4. [T-1740](enter.md) (partial: a cheaper syscall tier) and
   [T-1350](gate.md) (partial: the wrapper of the parked Windows lane).
5. Low priority, the asks of podswarm (PodUtils/podcrates#124):
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
