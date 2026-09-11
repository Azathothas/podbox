# RESUME

⛔ **A dead man's switch, not a record.** This file is overwritten at the start
of every session and refreshed while the session runs. It carries what is in
flight right now and nothing else. [PROGRESS.md](PROGRESS.md) is the record and
the work order.

## The task

Session of 2026-09-11 ended cleanly. Nothing is in flight.

## The resume point

[PROGRESS.md](PROGRESS.md)'s work order, item 1: [T-0215](image.md).

## In flight

Nothing. No file is half-written and no branch is open.

## The state of the tree

Clean, on `main`, level with `origin/main`. The host gate is green,
`./scripts/check-todo.py` is green, and the hosted gate on `main` is green on
all four jobs.

⚠ **One check is intermittent and it is not fixed.**
`cargo test --workspace` failed 5 of 12 runs on 2026-09-11, across four store
lock tests. [T-0215](image.md) carries the measurement. A red run of
`workspace and interposer tests` is that race until somebody proves otherwise,
and the operator's standing instruction is to re-run it.

## The machine

⭐ The distribution `wsl-toolkit-podbox` persists on purpose, with a warm image
cache. Leave it.

⛔ `podman-machine-default` is not this project's. It was stopped when the last
session found it and it was stopped again at the end. `eph-pgb` was never
touched.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
```
