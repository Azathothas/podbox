## The task

Session of 2026-09-18. It starts at `2026-09-18T15:03:32Z`. It works the order in [PROGRESS.md](PROGRESS.md). Item 1, T-0702 placement with T-0706, is done and committed. Item 2, T-0703, is next.

## The resume point

[PROGRESS.md](PROGRESS.md) carries the work order. Item 1 is [T-0703](interpose.md), the entry-point set and `*at` rules.

## In flight

Nothing is half-written. The tree is clean and the gate is green.

## The state of the tree

Clean, on `main`. `check-todo.py` passes through `py`: 132 rows, 40 open, 3 partial, 0 blocked, 89 done. The guest ran the full `dev.sh check` green plus `experiments/159-interpose-placement.sh` green; the report is at `experiments/results/interpose-placement.txt`. The host gate passes. This session's guest jobs are removed; the four older ones stay as found.

Findings kept: T-0706's `Prove` used a refused `run -v`; T-0702's used `/proc` the chroot lacks; the first resolver declined every dynamic payload on absolute links. Each is amended in its entry.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
Push straight to main. Create no branches.
```
