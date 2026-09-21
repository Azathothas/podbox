## The task

Continuous session of 2026-09-21. T-0408 is done and on main. Next is the
T-1208 plant run on the committed tree, then closing T-1208. Push straight
to main, no branches. Engine clauses run on host podman; stop the podman
machine at close-out.

## The resume point

Check 22 (closure records) with its plant case commits here. After the push,
run the full gate plus `scripts/plant.sh` in a Linux job, record the plant
verdict under T-1208, and close it with `todo-count.py --set`.

## In flight

T-1208 implementation open (check, plant case, five record conversions,
RULES shape sentence, T-1207 correction). No half-written code. Tree state:
verify at resume.

## State (tree dirty, closing)

```text
gate change: check-todo.py check 22, plant.sh case 22, RULES.md section 5,
enter.md T-0505, image.md T-0204, milestones.md T-1103, probe.md T-0107/T-0108,
gate.md T-1207 correction, PROGRESS.md record
```

Counts 138: 31 open, 3 partial, 3 blocked, 101 done. Host reader green with
the new check (closure_records=101).

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: check 22 with its plant case is on main, T-1208 stays
open until the plant run. Run the full gate plus scripts/plant.sh in a
Linux job, record the plant verdict under T-1208, and close it. Push
straight to main, no branches. The guest lane has no docker daemon;
engine clauses run on host podman.
```
