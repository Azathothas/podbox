## The task

Session of 2026-09-23, continued. The operator's challenge reopened the
two blocked entries (the wsl-toolkit base is a real machine: install
docker there and measure). Both closed on the base lane: T-1213 native
(20 and 130 exit 0), T-1212 under dockerd 29.8.1 (five scripts exit 0,
300 zero FAILs with one named SKIP, two premise repairs). Pushed to
main in two commits. Beta.5 tagged; nightly in flight.

## The resume point

Watch run 35809485405 (nightly, beta.5) to success, land the T-1314
record as its own commit (mirror the beta.4 record), run the teardown
below, then print the summary table and the next prompt.

## In flight

Teardown owed: stop base dockerd, remove `/root/pb-wk`
`/root/pb-bin` `/root/target-docker.tar` from the base, `gc --apply`
the three kept job containers, podman machine rests stopped. Kept in
the base (documented in PROGRESS): docker, go, jq, qemu-user-static.

## State

Tree dirty with commit C uncommitted (PROGRESS rewrite, RESUME, three
containers.md traps, one gate.md precision line). Gate `check-todo.py`
to be re-run at commit; sources byte-identical to the green checkC
tree. CI gate green on commit A; gate run on commit B in flight when
last seen.

## Standing operator rulings, 2026-09-22

- Pruning is authorized: remove kept podbox containers and base wsl
  machines we do not use and will not need, safely, touching nothing else.
- Publishing is authorized once the top-10 priority tasks finish and the
  session ends, under a pre-release tag. Work first.
  Spent 2026-09-22: `v0.1.0-beta.1` ships as a pre-release.
  Spent 2026-09-23: `v0.1.0-beta.2` proved the matrix and found the
  publish defect; `v0.1.0-beta.3` publishes the nightly with all seven
  archs, fourteen assets verified back; `v0.1.0-beta.4` publishes the
  nightly at the T-1207 close-out commit, fourteen assets verified back.
  Spent this session: `v0.1.0-beta.5` tagged at the T-1212 close-out
  commit, nightly in flight.
- This session's releases are directly ordered: finish the open tasks
  and release a new beta binary, pushed straight to main with no
  branches. Spent: T-1212 and T-1213 closed, `v0.1.0-beta.5` tagged.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Resume: beta.5 nightly (run 35809485405) was in flight; land the T-1314
record, run the teardown in TODO/RESUME.md, then close out the session
(summary table, next prompt). Work unattended; push straight to main
with no branches. End with deep reviews, and only blocked tasks that
genuinely can't be finished without my decision or help.
Release a new beta binary
```
