## The task

Continuous session of 2026-09-22. Ten tasks: T-0413, T-1003, T-1109
re-drive, T-1207/T-1112 ruling surfaces, beta publish plus verify, and
the close-out (gc, push, CI verify, record rewrite). All ten commit here.

## The resume point

Tree clean at the close-out commit. Counts: 142 entries, 2 open, 1
partial, 2 blocked, 137 done. Every P0 is done. Next: the operator rules
on T-1207 (gate.md items 2 to 4) and T-1112 (milestones.md, parked or
not); T-1109 (milestones.md) carries its remaining conditions in its own
file.

## In flight

Nothing half-written. No lane job running. Kept wsl-toolkit job
containers pruned this session (`gc --apply`); past results already live
in `TODO/`.

## State

The wsl-toolkit base is usable: probe jobs and the full `dev.sh check`
run through `sh scripts/windows/run-in-base.sh` against
`wsl-toolkit-podbox`. The host-podman substitute
(`scripts/windows/run-via-host-podman.sh`) stays as the fallback, not the
route. Podman machine state on the host is left as found. Beta
`v0.1.0-beta.1` is a pre-release with the verified musl binary beside it.

## Standing operator rulings, 2026-09-22

- Pruning is authorized: remove kept podbox containers and base wsl
  machines we do not use and will not need, safely, touching nothing else.
- Beta-binary publishing is authorized once the top-10 priority tasks
  finish and the session ends, under a pre-release tag. Work first.
  Spent this session: `v0.1.0-beta.1` ships as a pre-release.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: the close-out is pushed (tree clean, CI green) and
counts read 142 entries, 2 open, 1 partial, 2 blocked, 137 done. Next:
rule on T-1207 (gate.md items 2-4: interpose.map count, per-libc size
baseline, third-state reporting — dev.sh check or a slower gate) and
T-1112 (milestones.md non-Linux guest — parked or not). Push straight to
main with no branches.
```
