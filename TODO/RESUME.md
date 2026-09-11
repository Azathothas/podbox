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

Task 3: verify the one orientation script end to end, then the session-end
protocol from [RULES.md](RULES.md) section 3.

## In flight

Nothing is half-written. No source file is open.

## The state of the tree

The host gate is green. `./scripts/check-todo.py` is green.
⚠ `cargo test --workspace` is NOT deterministic: 5 of 12 runs failed on
2026-09-11 and [T-0215](image.md) carries the measurement.

## Settled this session

The operator ruled on four questions. Each ruling is written into the entry
that owns it and is listed in [PROGRESS.md](PROGRESS.md).

## What this session left for the machine

⛔ **`podman-machine-default` was stopped when this session found it and this
session started it.** It must be stopped again at the end.
[`../docs/containers.md`](../docs/containers.md) holds the rest.

⭐ The distribution `wsl-toolkit-podbox` is meant to persist. Leave it.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
```
