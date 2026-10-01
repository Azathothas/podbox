# Progress

## State

214 entries: 0 open, 3 partial, 0 blocked, 211 done.

Batch 3 (T-1559 through T-1569, the three tool crates) is landed
2026-10-01: `podbox-buildstate` (buildstate, release-licenses),
`podbox-release` (verify, size, prove-t0211, document-state,
release-notes, reconcile, publish), and `podbox-podvm` (podvm,
podvm-workload). Each retired script stays as an exec shim; the
nightly publish job runs the couriered musl-static `release-notes`
binary. Three lane proofs drove green with retired-vs-binary
agreement
([buildstate](../experiments/results/buildstate-crate-proof.txt),
[release](../experiments/results/release-crate-proof.txt),
[podvm](../experiments/results/podvm-crate-proof.txt)), each with its
plant transcript. T-1563 closes with the port faithful and T-0211
still partial on the live lock; T-1568 and T-1569 close with the
chroot intermittency and the guest gap recorded and owned by T-1302
and T-1308. The tasks no batch names are deliberately skipped; see
`refactor/DEFERRALS.md` section 4.

The 2026-09-30 repository audit is complete in source and records.
T-1348, T-1349, and T-1351 are complete. T-1350 lacks a successful current
KVM guest run. T-1405 is complete: beta.10 publishes all seven helper
archives with digests and signatures.
T-1003 and T-1401 are reopened from omitted acceptance clauses.
T-1112 remains partial on current KVM failures and ReactOS.
T-1406 owns SSH operation deadlines and live reconnect.
Issues 69 through 85 are triaged. T-1407 through T-1421 own the
fifteen findings. The publish order above does not move.

## Host failure and recovery

The audit session staged its work and did not commit. Its third KVM run
left an emulator that SIGKILL did not remove. Later, the Windows host
failed. The operator removed the WSL distributions and stopped the
processes. The recovery session on 2026-09-30 started from `005638d` with
the staged audit. It rebuilt `wsl-toolkit-podbox` with
`wsl-toolkit --instance podbox base ensure --probe` and ran no guest.

The recovery found a source defect. `podbox_windows::run` piped the
emulator's streams and did not read them. The fix and its mutation proof
are in T-1112 and [experiment 401](../experiments/results/emulator-streams.txt).
The KVM driver now needs `--accept-host-risk` and an operator who is present.
See T-1350 and [Limits](../docs/limits.md).

The rebuilt base has no cgroup delegation. The engine accepts memory and
CPU limits and does not enforce them.

## Reconciliation

Main contains the SSH transport, relay, interactive session, and remote and
machine command dispatch. Pull requests 66 and 67 are closed.
All eight commits on `publish/20260926T052210Z` are on main by patch
identity or by an equivalent commit. The one context difference is task
counts. [The comparison](../experiments/results/publish-branch-comparison.txt)
records the finding. Delete the branch after verified main publication.

The retained io12 loader is not linked. The native userland loader is in
podbox-enter. The passwd shim is retained source without a compiled build
path. See [Source state](../docs/runtime-state.md) and [Limits](../docs/limits.md).

The read-only [pg-toolkit report](../docs/history/references/pg-toolkit-2026-09-30.md)
records selected source, read depth, licence, and transfer decisions.
T-1349 implemented its input and output byte method independently.
No runtime code from that repository is linked.

## Verification

- [Host strict gate](../experiments/results/host-strict.txt): 11 of 11 checks passed, no skip.
- [Recovery Linux check](../experiments/results/recovery-linux.txt): 14 of 14 steps in the rebuilt base.
- [Audit Linux check](../experiments/results/repo-audit-linux.txt) with the plant baseline.
- [Emulator streams](../experiments/results/emulator-streams.txt): test and mutation.
- [Fault plants](../experiments/results/repo-audit-plants.txt): 44 plants caught, four clean controls quiet.
- [Default wrapper](../experiments/results/repo-audit-default.txt) and [exported bytes](../experiments/results/exported-build.txt).
- [Freshness fixture](../experiments/results/build-freshness.txt) and [gate diagnostics](../experiments/results/gate-diagnostics.txt).

The KVM repetitions of 2026-09-30 failed at command shutdown and setup.
[The first](../experiments/results/kvm-guest-2026-09-30-first.txt),
[the second](../experiments/results/kvm-guest-2026-09-30-second.txt), and
[the third](../experiments/results/kvm-guest-2026-09-30-third.txt) are saved.
The [2026-09-29 success](../experiments/results/kvm-guest-2026-09-29.txt)
keeps its historical scope. It does not establish current reliability.

## Work order

1. T-1405 is done: beta.10 publishes all helper archives and signatures.
2. T-1406 is done: DNS and write deadlines with the reconnect proof.
3. T-1401 is done: the compiled shim with the exit-42 proof.
4. T-1003 is done: rung-complete FUSE, armed tmpfs entry, OCI-tarball rootfs.
5. T-1407 through T-1421 are done: the fifteen findings with their drives.
6. T-1350 and T-1112 stay partial: one KVM run with the operator present,
   then ReactOS. Deferred, not closed.
7. T-1559 through T-1569 are done: the three tool crates with their
   shims, proofs, and plants. The nightly wiring for `release-notes`
   lands with them; a live tag run owns the end-to-end proof.

An agent must not start a KVM guest without the operator present.
Read each entry's exact proof and referenced source before implementation.

## Publication

Beta.10 is published at build commit `d6cb926`. Beta.11 is published
with a version-string defect: its binary reports beta.10. Beta.12 is
published at build commit `067c0ce` with twenty-one closures: T-1003,
T-1401, T-1405, T-1406 through T-1421, T-1422, and T-1423.
No operator decision is pending.
