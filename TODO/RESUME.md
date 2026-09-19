## The task

Session of 2026-09-19, fourth half. Work order is TODO/PROGRESS.md: T-1209
first (sweep Prove refs to M5 rows, add registry check + plant, close), then
T-1210 through T-1213 one conversion group at a time without changing what
any script asserts. Push straight to main, no branches. Guest lane has no
docker daemon, so engine clauses run on host podman.

## The resume point

T-1209 closed. Next: T-1210 (convert 80, 100, 245 through
`experiments/lib/engine.sh`, run each on host podman, record the runs).

## In flight

Commit B (check 21 + plant 21a/21b + T-1209 close) staged but uncommitted.
Tree on main at `f1e423a` plus the staged change.

## The state of the tree

`check-todo.py` green with the new check (`prove_registry=136`); `plant.sh`
26 caught, 0 missed, 3 controls quiet. Counts: 136 entries, 38 open,
3 partial, 0 blocked, 95 done.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order: T-1209 first,
then T-1210 through T-1213 one group at a time without changing what any
script asserts. Push straight to main and create no branches. Work
unattended; the operator reads the result later. The guest lane still has no
docker daemon (no NET_ADMIN), so engine clauses run on host podman and the
lock-table suite may need more than one pass.
```
