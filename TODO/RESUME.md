## The task

Continuous session of 2026-09-22. T-0415 (device stand-in type check)
is implemented, tested and recorded in the worktree: shape pre-check
in `devices.rs`, three unit tests, entry Decision taken with the
replace-loudly/refuse-narrowly ruling. Commit, push, verify CI, then
pick the next entry. Push straight to main, no branches. Engine clauses
run on host podman; stop `podman-machine-default` at close-out.

## The resume point

Commit the T-0415 change (`devices.rs`, `TODO/complete.md`,
`TODO/INDEX.md`, `TODO/PROGRESS.md`, three sweep transcripts, this
file), push origin main, verify the CI run by re-listing. Then the next
open entry from the pool: T-0413/T-0414 (complete), image T-0205-0209,
T-1109 (partial), T-0503 (partial), T-0606, packaging T-1002-1004.

## In flight

T-0415 change complete in the worktree, uncommitted. Full lane check
green (rc=0), devices suite 47 of 47, host gates green, 240 exit 0 at
10 of 10 with device rows identical. No code is half-written.

## State

Tree dirty with the T-0415 change only. Podman machine
`podman-machine-default` running. No lane job running.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-0415 is implemented and green, uncommitted in
the worktree. Commit it whole (no subset staging without re-gating),
push straight to main with no branches, verify CI by re-listing, then
continue with the next open entry. Engine clauses run on host podman;
stop the podman machine at close-out.
```
