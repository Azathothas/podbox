## The task

Continuous session of 2026-09-22. T-1107 closed in the worktree: both
milestone halves driven green on host podman, entry Prove amended
(`-v` to `PODBOX_MAPS`). Commit whole, push, verify CI, continue.

## The resume point

Commit the T-1107 change whole (milestones.md, INDEX.md, PROGRESS.md,
this file), push origin main, verify the CI run by re-listing. Next
pool: T-0413/T-0414 (complete, both undecided), image T-0205-0209,
T-0606, T-1003/T-1004, gate T-1207 (needs a ruling).

## In flight

T-1107 change complete in the worktree, uncommitted. Host gates green.
Drive green (`0:42`, `MAPPED_OK`).

## State

Tree dirty with the T-1107 change only. Podman machine
`podman-machine-default` running. No lane job running.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-1107 closed in the worktree, uncommitted. Gate
the whole tree, commit it whole, push straight to main with no
branches, verify CI by re-listing, then continue. Engine clauses run
on host podman; stop the podman machine at close-out.
```
