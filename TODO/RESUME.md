## The task

Continuous session of 2026-09-22. T-0606 is done in the worktree,
uncommitted. Gate the whole tree, commit it whole, push straight to main
with no branches, verify CI by re-listing, then continue at T-1003's CLI
wiring (P2, the last packaging leg) or T-0413 (P1).

## The resume point

Tree clean at the T-0606 commit. Counts: 142 entries, 4 open, 1 partial,
2 blocked, 135 done. Every P0 is done. Next pool: packaging T-1003 (open:
CLI wiring reads `PODBOX_MODE` and feeds `Availability`, then the Prove
drive), complete T-0413 (P1), gate T-1207 (needs a ruling), T-1112 (needs
an operator ruling: surface, do not implement).

## In flight

Nothing half-written. No lane job running. Kept wsl-toolkit job
containers are pruned at session end (`gc --apply`); past results already
live in `TODO/`.

## State

The wsl-toolkit base is usable: probe jobs and the full `dev.sh check`
run through `sh scripts/windows/run-in-base.sh` against
`wsl-toolkit-podbox`. The host-podman substitute
(`scripts/windows/run-via-host-podman.sh`) stays as the fallback, not the
route. Podman machine state on the host is left as found.

## Standing operator rulings, 2026-09-22

- Pruning is authorized: remove kept podbox containers and base wsl
  machines we do not use and will not need, safely, touching nothing else.
- Beta-binary publishing is authorized once the top-10 priority tasks
  finish and the session ends, under a pre-release tag. Work first.

## The paste

```text
note: The prompt below is stale, and some work is already done. get oriented and reconcile first (wsl base is fixed i believe), and then work.

---
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-0209 is pushed (7579adc, CI green) and the tree is
clean. Continue the PROGRESS.md work order at T-0207 (bounded-concurrency
layer fetch) or T-1003 (launch ladder). Push straight to main with no
branches. The wsl-toolkit base is still unusable; use the host-podman
substitute recorded in the T-0209 entry until the base is repaired

Open questions answered:
- base-recreate authorized (prune all podbox containers/base wsl machines we don't use/won't need, safely, don't touch anything ense )
 - beta-binary publishing authorized, publish it once some of the more critical/high priority tasks finish and session ends (work first, finish the top 10 most high priority tasks first), use a pre-release tag
```
