# Deferred work: needs a human in the room

Recorded 2026-10-01, unattended session. None of the below was attempted:
no guest was started, no emulator was launched, no preparation toward
running them was made. Each names what clears it and who must be present.

## 1. Ten KEEP-SHELL scripts needing a real emulator or guest

Rule: an agent must not start a KVM guest without the operator present.
`TODO/PROGRESS.md` records that nested KVM stopped the Windows host on
2026-09-30. All ten are KEEP-SHELL on the ledger and stay in shell; the D-10
move to `scripts/` (rewritten, extended, improved) is also deferred for
these ten, because rewriting a measurement untested is fabrication.

| script | needs | clears it |
| --- | --- | --- |
| `experiments/147-podvm-exec.sh` | podvm guest exec | operator-present session with a podvm guest |
| `experiments/290-microvm.sh` | microvm facility | operator-present session with microvm |
| `experiments/357-tcg-profile.sh` | TCG emulation | operator-present session with QEMU TCG |
| `experiments/361-guest-usernet.sh` | guest user networking | operator-present session with a guest |
| `experiments/367-qemu-user-aarch64.sh` | qemu-user aarch64 | operator-present session with qemu-user |
| `experiments/369-windows-tcg-dos.sh` | Windows guest over TCG | operator-present session with the guest |
| `experiments/371-validationos-stream.sh` | ValidationOS stream | operator-present session with the stream |
| `experiments/385-kvm-open.sh` | /dev/kvm | operator-present session on a KVM host |
| `experiments/392-kvm-guest.sh` | KVM guest | operator-present session on a KVM host |
| `experiments/400-kvm-cleanup.py` | KVM cleanup | operator-present session on a KVM host |

Who must be present: the operator, to start and to own the guest. An agent
takes the run output and carries the clauses into Rust or `scripts/` after.

## 2. T-1350 and T-1112

Both parked on the same KVM rule. Same clearing condition as section 1:
an operator-present session. Do not fold them into unattended batches.

## 3. `references/` deletion (end state point 3)

46 repositories, 168 MB, 47 citations from `TODO/reference-map.md`. The
record gate reads every cited path and line (`check_tree`), so deleting
`references/` turns the gate red until every citation is repointed at
podbox's own source or dropped with a recorded reason. Every repoint is
its own task, and no wave in the current plan files them: the next
planning round owns that batch. Clearing condition: a filed repoint batch
with all 47 citations moved, then the deletion. Who: the planning round
first, then any implementer.

## 4. Tasks in no batch, deliberately not taken

T-1550, T-1571, T-1582 to T-1600 (22 tasks). Most are decision records
belonging to whichever wave implements them; T-1550 (the `10`/`20`/`130`
three-way) belongs to the wave that deletes `10`. No missing batch was
filed in INDEX.md: the implementors of each wave record their own
decisions in their entry's Decision field, which is where D-1 through D-10
already live. T-1571 (`153-store-lock-race.sh` clause 7 under T-0215)
waits on the store work it cites.
