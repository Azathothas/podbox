## The task

Work the PROGRESS work order entry by entry, unattended, pushing
straight to main. Item 1 first: T-0801 parity rows plus the T-1325
curated-list check and plant.

## The resume point

Item 1 in progress: T-0801 (46 run flags, 10 verbs) with the T-1325
check extension. Next: T-0503 ordering with T-1317.

## In flight

Reading `crates/podbox-cli/src/parity.rs` (done, 856 lines) and the
`run` parser. Nothing half-written.

## State

Tree clean at `0b89c3c`. `check-todo.py` green (168 rows: 3 open,
20 partial, 0 blocked, 145 done).

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
