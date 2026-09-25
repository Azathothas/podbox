## The task

None open. The client-beta batch is finished: T-1328 and T-1334
closed on `v0.1.0-beta.6`, and the nightly (run 36110971684)
concluded success with twenty-one assets, verified back through the
release API.

## The resume point

Teardown is still owed: `gc --apply` the kept lane job container,
remove base scratch, stop the podman machine this session started
(found stopped), tree clean, gate green. Then the operator-owed
close-out comments on issues 13 and 26.

## In flight

Nothing half-written. Commit `6d86129` (non-tag work) and tag
`v0.1.0-beta.6` are pushed; the T-1328/T-1334 close-out commit is
next.

## State

Tree clean at `6d86129` plus the uncommitted close-out change. Gate
green on the close-out bytes (check-todo ok, host ps1 gate 10
passed 1 skip). 165 entries: 0 open, 1 partial, 0 blocked, 164
done. The partial entry is T-1109; T-1112 is the next schedulable
work, staying P3.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Read TODO/PROGRESS.md first, then TODO/RESUME.md. Teardown is owed
from the beta.6 session (lane job gc, base scratch, stop the podman
machine, tree clean, gate green), then continue with T-1112 per
PROGRESS. Work unattended; push straight to main with no branches.
```
