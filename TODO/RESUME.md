## The task

Session of 2026-09-23, continued. T-1207 (gate.md) items 2 to 4
implemented in `dev.sh check` with plants, entry closed, `plant.sh`
31 caught 0 missed. T-1109 re-driven with zero FAILs and the same two
skips, stays partial. Counts: 143 entries, 1 open, 1 partial, 2
blocked, 139 done. Every P0 is done. Beta.4 nightly published with
fourteen assets. Pushed to main in three commits.

## The resume point

Done. Tree clean at the beta.4 record commit on main. CI gate green
on the T-1207 close-out commit; nightly 35805033988 success on
`v0.1.0-beta.4`. Next session: T-1112 stays parked, T-1212/T-1213 stay
blocked; PROGRESS carries the work order.

## In flight

Nothing half-written. No lane job running. Kept job containers pruned
(`gc --apply`: the five session jobs removed, ledger compact).

## State

Tree clean. Gate green on this tree. Podman machine stopped (as
found). Base distribution kept.

## Standing operator rulings, 2026-09-22

- Pruning is authorized: remove kept podbox containers and base wsl
  machines we do not use and will not need, safely, touching nothing else.
  Spent: nine kept job containers pruned last session; five this
  session, nothing else touched.
- Publishing is authorized once the top-10 priority tasks finish and the
  session ends, under a pre-release tag. Work first.
  Spent 2026-09-22: `v0.1.0-beta.1` ships as a pre-release.
  Spent 2026-09-23: `v0.1.0-beta.2` proved the matrix and found the
  publish defect; `v0.1.0-beta.3` publishes the nightly with all seven
  archs, fourteen assets verified back; `v0.1.0-beta.4` publishes the
  nightly at the T-1207 close-out commit, fourteen assets verified back.
- This session's releases are directly ordered: finish the open tasks
  and release a new beta binary, pushed straight to main with no
  branches. Spent: T-1207 closed, `v0.1.0-beta.4` published.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
podbox is a container runtime for agent sandboxes that answers to docker
and podman; the record below is where the work order lives.
Read TODO/PROGRESS.md first (the state, the counts, and the next
schedulable work), then TODO/RESUME.md (where this session stopped).
Then read the gate-check entry your task names, in full: it rules three
interposer checks into dev.sh check, each with its plant in the same
change. Work unattended; push straight to main with no branches.

Finish all open/partial tasks in earnest
For blocked tasks, say what would unblock them; if it doesn't need human intervention, finish them, else print exactly the blocker and solution in table at session end
End with deep reviews, and only blocked tasks that genuinely can't be finished without my decision or help
Release a new beta binary
```
