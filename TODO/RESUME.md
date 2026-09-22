## The task

Continuous session of 2026-09-23. T-1314 (packaging.md) implemented and
closed with the beta.3 nightly carrying fourteen assets. Counts:
143 entries, 2 open, 1 partial, 2 blocked, 138 done. Every P0 is done.

## The resume point

Done. Tree clean at the close-out commit on main. CI green on main,
nightly published on `v0.1.0-beta.3`. Next session: T-1207 items 2 to 4
(ruled into `dev.sh check`, each with its plant); T-1109 carries its
remaining conditions; T-1112 parked, T-1212/T-1213 blocked.

## In flight

Nothing half-written. No lane job running. Kept job containers pruned
(`gc --job --apply` on all nine); guest job dirs read back as none.

## State

Tree clean. Gate green on this tree. Podman machine left as found
(running). Base distribution kept.

## Standing operator rulings, 2026-09-22

- Pruning is authorized: remove kept podbox containers and base wsl
  machines we do not use and will not need, safely, touching nothing else.
  Spent: nine kept job containers pruned, nothing else touched.
- Publishing is authorized once the top-10 priority tasks finish and the
  session ends, under a pre-release tag. Work first.
  Spent 2026-09-22: `v0.1.0-beta.1` ships as a pre-release.
  Spent 2026-09-23: `v0.1.0-beta.2` proved the matrix and found the
  publish defect; `v0.1.0-beta.3` publishes the nightly with all seven
  archs, fourteen assets verified back.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Continuous session: T-1314 is filed (tree clean, CI green) and counts
read 143 entries, 3 open, 1 partial, 2 blocked, 137 done. Work
continuously until ten tasks finish in earnest, with T-1314
(packaging.md) last before the gates, then push the next v-tag: it
publishes the nightly pre-release with all seven archs built and
smoke-tested. Push straight to main with no branches.
```
