## The task

Session of 2026-09-18. It starts at `2026-09-18T15:03:32Z`. It follows AGENTS.md. It runs startup and stops. It names no TODO entry.

## The resume point

[PROGRESS.md](PROGRESS.md) carries the work order. Item 1 names T-0702, the placement half, with T-0706 first or together. A fresh session starts there.

## In flight

Nothing is half-written. This file is the only change. It holds the startup state.

## The state of the tree

Clean, on `main`, at `3d3d605`. `check-todo.py` passes through `py`: 132 rows, 41 open, 4 partial, 0 blocked, 87 done. The host gate passes: 10 passed, 0 failed, 1 skipped. `dev.sh status` reads `unknown`, because the build never started on this host. That reading is correct on the Windows lane: the build runs in a container inside `wsl-toolkit-podbox`.

The base persists and runs. No container is `Up`: four job records sit `Exited`, all from about two hours before this session. `podman-machine-default` reads `Running`; the last session left it `Stopped`. This session did not start it, and it stays as found.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
Push straight to main. Create no branches.
```
