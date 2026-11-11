# Progress

This page gives the state of the record and the only work order
(`AGENTS.md`, section 11). [INDEX](INDEX.md) lists the entries.
[RULES](RULES.md) gives the procedure. [RESUME](RESUME.md) gives the cold
start.

## State

328 entries: 33 open, 9 partial, 4 blocked, 282 done.

The state on 2026-11-11:

* The newest release is `v0.2.0-beta.2` (2026-11-10), the first release
  with the name podvm (T-1671). A release starts from a dispatched
  `release.yml`. Dispatch `nightly.yml` with `trial=true` on `main` first.
* podvm takes 11 crates of PodUtils/podcrates (`pod-core`, `pod-tls`,
  `pod-proxy`, `pod-term`, `pod-hostprobe`, `pod-supervise`, `pod-guest`,
  `pod-enter`, `pod-oci`, `pod-extract`, `pod-rootfs`) at the tag
  `v0.17.0` (T-1675 to T-1677, T-1681, T-1716, T-1708, T-1655). A change of moved code
  goes to podcrates as a COORD issue.
* The errand-host entries T-1643 to T-1650 are done. Their issues are
  closed. Issue 114 is the dossier behind the open rungs T-1651 to T-1653.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`.
  The KVM legs of T-1112, T-1350 and T-1641 need a host with `/dev/kvm`.
  T-1682 gives them a hosted runner.
* podcrates `v0.17.0` (2026-11-11) is the last tag, and podvm takes it
  (the child limits of T-1655, PodUtils/podcrates#58).
* PodUtils/podrunner has a lead session since 2026-11-11 (the ruling
  "podrunner online"): DR-1 and DR-2 are yes. Its issues are under
  PodUtils/podrunner#1; podvm's part is T-1704 to T-1715 (issues 229 to
  240); the crate asks are podcrates T-160 to T-165 and podnet T-229 to
  T-232.

Each partial entry names its remaining clauses. Each blocked entry names
its blocker and the condition that clears it.

## Work order

The owner resumed podvm's own work on 2026-11-11
([decisions](../docs/decisions.md), "podvm resumes"), and put the needs of
podrunner first ("podrunner first", the same day). Take the first entry
that is not done. Read its Prove before you start. Set its status with
`podvm-count` in the same commit as the work.

1. [T-1655](cli.md): each flag of the hardening line works on the tier or
   refuses by name. Its read-only layer closes the last clause of
   [T-1653](enter.md), `-v HOST:GUEST:ro` with `EROFS` (issue 130).
2. [T-1703](interpose.md): `fexecve`, `execveat` with flags, `system()`
   and `popen()` enter through the image's loader. Then
   [T-1710](interpose.md), [T-1707](enter.md) and [T-1719](enter.md)
   (COORD issues for `pod-enter`), and [T-1706](cli.md) (the official
   runner on the userland rung, after T-1707; T-1704 and T-1705 are done).
3. [T-1682](machine.md): `kvm.yml`, the Linux guest of the machine tier
   on a hosted runner with `/dev/kvm`. Then [T-1656](machine.md)
   (microVMs from `run`; podrunner's `microvm` unit waits for it, DR-5).
   Then the KVM legs on a host with `/dev/kvm`: [T-1641](gate.md),
   [T-1350](gate.md) and [T-1112](milestones.md), in one bounded run.
   Then [T-1717](machine.md) (the incus study) and [T-1718](machine.md)
   (the incus back end).
4. [T-1652](enter.md): a native-static rung with Syscall User Dispatch
   and `openat2(RESOLVE_IN_ROOT)` (issue 109).
5. [T-1651](enter.md): an `emulate` tier on a patched, fail-closed
   qemu-user (issue 94). It comes after T-1652, which is the first choice
   for a native static payload.
6. [T-1604](enter.md), partial: the plant, a second lane run and the
   saved lane log of `detached_stdio`. No guest is necessary.
7. [T-0211](image.md), partial: retarget the mutations of experiment 157
   so that each one turns one test red.
8. [T-0204](image.md), partial: a Prove that reads the lock skip
   (`is in use by a running container`) with podvm as the holder. Then
   [T-1709](image.md) (a rootfs for each container), on the same code.
9. The scope entries (issues 166 to 189, index issue 193), in this order:
   [T-1654](cli.md) (profiles over the flags of T-1655),
   [T-1658](enter.md) and [T-1657](enter.md) (on T-1652),
   [T-1666](probe.md) and [T-1667](cli.md) (the probe and the doctor),
   the network entries [T-1665](net.md), [T-1662](net.md),
   [T-1663](net.md) and [T-1664](net.md) (on T-1656), the profiles
   [T-1659](cli.md), [T-1661](cli.md) and [T-1660](cli.md), and
   [T-1668](gate.md) beside them.
10. The record and the tree: [T-1679](deps.md) (`pod-services`),
    [T-1672](gate.md) (the record in the format of `pod-todo`), then
    [T-1680](deps.md) (blocked on T-1672), [T-1673](gate.md) (the 29
    plan rows), [T-1683](gate.md) (retire the PowerShell twins),
    [T-1684](gate.md) (fuzzing and the vendor watch) and
    [T-1685](gate.md) (the two pages in the sibling form).
11. Blocked on podcrates: [T-1678](deps.md) waits for a `pod-relay`
    release. [T-1406](podssh.md), partial, proves live-relay pairing
    after it. [T-1714](probe.md) waits for the schema of podcrates T-107.
12. After podvm's own order (the owner's ruling "Priority",
    2026-11-09): the asks of cfb, [T-1694](cli.md); the podrunner part
    (issues 229 to 240), [T-1695](cli.md);
    then [T-1711](cli.md) (the container-hook calls, after T-1655,
    T-1709 and T-1710); and the clauses of PodUtils/podcrates#39.

## History

The layers of this page before 2026-11-10 (the CI remainder of
2026-10-01, the host failure of 2026-09-30, the publication of beta.10 to
beta.12, the reconciliation, the standing decisions and the session tables
of 2026-10-02) are in
[progress-before-2026-11-10.txt](../docs/history/progress-before-2026-11-10.txt).
