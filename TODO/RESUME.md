## The task

None open. The client-beta batch is finished, `v0.1.0-beta.6` is
published and verified, all twelve open issues are closed with
proof comments, and the T-0210 follow-up is implemented and driven
as commit `9899280`.

## The resume point

Teardown is still owed: `gc --apply` the kept lane job container,
remove session scratch, stop the podman machine this session
started (found stopped), tree clean, gate green. Then continue
with T-1112 per PROGRESS.

## In flight

Nothing half-written. Commits `6d86129` (non-tag work), tag
`v0.1.0-beta.6`, `bc7e39a` (T-1328/T-1334 close-out) and `9899280`
(210 fix) are pushed; the PROGRESS/RESUME record of the
issue-closing pass is next.

## State

Tree clean at `9899280` plus the uncommitted record change. Gate
green on the record bytes (check-todo ok, host ps1 gate 10 passed
1 skip). 165 entries: 0 open, 1 partial, 0 blocked, 164 done. No
open issue remains. The partial entry is T-1109; T-1112 is the next
schedulable work, staying P3.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Read TODO/PROGRESS.md first, then TODO/RESUME.md. Teardown is owed
from the issue-closing session (lane job gc, scratch, stop the
podman machine, tree clean, gate green), then continue with T-1112
per PROGRESS. Work unattended; push straight to main with no
branches.
```
