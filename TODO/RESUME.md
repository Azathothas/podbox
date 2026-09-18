## The task

Session of 2026-09-18. It starts at `2026-09-18T15:03:32Z`. It works the order in [PROGRESS.md](PROGRESS.md). Items 1, 2 and 3 are done and committed: T-0702 with T-0706, T-0703, then T-1110 with T-0712. Item 4, T-0710 and T-0711, is next.

## The resume point

[PROGRESS.md](PROGRESS.md) carries the work order. Item 1 is [T-0710](interpose.md) and [T-0711](interpose.md), the two ruled implementations.

## In flight

Nothing is half-written. The tree is clean and the gate is green.

## The state of the tree

Clean, on `main`. `check-todo.py` passes through `py`: 132 rows, 37 open, 3 partial, 0 blocked, 92 done. The guest ran the full `dev.sh check` green plus 105 (A/B, CDE need docker), 159, 161 and 245 with 250 clause 1; reports are tracked under `experiments/results/`. The host gate passes. This session's guest jobs are removed; the four older ones stay as found.

Findings kept: the flaky lock-table reds (recorded in PROGRESS for T-1204); three amended `Prove` lines with reasons; `250` clause 2 red on image content (recorded, owned elsewhere).

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
Push straight to main. Create no branches.
```
