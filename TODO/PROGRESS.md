# Progress

This page gives the state of the record and the only work order
(`AGENTS.md`, section 11). [INDEX](INDEX.md) lists the entries.
[RULES](RULES.md) gives the procedure. [RESUME](RESUME.md) gives the cold
start.

## State

346 entries: 10 open, 9 partial, 6 blocked, 321 done.

The state on 2026-11-11:

* The newest release is `v0.2.0-beta.3` (2026-11-11), the beta that
  podrunner asked for (PodUtils/podrunner#53). `v0.2.0-beta.2`
  (2026-11-10) is the first release with the name podvm (T-1671). A
  release starts from a dispatched
  `release.yml`. Dispatch `nightly.yml` with `trial=true` on `main` first.
* podvm takes 12 crates of PodUtils/podcrates (`pod-core`, `pod-tls`,
  `pod-proxy`, `pod-term`, `pod-hostprobe`, `pod-supervise`, `pod-guest`,
  `pod-enter`, `pod-oci`, `pod-extract`, `pod-rootfs`, `pod-incus`) at the
  tag `v0.21.0` (T-1675 to T-1677, T-1681, T-1716, T-1708, T-1655, T-1719,
  T-1727). A change of moved code
  goes to podcrates as a COORD issue.
* The errand-host entries T-1643 to T-1650 are done. Their issues are
  closed. Issue 114 is the dossier behind the open rungs T-1651 to T-1653.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`.
  The KVM legs of T-1112, T-1350 and T-1641 need a host with `/dev/kvm`.
  T-1682 gives them a hosted runner.
* podcrates `v0.21.0` (2026-11-11, `ada3699`) is the last tag, and podvm
  takes it (`pod-incus` of T-1727, PodUtils/podcrates#65).
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

1. The KVM legs on a host with `/dev/kvm`: [T-1641](gate.md),
   [T-1350](gate.md) and [T-1112](milestones.md), in one bounded run.
   [T-1723](machine.md) (the `microvm` machine, a shared `/workspace`)
   and [T-1722](machine.md) (the other verbs of incus) come after the
   scope entries of step 2.
2. The scope entries (issues 166 to 189, index issue 193), in this order:
   [T-1658](enter.md) (blocked on `pod-confine` of podcrates, COORD-040,
   issue 259; it is in podcrates `v0.23.0`); [T-1667](cli.md) (partial:
   the `gh` fallback waits for an installer); the network entries
   [T-1665](net.md) (partial: the fake address of a named upstream
   remains), [T-1662](net.md) (blocked on `pod-netlab` of podcrates,
   COORD-041, issue 260), [T-1663](net.md) and [T-1664](net.md) (they
   wait with T-1662); the profiles [T-1659](cli.md) (partial: the
   lambda-al2023 clause waits for COORD-042, issue 261) and
   [T-1661](cli.md) (partial: wasmtime-wasi waits for a first-use asset
   record, workerd-test for T-1658).
3. The record and the tree: [T-1679](deps.md) (`pod-services`; partial:
   COORD-044, issue 263), the plan row [T-1737](packaging.md),
   [T-1684](gate.md) (fuzzing and the
   vendor watch).
4. Blocked on podcrates: [T-1678](deps.md) waits for a `pod-relay`
   release. [T-1406](podssh.md), partial, proves live-relay pairing
   after it. [T-1714](probe.md) waits for the schema of podcrates@T-107.
5. After podvm's own order (the owner's ruling "Priority",
   2026-11-09): the asks of cfb, [T-1694](cli.md); the podrunner part
   (issues 229 to 240), [T-1695](cli.md); then [T-1711](cli.md) (the
   container-hook calls); and the clauses of PodUtils/podcrates#39.

The entries done since the resume of 2026-11-11 are in the record; the
handoff (`docs/HANDOFF.md`) names them.

## History

The layers of this page before 2026-11-10 (the CI remainder of
2026-10-01, the host failure of 2026-09-30, the publication of beta.10 to
beta.12, the reconciliation, the standing decisions and the session tables
of 2026-10-02) are in
[progress-before-2026-11-10.txt](../docs/history/progress-before-2026-11-10.txt).
