# Repository audit review

## Scope

The review covers the audit diff from source commit `005638d`: live documents,
task records, generated source state, build freshness, Windows job and KVM
drivers, package and licence scripts, workflows, saved evidence, and the
selected pg-toolkit capture. The capture is reference data, not linked code.
Its provenance limits the source reading; no full-tree review is claimed.

## Path review

Read CLI dispatch, entry selection, loader use, helper discovery, Windows
provision and run paths, and every changed script entry point.
Read live document links and their shell or PowerShell command paths.
Checked the copied source against the pinned archive and retained notices.

Findings corrected: stale loader attribution, unsupported namespace claims,
uncompiled passwd shim claims, obsolete SSH work orders, and a missing
default artifact export. T-1003 and T-1401 retain their omitted acceptance.
Interposer generation now precedes CLI compilation in every dev build path.
Git metadata is assigned before export, so an empty override cannot hide HEAD.

KVM review found an observer that searched for the base path instead of
the overlay. Private runtime scratch now makes the emulator path explicit.
Provision has two waits. The outer test bound includes both, and cleanup
checks the executable field and the owned scratch argument before cleanup.
The earlier selector matched its observer; experiment 400 tests that case
with controlled real processes. Complete failure output and exit-2 skip
handling are tested in both gate runners by experiment 398.

## Failure review

Read each new assertion, refusal, return code, and cleanup path.
The byte fixture changes source content without changing size or time.
It also changes each helper and removes an output. Those cases are refused.
Native package smoke rejects a missing helper and invalid ELF with their
own messages. All 44 gate plants were caught. Four clean controls were quiet.

Three Linux repetitions failed document checks. The failures included an
unlinked live page, a repeated sentence, and retained template text. Their negative
records remain tracked. The corrected full run passed 12 steps.
The default wrapper also passed and returned all five matching binaries.
The Linux common check skips remote reads and platform twins; host strict
verification must cover them before completion.

The KVM failures remain failures. Earlier success cannot replace them.
T-1350 remains partial until the current guest acceptance passes.
T-1405 remains partial until the seven-target release matrix and signatures pass.

## Evidence and resume review

Compared current declarations, Git refs, patch identities, task counts,
status fields, saved commands, host conditions, and publication state.
Seven publish patches are equivalent. The remaining context difference
has no unique runtime implementation. A merge would repeat completed work.
Issues 13 and 26 are closed; their pending-comment text was corrected.

Current runbooks remove repeated approvals, fixed session quotas, and
invented review findings. Source state is generated and checked by plant 31.
The cold-start route uses tracked scripts and explicit external inputs.
The licensed Windows disk remains an operator input. Ignored diagnostics
are historical investigation aids, not requirements for future work.

Remaining limits are owned by T-1003, T-1112, T-1350, T-1401, T-1405, and
T-1406. CI and release results must be added after their actual completion.

## Reconciliation after the host failure

The audit session did not commit. Its last KVM run left an emulator that
SIGKILL did not remove, and the Windows host then failed. The operator
removed the WSL distributions and stopped the processes. A second session
started from the staged audit, rebuilt `wsl-toolkit-podbox` from its image,
and did not run a guest.

Code pass: read `podbox_windows::run`, `provision`, `dos::run`, and the KVM
driver against the failed third result. `run` piped the emulator's stdout
and stderr and read neither before exit. A full pipe blocks the emulator,
and the run reports a guest timeout. The streams now go to null and to a
run-directory log. Experiment 401 proves the test and its mutation.
`provision` and `dos::run` already use null streams.

Failure pass: the retained third result shows `FAIL: owned emulator still
exists` after SIGKILL. Estimate, not measured: the emulator was in an
uninterruptible kernel wait, because a zombie has no matching arguments.
The run did not record the process state. podbox cannot correct that state. The driver now
refuses without the operator flag, beside another emulator, or below
6144 MiB available. The base enforces no memory limit.

Record pass: checked every live reference to the KVM command, each task
status against its proof, the publish-branch patch identities again with
`git cherry`, and the release notes gate. The verbatim pre-audit document
copy was removed; commit `005638d` holds the same text.
