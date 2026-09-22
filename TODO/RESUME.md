## The task

Continuous session of 2026-09-22. T-0503 closed in the worktree: one
shared `ptmx_usable` predicate in `podbox-probe`, three call sites
routed through it, five-case unit test, positive arm driven on host
podman. Commit whole, push, verify CI, then continue.

## The resume point

Commit the T-0503 change whole (probe, cli, enter.md, INDEX.md,
PROGRESS.md, this file), push origin main, verify the CI run by
re-listing. T-1002's CI run should also be re-listed if still pending.

## In flight

T-0503 change complete in the worktree, uncommitted. Full lane check
green (rc=0), host gates green, positive arm driven (`run -t` rc=0).

## State

Tree dirty with the T-0503 change only. Podman machine
`podman-machine-default` running. No lane job running.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-0503 closed in the worktree, uncommitted. Gate
the whole tree, commit it whole, push straight to main with no
branches, verify CI by re-listing, then continue. Engine clauses run
on host podman; stop the podman machine at close-out.
```
