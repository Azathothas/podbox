## The task

Continue the batch: T-1336 (ASCII scrub) is next in entry order, then
T-1327/T-1328/T-1329/T-1334 per PROGRESS. T-1328/T-1329/T-1334 prove
against the next beta's release; tag `v0.1.0-beta.6` when the
non-tag work is green.

## The resume point

T-1336 first: scrub `crates/podbox-cli/src` printed strings to ASCII
(comments stay), add the gate guard, lane-prove. Commit at `2cc8bfd`
(T-1335 close) is pushed; tree clean, gate green.

## In flight

Nothing half-written. T-1335 closed as `2cc8bfd` with
`experiments/351-signal-forward.sh` (exit 0 in lane) and
`experiments/results/signal-forward.txt`.

## State

Tree clean at `2cc8bfd`. Gate green (check-todo ok, check-gate
--fast 10 passed 1 skip). 165 entries: 5 open, 1 partial, 0 blocked,
159 done.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Read TODO/PROGRESS.md first, then TODO/RESUME.md. Continue the podbox
batch from commit 2cc8bfd (T-1335 closed): implement T-1336, then
T-1327/T-1328/T-1329/T-1334 per PROGRESS, tagging v0.1.0-beta.6 when
the non-tag work is green. Work unattended; push straight to main
with no branches.
```
