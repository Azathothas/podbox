## The task

Session of 2026-09-19. It reconciled the dirty tree the prior session left
(T-0711 identity implementation, uncommitted) and continued the work order in
[PROGRESS.md](PROGRESS.md). It closed [T-0711](interpose.md) with its proof.

## The resume point

[PROGRESS.md](PROGRESS.md) carries the work order. Item 1 is
[T-0710](interpose.md), the host-side ownership memo with its bounded read.

## In flight

Nothing is half-written. The tree is clean and the gate is green: host
`check-todo.py`, markers, secrets and fmt pass; the guest ran the full
`dev.sh check` green apart from the recorded lock-table flake, and a fourth
`cargo test --workspace` pass on the same tree reads 344 passed, 0 failed.

## The state of the tree

Clean, on `main`, after the commit this session ends with. `check-todo.py`
passes: 132 rows, 36 open, 3 partial, 0 blocked, 93 done. `106` is green
with `experiments/results/interpose-identity.txt` tracked.

Findings kept: the job container cannot run dockerd (no NET_ADMIN,
measured 2026-09-19; `106` routes around it); the lock-table flake fired
6, 5, then 0 of 344 across three same-tree passes today.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
Push straight to main. Create no branches.
```
