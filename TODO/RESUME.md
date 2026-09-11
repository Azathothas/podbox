# RESUME

⛔ **A dead man's switch, not a record.** This file is overwritten at the start
of every session and refreshed while the session runs. It carries what is in
flight right now and nothing else. [PROGRESS.md](PROGRESS.md) is the record and
the work order.

## The task

Session of 2026-09-11. Update the orientation rules after the move to the new
home. Reconcile the entries. Research four external repositories. Get the tree
clean and the CI green.

## The resume point

Task 1, item 1: write the six new operator rules into the files that own them.

## In flight

Nothing is half-written. No source file is open.

## The state of the tree

Clean at the start, at `b27b3d9`. The gate on `main` was green.
Four pull requests change the corpus and must not be merged. Two pull requests
break the build and need a code fix.

## Settled this session

The operator ruled on four questions. Each ruling is written into the entry
that owns it.

| question | the ruling |
| --- | --- |
| the corpus pull requests | close them, and fence `references/` off from every updater |
| the ownership memo | it lives on the host, beside the container record |
| the identity calls | honest failure by default, and a flag turns on the lie |
| the host CA bundle | keep the current behaviour |

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
```
