## The task

Session of 2026-09-19, second half. The operator asked whether host podman
can replace the broken in-guest docker daemon, then directed the rewrite:
safer scripts, proper docs, record current, three deep reviews.

## The resume point

Done and committed. The next session files the remaining fourteen engine
conversions as entries and continues the work order in [PROGRESS.md](PROGRESS.md).

## In flight

Nothing is half-written. `experiments/lib/engine.sh` is the one way scripts
reach an engine; `105` is converted and green through host podman with
`experiments/results/interpose-ownership.txt` carrying the run (every check
ran and matched, exit 0). Lane docs and the record are updated in the same
change.

## The state of the tree

Clean, on `main`. Host `check-todo.py`, markers, secrets, one-home and docs
pass; shellcheck is clean on both shell files apart from the tree-wide
`CDPATH` idiom lines. The guest ran the full `dev.sh check` green.

Findings kept: base `podman run` needs `C:/` spellings and `MSYS_*` flags;
`chmod` is a silent no-op on this checkout; `base exec` mangles argv for
automount binaries and drops trailing words intermittently, so
argument-carrying commands run inside containers instead. Fourteen engine
scripts still await conversion.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
Push straight to main. Create no branches.
```
