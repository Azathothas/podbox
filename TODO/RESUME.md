## The task

Done. T-1109 closed on its Prove, polish in, `v0.1.0-beta.7` tagged at
the green commit `4f087d0` and verified through the beta.6 pipeline
unchanged (seven legs green, twenty-one assets, two bundles verify,
notes read back with gate plus boundary).

## The resume point

Nothing is open: 165 entries done, no open issue, the final beta
published and verified. The next session takes whatever the operator
orders. Start it per AGENTS.md: session-start, PROGRESS, RESUME, gate.

## In flight

Nothing half-written. The close-out record is the uncommitted change
below (this file plus PROGRESS with the beta.7 verification). The lane
gate runs before the commit; teardown is done.

## State

Tree at `4f087d0` plus the uncommitted record change. 165 entries: 0
open, 0 partial, 0 blocked, 165 done. Tag `v0.1.0-beta.7` sits at
`4f087d0`; nightly run 36117747572 concluded success.

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
