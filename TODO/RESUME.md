## The task

Session of 2026-09-18. It starts at `2026-09-18T15:03:32Z`. It works the order in [PROGRESS.md](PROGRESS.md). Items 1 and 2 are done and committed: T-0702 with T-0706, then T-0703. Item 3, T-1110, is next.

## The resume point

[PROGRESS.md](PROGRESS.md) carries the work order. Item 1 is [T-1110](milestones.md), M6 acceptance, with [T-0712](interpose.md) as its rule.

## In flight

Nothing is half-written. The tree is clean and the gate is green.

## The state of the tree

Clean, on `main`. `check-todo.py` passes through `py`: 132 rows, 39 open, 3 partial, 0 blocked, 90 done. The guest ran the full `dev.sh check` green plus 105 (A/B, CDE need docker), 159 and 161; reports are at `experiments/results/interpose-placement.txt` and `experiments/results/interpose-paths.txt`. The host gate passes. This session's guest jobs are removed; the four older ones stay as found.

Findings kept: the flaky lock-table reds (three runs, recorded in PROGRESS for T-1204); three amended `Prove` lines with reasons in their entries.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
Push straight to main. Create no branches.
```
