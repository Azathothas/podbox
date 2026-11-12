# Progress

This page gives the state of the record and the only work order
(`AGENTS.md`, section 11). [INDEX](INDEX.md) lists the entries.
[RULES](RULES.md) gives the procedure. [RESUME](RESUME.md) gives the cold
start.

## State

351 entries: 1 open, 6 partial, 7 blocked, 337 done.

The state on 2026-11-11:

* The newest release is `v0.2.0-beta.3` (2026-11-11), the beta that
  podrunner asked for (PodUtils/podrunner#53). `v0.2.0-beta.2`
  (2026-11-10) is the first release with the name podvm (T-1671). A
  release starts from a dispatched
  `release.yml`. Dispatch `nightly.yml` with `trial=true` on `main` first.
* podvm takes 15 crates of PodUtils/podcrates at the tag `v0.22.0`
  (`93de04a`, T-1732): `pod-core`, `pod-tls`, `pod-proxy`, `pod-term`,
  `pod-hostprobe`, `pod-supervise`, `pod-guest`, `pod-enter`, `pod-oci`,
  `pod-extract`, `pod-rootfs`, `pod-incus`, `pod-broker`, `pod-services`
  and `pod-todo`. A change of moved code goes to podcrates as a COORD
  issue.
* The errand-host entries T-1643 to T-1653 are done. Their issues are
  closed, and issue 114, their dossier, is closed.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`.
  The KVM legs of T-1112, T-1350 and T-1641 need a host with `/dev/kvm`.
  T-1682 gives them a hosted runner.
* podcrates `v0.22.0` (`93de04a`) is the last tag, and podvm takes it
  (T-1732, PodUtils/podcrates#71). podvm waits for `v0.23.0`
  (`pod-confine` and the fix of issue 261).
* PodUtils/podrunner has a lead session since 2026-11-11 (the ruling
  "podrunner online"): DR-1 and DR-2 are yes. Its issues are under
  PodUtils/podrunner#1; podvm's part is T-1704 to T-1715 (issues 229 to
  240); the crate asks are podcrates podcrates@T-160 to podcrates@T-165 and podnet podnet@T-229 to
  podnet@T-232.

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
   [T-1723](machine.md) (the `microvm` machine, a shared `/workspace`)
   and [T-1722](machine.md) (the other verbs of incus) come after the
   scope entries of step 2.
2. The scope entries (issues 166 to 189, index issue 193), in this order:
   [T-1667](cli.md) (partial:
   the check of labs in a guest waits for the network entries); the network entries
   [T-1662](net.md) (blocked on `pod-netlab` of podcrates,
   COORD-041, issue 260), [T-1663](net.md) and [T-1664](net.md) (they
   wait with T-1662).
3. The record and the tree: no entry is open.
4. Blocked on podcrates: [T-1678](deps.md) waits for a `pod-relay`
   release. [T-1406](podssh.md), partial, proves live-relay pairing
   after it. [T-1714](probe.md) waits for the schema of podcrates@T-107.
   [T-1741](image.md) and [T-1742](enter.md) wait for the fixes of
   pod-extract and pod-enter (issues 281 and 282); then
   [T-1721](interpose.md), partial, runs `arm.yml` again.
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
