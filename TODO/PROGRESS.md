# Progress

## State

186 entries: 1 open, 5 partial, 0 blocked, 180 done.

The 2026-09-30 repository audit is complete in source and records.
T-1348, T-1349, and T-1351 are complete. T-1350 lacks a successful current
KVM guest run. T-1405 needs the beta.10 release matrix and signatures.
T-1003 and T-1401 are reopened from omitted acceptance clauses.
T-1112 remains partial on current KVM failures and ReactOS.
T-1406 owns SSH operation deadlines and live reconnect.

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

1. Publish beta.10 and close T-1405 when all helper archives and signatures verify.
2. T-1406: DNS and write deadlines, and the reconnect proof.
3. T-1401: the restricted-server build input and live acceptance.
4. T-1003: FUSE, the permitted tmpfs proof, and the embedded rootfs.
5. T-1350 and T-1112: one KVM run with the operator present, then ReactOS.

An agent must not start a KVM guest without the operator present.
Read each entry's exact proof and referenced source before implementation.

## Publication

Main publication and the beta.10 release are pending in this session.
Publish only after green CI for the exact source commit.
No operator decision is pending.
