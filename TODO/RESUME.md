# RESUME

⛔ **A dead man's switch, not a record.** This file is overwritten at the start
of every session and refreshed while the session runs. It carries what is in
flight right now and nothing else. [PROGRESS.md](PROGRESS.md) is the record and
the work order.

## The task

Session of 2026-09-11, the reference sweep, ended cleanly. Nothing is in flight.

## The resume point

[PROGRESS.md](PROGRESS.md)'s work order, item 1: [T-0215](image.md).

⚠ **Item 2 now depends on item 1.** [T-0211](image.md) was moved from `done` to
`partial` in this session because both tests its `Prove` names are in T-0215's
measured-intermittent set. Do not close it on a single green run: that is the
mistake the baseline already records against this suite.

## In flight

Nothing. No file is half-written and no branch is open.

## The state of the tree

Clean, on `main`, level with `origin/main`. `./scripts/check-todo.py` is green
and `sh scripts/common/check-gate.sh --fast` is green on this host.

⭐ **`main` accepts a direct push.** `enforce_admins` was turned off on
2026-09-11 and a push was verified. The four required checks still run.
[RULES.md](RULES.md) section 2 carries the rule and the fallback. ⛔ Create no
branches.

⚠ **One check is intermittent and it is not fixed.**
`cargo test --workspace` failed 5 of 12 runs on 2026-09-11, across four store
lock tests. [T-0215](image.md) carries the measurement. A red run of
`workspace and interposer tests` is that race until somebody proves otherwise,
and the operator's standing instruction is to re-run it.

## The corpus

⭐ **Eleven trees were mined on 2026-09-11 and all of them are tracked.**
`references/` is now 41 trees, 166 MB over 7,815 tracked files.
[reference-map.md](reference-map.md) carries a licence determination and a
verdict for every one, and
[`../docs/history/2026-09-11-reference-sweep.md`](../docs/history/2026-09-11-reference-sweep.md)
states **the depth reached per tree**. ⚠ Three of the eleven were read at one
pass, so check the depth before leaning on one.

⛔ **Do not re-mine them.** They are at captured commits with zero gaps, and
their trackers are in `api/`.

## The machine

⭐ The distribution `wsl-toolkit-podbox` persists on purpose, with a warm image
cache. Leave it.

⛔ `podman-machine-default` is not this project's. Measured at the end of this
session with `podman machine inspect`: **`state=stopped`**, which is how it
should be left. ⚠ `./scripts/session-start.sh` brings the base up and its probe
can start this machine, so check the state at the end of a session rather than
assuming it.
⚠ **`eph-pgb` is not on this host.** `podman machine list` reports one machine
and that is the one above. A previous record named `eph-pgb` as somebody else's;
whatever it was, it is not here now.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
Push straight to main. Create no branches.
```
