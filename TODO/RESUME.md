## The task

Continuous session of 2026-09-21. T-1304 is implemented, proven green in
the lane, and (pending the full lane check below) committed and pushed.
Push straight to main, no branches. Engine clauses run on host podman;
stop the podman machine at close-out.

## The resume point

If the lane check is green: commit T-1304's change, push straight to
main, verify CI green. The work order then continues at
`TODO/PROGRESS.md` item 7 (podvm T-1305, the fleet decision).
If the lane check is red: the red is in the final tree, not in 147's
evidence; fix forward in the worktree and re-run the failing gate only.

## In flight

T-1304's change is staged: `TODO/podvm.md` (Status done, Decision
ruled, Done record with five findings plus the curl residual),
`experiments/lib/podvm-guest.sh` (new, shared assembly),
`experiments/146-podvm-initramfs.sh` (refactored onto the lib),
`experiments/147-podvm-exec.sh` (new, exits 0 in the lane),
`experiments/results/podvm-exec.txt` (new, the green run) and
`experiments/results/podvm-initramfs.txt` (refreshed: the refactored
146 re-drive, identical bar the date line). Counts are closed
(`todo-count --set T-1304 done`: 140 items, 24 open, 3 partial,
3 blocked, 110 done). `TODO/INDEX.md`, `TODO/PROGRESS.md` and this
file ride in the same commit, as T-1301/T-1302/T-1303 did.
Nothing is half-written.

## State

Host gate green on the closed entry (`check-todo: ok` once PROGRESS
carries the new counts); markers green. Full lane check running as
`.dev/drive147e.log`'s successor (`.dev/fullcheck-t1304.log`):
commit only after `LANE_RC=0` there.
Podman machine `podman-machine-default` running; the operator orders
it stopped at close-out. Lane job ledger: this session's containers
await GC at close-out (`gc --apply`, live jobs excluded).

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-1304 is staged with 147 green in the lane, the
full lane check is running. If green, commit T-1304, push straight to
main with no branches, verify CI, then continue the PROGRESS.md work
order (podvm T-1305). Engine clauses run on host podman; stop the
podman machine at close-out.
```
