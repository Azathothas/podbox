# Progress

## State

225 entries: 11 open, 3 partial, 0 blocked, 211 done.

The 11 open entries are the CI remainder below, filed on 2026-10-02 as
tracked tasks with a Prove clause each, replacing the operator-blocked
list and the untracked `refactor/DEFERRALS.md`. Read
`TODO/RULES.md` section 11 before taking any of them: the operator
settled four standing decisions on 2026-10-02 and they change how the
work is done, not only what it is.

`refactor/DEFERRALS.md` is superseded and is no longer the record of
what waits. Its four sections now live in tracked entries: the CI
failures as T-1601 through T-1604, the KVM and emulator condition
under T-1350 and T-1112 below, the corpus deletion as T-1606, and the
unbatched plan rows as T-1607. The file itself stays untracked on
purpose: `.gitignore:147` records that tracking `refactor/` turns the
record gate red, measured 2026-10-01 at 1047 problems from that
directory alone.

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
6. T-1350 and T-1112 stay partial: one KVM run with the operator
   present, then ReactOS. Tracked and open, not deferred.
7. T-1559 through T-1569 are done: the three tool crates with their
   shims, proofs, and plants. The nightly wiring for `release-notes`
   lands with them; T-1605 owns the live tag proof.
8. T-1601 through T-1604 clear CI: the 1.99 interpose lint, the
   registry fixture dead code, the secrets check, and the detached
   stdio cascade. Take T-1601 first; it also clears the plant MISSes.
9. T-1607 files the 21 unbatched plan rows. T-1606 repoints the
   `references/` citations. T-1605 pushes the version tag.
10. T-1609 then T-1610 unblock the KVM proofs T-1350 and T-1112.
    T-1611 repoints the code maps after the Batch 3 port.

An agent may start a KVM guest unattended. The operator reversed
the 2026-09-30 rule on 2026-10-02, conditional on a watchdog that runs
outside the guest and removes an emulator that outlives its bound. See
T-1609, which builds that watchdog; until it lands, the driver still
refuses without `--accept-host-risk`.
Read each entry's exact proof and referenced source before
implementation.

## CI remainder

Main CI is red on all four jobs, measured on run 36894578988 at
`b9ed3ac`. Every cause below was read off the run's own log rather
than carried from a report. Stable Rust moved from 1.98.1 to 1.99.0
between the 12:17 and 16:10 UTC runs on 2026-10-01
(`rust-toolchain.toml` floats on `stable`).

Four failures, four tracked entries. The rustfmt drift spots the
previous session reported are fixed and the `fmt` step passes on that
run.

| id | failure | reading |
| --- | --- | --- |
| T-1601 | `invalid definition of the runtime `open` symbol` at `crates/podbox-interpose/src/lib.rs:2144`, deny-by-default, takes the static build, the interposer tests, and plants 32e and 32f | T-1601 settles it: allow the lint at the definition. Do not pin the toolchain. |
| T-1602 | clippy dead code, `crates/podbox-image/tests/common/registry.rs:30`, four unread fields | T-1602 makes them needed. Do not delete the fields. |
| T-1603 | `check-no-secrets --public` exits 1 on `scripts/dev-lane.sh:205` `/home/toolkit` | T-1603 removes the check and adds trufflehog. Reproduced locally, not assumed. |
| T-1604 | `detached_stdio` 1 passed, 2 failed | T-1604: one defect, two symptoms. The `:138` PoisonError is the cascade, not a second defect. |

The plant step's `MISS 32e` and `MISS 32f` lines read "red for
another reason, not this one". They are T-1601's consequence, not a
separate task, and they need no entry of their own. T-1601's Prove
clause covers them.

## Standing decisions

Four, settled by the operator on 2026-10-02 and recorded in
`TODO/RULES.md` section 11, with the read-write grant in section 2.
They bind every future session and are not restated anywhere else:

1. Build compatibility is not a goal. Users take a published binary.
   Take whatever toolchain feature unlocks the task, nightly included.
   Fix a toolchain rejection rather than asking for a pin.
2. Dead code is a fault of the reader until proven otherwise. Never
   delete a field to quiet a lint.
3. No deferrals. Work that needs a human becomes a tracked task with a
   named clearing condition. Batch for a later queue, then finish it.
4. Read and write on this repository is authorized, including tags and
   releases. It does not extend to another repository.

## What no longer waits for the operator

| item | before | now |
| --- | --- | --- |
| toolchain pin or port | operator call | settled, T-1601 |
| registry fixture dead code | operator call | settled, T-1602 |
| secrets-check pattern | operator call | settled, T-1603 |
| version tag and release | operator action | authorized, T-1605 |

The KVM and emulator work no longer needs a person in the room. The
operator permitted an unattended run on 2026-10-02, conditional on the
watchdog T-1609 builds. Two measured blockers stand between here and
the proof: T-1609 the watchdog, and T-1610 the missing qemu and OVMF
packages. T-1350 and T-1112 stay `partial` until both clear.

## Publication

Beta.10 is published at build commit `d6cb926`. Beta.11 is published
with a version-string defect: its binary reports beta.10. Beta.12 is
published at build commit `067c0ce` with twenty-one closures: T-1003,
T-1401, T-1405, T-1406 through T-1421, T-1422, and T-1423.
No operator decision is pending.

## Session 2026-10-02, decision round

| Row | Evidence |
| --- | --- |
| Elapsed | 06:59 to 07:40 local, 41 minutes |
| Commits | `b6da03b`, then `246b462` reverting a bad commit; both on `main` |
| Work | Ten questions asked and settled; eight entries filed (T-1601 to T-1608); no source changed |
| Changes | 10 files under `TODO/`, 560 insertions, 43 deletions |
| Size | `git diff --shortstat` over `b9ed3ac..b6da03b` |
| Checks | Record gate exit 0, 214 rows then 222; seven maintained checks exit 0; `check-one-home` exit 1 and `check-no-secrets` exit 1, both owned by T-1608 and T-1603 |
| CI | Run 36952440949 on `b6da03b`; the prior run 36894578988 was read for all four causes |
| Cost | - |
| Health | Tree clean and pushed at the close; no guest started; no engine state changed |

### A commit made during this session was wrong, and it was reverted

Commit `0c0ae8b`, titled "Record the 2026-10-02 session summary beside
the record", contains one inserted line and nothing else:
`assert!(false, "plant crates/podbox-cli/tests/qol.rs")` at
`crates/podbox-cli/tests/qol.rs:176`. It is a live plant, and it was
committed while the plant harness was still running in the background.

The cause is the order of operations, not a mistake in the content: the
plant writes its defects into tracked files and restores them at the
end, so the tree is briefly red with defects that are not real. This
session ran `git add -A` while that window was open and swept a planted
line into a commit. `246b462` reverts it. The session summary the commit
claimed to record was lost the same way, when the plant restored
`TODO/PROGRESS.md` from its own backup; it is rewritten here.

Two consequences for the next session. The record gate exits 0 over a
tree carrying a planted `assert!(false)`, because it reads records and
not test bodies, so the gate is not evidence that the tree builds. And
no session may run `git add -A` while `podbox-plant` is running. Either
finish the plant and read its exit code first, or stage explicit paths.
The same applies to any tool that edits tracked files and restores them
at the end.
