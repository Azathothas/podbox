## The task

Continuous session of 2026-09-22. T-1002 verified against the tree and
closed without new code (embed, atomic placement and LD_PRELOAD merge
all present; adapted Prove driven green). Commit whole, push, verify
CI, then continue.

## The resume point

Commit the T-1002 change whole (packaging.md, INDEX.md, PROGRESS.md,
this file), push origin main, verify the CI run by re-listing. Next
pool: T-0413/T-0414 (complete, both undecided), image T-0205-0209,
T-0503 (partial), T-0606, T-1003/T-1004, gate T-1207 (needs a ruling).
A check-22 ordering guard (Done-before-amendment) is owed its own
change with a plant.

## In flight

T-1002 change complete in the worktree, uncommitted. Host gates green.
T-1109 CI watch still pending from the earlier push.

## State

Tree dirty with the T-1002 change only. Podman machine
`podman-machine-default` running. One CI watch job pending.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-1002 verified and closed in the worktree,
uncommitted. Gate the whole tree, commit it whole, push straight to
main with no branches, verify CI by re-listing, then continue.
Engine clauses run on host podman; stop the podman machine at
close-out.
```
