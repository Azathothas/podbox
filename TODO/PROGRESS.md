# Progress

This page gives the state of the record and the only work order
(`AGENTS.md`, section 11). [INDEX](INDEX.md) lists the entries.
[RULES](RULES.md) gives the procedure. [RESUME](RESUME.md) gives the cold
start.

## State

351 entries: 0 open, 5 partial, 4 blocked, 342 done.

The state on 2026-11-12 (the end of a session; `docs/HANDOFF.md` has the
detail):

* The newest release is `v0.2.0-beta.6` (2026-11-12, release run
  43241608767 on `860d6294`): the aarch64 interposer (T-1721), the runsc
  tier (T-1729), the 9p volumes and `-t` of the machine tier (T-1723),
  the incus verbs (T-1722, partial), the relay over `pod-relay` (T-1678).
  `release.yml` needs a successful gate run on its commit; dispatch
  `nightly.yml` with `trial=true` on `main` first.
* podvm takes the crates of PodUtils/podcrates at `v0.32.0` (`032e86f`,
  with the trial of `pod-relay`). A change of moved code goes to podcrates
  as a COORD issue.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`;
  `kvm.yml` (T-1682) gives a hosted runner with it, and `arm.yml` a
  hosted arm64 runner (T-1721).
* PodUtils/podrunner has a lead session; it waits to pin beta.6 for
  PodUtils/podrunner#52 (COORD-065, issue 286).

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
   [T-1662](net.md) (blocked on `pod-netlab` of podcrates,
   COORD-041, issue 260), [T-1663](net.md) and [T-1664](net.md) (they
   wait with T-1662).
3. The record and the tree: no entry is open.
4. [T-1406](podssh.md), partial: live-relay pairing over `pod-relay`.
   Blocked on podcrates: [T-1714](probe.md) waits for
   the schema of podcrates@T-107.
5. After podvm's own order (the owner's ruling "Priority",
   2026-11-09): the podrunner part (issues 229 to 240) and the clauses
   of PodUtils/podcrates#39.

The entries done since the resume of 2026-11-11 are in the record; the
handoff (`docs/HANDOFF.md`) names them.

## History

The layers of this page before 2026-11-10 (the CI remainder of
2026-10-01, the host failure of 2026-09-30, the publication of beta.10 to
beta.12, the reconciliation, the standing decisions and the session tables
of 2026-10-02) are in
[progress-before-2026-11-10.txt](../docs/history/progress-before-2026-11-10.txt).
