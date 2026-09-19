## The task

Session of 2026-09-19, fourth half. Work order is TODO/PROGRESS.md: T-1209
first (sweep Prove refs to M5 rows, add registry check + plant, close), then
T-1210 through T-1213 one conversion group at a time without changing what
any script asserts. Push straight to main, no branches. Guest lane has no
docker daemon, so engine clauses run on host podman.

## The resume point

T-1209 in progress: Prove-block scan measured 41 violating lines plus 2
history notes to reword plus 1 tag alignment, against the entry's 37.
Sweep next, then the check and plant.

## In flight

Nothing half-written. Tree clean on main at `bfe7ef4`. Host podman answers
(5.8.6 client, linux backend). `check-todo.py` green at start.

## The state of the tree

Clean, on main. Host `check-todo.py` green (exit 0). Guest/workspace checks
not yet run this session.

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
