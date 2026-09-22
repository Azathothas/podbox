## The task

Continuous session of 2026-09-23. T-1314 (packaging.md) implemented and
closed: nightly workflow, smoke script, cross-link config, riscv64
renameat2, i686 crt-static. Counts: 143 entries, 2 open, 1 partial,
2 blocked, 138 done. Every P0 is done.

## The resume point

Beta.3 nightly is green with fourteen assets verified. Next: commit the
Prove record (this tree), push main, run the close-out (PROGRESS rewrite,
gc, summary, next prompt).

## In flight

T-1314 Prove paragraph filled with the beta.3 run. Files open in it: none
half-written; the staged tree is the change.

## State

Staged, gate `check-todo.py` re-run pending on this tree. No lane job
running. `v0.1.0-beta.2` (matrix green, publish red) and `v0.1.0-beta.3`
(nightly published) both pushed.

## Standing operator rulings, 2026-09-22

- Pruning is authorized: remove kept podbox containers and base wsl
  machines we do not use and will not need, safely, touching nothing else.
- Publishing is authorized once the top-10 priority tasks finish and the
  session ends, under a pre-release tag. Work first.
  Spent 2026-09-22: `v0.1.0-beta.1` ships as a pre-release.
  Next: `v0.1.0-beta.2` publishes the nightly with all seven archs.

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
