## The task

Continuous session of 2026-09-22. T-1109 stays partial with two
conditions closed: the 250 re-drive in the lane closes the Go decline
and the step clause (after an A/B run proved the step needs an
announced CA bundle, which 250 now provisions). Commit the 250 fix,
the new report and the record, push, verify CI, then continue.

## The resume point

Commit the T-1109 change whole (250 script, new report,
`TODO/milestones.md`, `TODO/PROGRESS.md`, this file), push origin
main, verify the CI run by re-listing. Next pool: T-0413/T-0414
(complete, both undecided), image T-0205-0209, T-0503 (partial),
T-0606, packaging T-1002-1004, gate T-1207 (needs a ruling).

## In flight

T-1109 change complete in the worktree, uncommitted: 250 exits 2 with
zero FAILs (Go green, step reason named, `-t` and census skip by
name). Host gates to run before the commit.

## State

Tree dirty with the T-1109 change only. Podman machine
`podman-machine-default` running. No lane job running.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-1109's 250 re-drive closes the Go and step
conditions (report green except two environmental skips), uncommitted.
Gate the whole tree, commit it whole, push straight to main with no
branches, verify CI by re-listing, then continue with the next open
entry. Engine clauses run on host podman; stop the podman machine at
close-out.
```
