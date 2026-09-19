## The task

Session of 2026-09-19, third half. Work order was TODO/PROGRESS.md item 1
T-0710 host-side ownership memo plus filing the remaining engine conversions
as entries per docs/methodology/authoring.md. Push straight to main, no
branches.

## The resume point

Done and committed. Next session continues the work order in
[PROGRESS.md](PROGRESS.md): T-1209 first, then T-1210 through T-1213.

## In flight

Nothing half-written. T-0710 done with 105 checks A-G green (exit 0) and a
guest podbox-run smoke test; T-1210 through T-1213 filed as open entries with
no implementation.

## The state of the tree

Clean, on main. Host check-todo.py green, host gate green (10 passed, 1
skipped), guest workspace green on the eighth pass after seven lock-table
flakes with the recorded 16-lock refusal. 105 green on host podman.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
Push straight to main. Create no branches.
```
