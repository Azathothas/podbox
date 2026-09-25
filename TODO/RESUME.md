## The task

Release the final beta (ruled 2026-09-25): T-1109 closed on its own
Prove, T-1112 already done, polish in. Tag `v0.1.0-beta.7` as a
nightly pre-release through the beta.6 pipeline unchanged.

## The resume point

The non-tag change sits in the tree below, uncommitted. Run the gate
(host plus lane), commit, push straight to `main`, and tag only when
the gate on that commit is green. Then the bundles verify and the
notes carry the gate state plus the boundary, each read back, before
anything is called done.

## In flight

T-1109 close (251 script plus two result files, entry Done with the
drive and the census cause), version bump to 0.1.0-beta.7 with the
lock regenerated, changelog entry, PROGRESS rewrite. Nothing is
half-written; the lane control rerun reproduced the drive and named
the census cause.

## State

165 entries: 0 open, 0 partial, 0 blocked, 165 done. No open issue
remains. Host `check-todo` green; the lane `dev.sh check` runs before
the commit.

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
