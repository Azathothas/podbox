## The task

Continuous session of 2026-09-22. T-0705 is committed, pushed and CI-green
(`e79725e`, run 35677703589 success). Implement T-0707 (exclusion paths),
then work the open entries. Push straight to main, no branches. Engine
clauses run on host podman; stop `podman-machine-default` at close-out.

## The resume point

Full lane check for the T-0707 tree is running (job `bash-kfr8x9t1`,
`.dev/job-check.sh` wrapper with bootstrap). When green: rebuild the
shipped binary, run `.dev/t0707-drive.sh` (expect D1..D4 RC=0; D1 and D3
are red on the pre-fix binary, measured), re-run `.dev/t0705-drive.sh`
for regression, close the entry with `todo-count --set T-0707 done`,
commit, push origin main, watch CI to green.

## In flight

T-0707 implemented in the worktree, uncommitted: `map::excluded`
(built-in `/proc` tree first, `PODBOX_EXCLUDE_PATH` colon list second)
consulted from `longest` and `longest_to`, four new unit tests, entry
text still owes the update. Lane job ledger: kept job containers await
GC at close-out (`gc --apply`, live jobs excluded).

## State

Tree dirty, `map.rs` only, HEAD `e79725e`. Interpose unit tests 20 of 20
on the lane (gnu target). Workspace clippy failed once on the
three-leaf tree (message not captured); the rerun carries bootstrap
(musl link needs zig) and will say whether it is code or environment.
`real.rs:157` carries a pre-existing `needless_return` the host clippy
names; the lane will say whether it fires there. Podman machine
`podman-machine-default` running.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-0705 is done and green; T-0707 is implemented
in the worktree and its lane check is the first thing to look at.
Close T-0707, push straight to main with no branches, verify CI,
then work the open entries. Engine clauses run on host podman;
stop the podman machine at close-out.
```
