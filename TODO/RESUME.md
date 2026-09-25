## The task

Finish all tasks and release the final beta binary (ruled
2026-09-25): T-1109's remaining conditions plus T-1112 plus release
polish (version bump, changelog pass, doc sweep), then tag
`v0.1.0-beta.7` as a nightly pre-release through the beta.6
pipeline unchanged.

## The resume point

Start with T-1109/T-1112 per PROGRESS. Tag only when the non-tag
work is green; the bundles verify and the notes carry the gate
state plus the boundary, each read back, before anything is called
done.

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
Read TODO/PROGRESS.md first, then TODO/RESUME.md. Finish all tasks
and release the final beta binary: T-1109's remaining conditions
plus T-1112 plus release polish (version bump, changelog pass, doc
sweep), then tag v0.1.0-beta.7 as a nightly pre-release through the
beta.6 pipeline unchanged (tag only when green; bundles verify and
notes carry gate plus boundary, each read back). Work unattended;
push straight to main with no branches.
```
