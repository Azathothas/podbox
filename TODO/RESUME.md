## The task

Work the PROGRESS work order entry by entry, unattended, pushing
straight to main. T-1340 (run-decay isolation) is done 2026-09-27:
`experiments/368-run-decay.sh` exits 0 on the lane
(`experiments/results/run-decay-368.txt`, 11 of 11 predictions
held). Next: author the run-rows-with-keeper follow-up named in
T-1340's Done text.

## The resume point

T-1340 closed. The `--rm` arm deletes the unreferenced rootfs at
exit, so keeper-less runs re-pay extraction; `create` pins the
tree; kept runs ride it warm. No ceiling moves.

## In flight

Nothing half-written. New files staged: `experiments/368-run-decay.sh`
(with exec bit), `experiments/results/run-decay-368.txt`.

## State

Tree at `62ddba5` plus the T-1340 change. `check-todo.py` ok on
the host (169 rows, 0 open, 0 partial, 1 blocked, 168 done) and
the full lane check green (`dev.sh check` rc 0 in a disposable
container). Lane jobs `c300ecf9d36c7583` (368 drive) and
`bf643f1b31dcc77a` (full check) collected with `gc --job
--apply`; ledger at 0 open records. `.tmp/368-out/` holds the
retrieved drive artifact until the results file is committed,
then it goes per RULES section 8.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
Read TODO/PROGRESS.md first, then TODO/RESUME.md. Implement the
work order in PROGRESS entry by entry with fallbacks built in:
a refusal is the last resort after all rungs fail, never the whole
answer, and it names tried rungs plus the missing leg. Close TODO
entries in place with Prove actually run and output recorded
underneath, per RULES section 5. Update counts only via
scripts/todo-count.py plus scripts/check-todo.py. For conclusions
that ship, enumerate at least three candidate explanations before
testing, test to refute, then do one more pass for what is
missing. Close each GitHub issue only with a proof comment
showing fix commit, drive output, and guard that stops
recurrence. Work unattended; push straight to main with no
branches.
```
