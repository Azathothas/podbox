## The task

Continuous unattended work is finished: every TODO entry is done
except T-1112 (partial, blocked), zero GitHub issues are open, and
`v0.1.0-beta.8` is released with its nightly pre-release green.

## The resume point

T-1112 partial on one blocker: the DOS and ValidationOS guest arms
run under `tcg` (lane-driven here), the `kvm` arm is unit-tested
only, and no licensed image exists anywhere. What clears it is a
KVM host holding `/dev/kvm` with the image installed under the
accept-terms gate the entry names. (`/dev/kvm` is present on the
wsl-toolkit base; the arm owes the image, not the host.)

## In flight

Nothing half-written, nothing half-run. Two open questions for the
next session, neither needing the operator: the gate rate ruling
(T-1204 neighbourhood) and the blob retry comment contract
(`registry.rs`, PROGRESS names both).

## State

Tree at `7e43f23` plus the closeout change (PROGRESS rewrite,
history move, this file). Tag `v0.1.0-beta.8` on `7e43f23`,
nightly workflow green, 21 assets published. Lane ledger at 0
open records; verify with `gc` at session start per the rulings.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Read TODO/PROGRESS.md first, then TODO/RESUME.md. The work order in
PROGRESS holds one blocked item (T-1112 kvm arm plus licensed
image); until a KVM host with the image exists there is nothing to
implement, so start new work by authoring entries per the
methodology, never by implementing in the same pass. Update counts
only via scripts/todo-count.py plus scripts/check-todo.py. For
conclusions that ship, enumerate at least three candidate
explanations before testing, test to refute, then do one more pass
for what is missing. Close each GitHub issue only with a proof
comment showing fix commit, drive output, and guard that stops
recurrence. Work unattended; push straight to main with no
branches.
```
