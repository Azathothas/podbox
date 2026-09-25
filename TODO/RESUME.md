## The task

Triage all 22 open issues (29-38, 49-60) with the dependabot PR and
the Actions deprecation audit. Done: drives 353/354 green, 20
entries reopened partial with decisions, 3 opened new, README and
parity note fixed, PR 9 merged, no issue closed (each closes with
proof from its fix).

## The resume point

Gate green, tree clean, pushed to `main`. Next session implements
the work order in TODO/PROGRESS.md, entry by entry, unattended.

## In flight

Nothing half-written. The triage commit is pushed. Kept lane job
containers went through `gc --apply`, session scratch removed.

## State

Tree clean. `check-todo.py` green. 168 entries: 3 open, 20
partial, 0 blocked, 145 done. PR 9 merged on origin.

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
