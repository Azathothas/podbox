# RESUME

⛔ **A dead man's switch, not a record.** This file is overwritten at the start
of every session and refreshed while the session runs. It carries what is in
flight right now and nothing else. [PROGRESS.md](PROGRESS.md) is the record and
the work order.

## The task

Session of 2026-09-12, which started at `02:35:53Z`. It took the work order's
items 1, 2 and 3, and the operator called the end-of-session protocol. The
summary beside [`../docs/history/sessions/`](../docs/history/sessions/) carries
the end instant.

## The resume point

[PROGRESS.md](PROGRESS.md)'s work order, item 1: [T-0215](image.md), which is
still `open` and still P0.

⛔ **Read the entry before the number.** T-0215 now carries a measured series, a
refuted mechanism, an unrefuted one, and an instrument with a positive control.
⚠ **Its fork control has been wrong three times**, always because the skip list
was short of what the code does. `experiments/results/store-lock-race.txt`
prints the COMMAND for every clause above its figures, and that command is the
authority on what the figures measured.

## In flight

Nothing is half-written and nothing is uncommitted.

⚠ **One re-run was in flight when the session ended and its result is NOT in
the tree.** `experiments/153-store-lock-race.sh` with clauses `0 1 2 3 6 7` at
20 control runs, launched to re-take clause 6 with all four forking paths in the
skip list. ⛔ **The committed evidence's clause 6 was taken with three**, which
is why T-0215 does not claim a fork verdict. Re-run it and the entry can:

```sh
PODBOX_RACE_RUNS=12 PODBOX_RACE_CONTROL_RUNS=20 \
  PODBOX_RACE_CLAUSES="0 1 2 3 6 7" ./experiments/153-store-lock-race.sh
```

## The state of the tree

Clean, on `main`. `./scripts/check-todo.py` is green and
`sh scripts/common/check-gate.sh --fast` is green on this host, all ten checks.
`sh scripts/windows/run-in-base.sh` ran the complete `dev.sh check` and it
exited 0 with the new step order.

⭐ **`main` accepts a direct push.** `enforce_admins` was turned off on
2026-09-11 and a push was verified twice on 2026-09-12. The four required checks
still run, and they were green on all four at `17a022f`.
[RULES.md](RULES.md) section 2 carries the rule and the fallback. ⛔ Create no
branches.

⚠ **One check is intermittent and it is not fixed.** A red
`workspace and interposer tests` is [T-0215](image.md) until somebody proves
otherwise, and the standing instruction is to re-run it. ⛔ Never ignore or
disable it.

## The corpus

⭐ **41 trees, all at captured commits, all with zero gaps**, and
[reference-map.md](reference-map.md) carries a licence determination and a
verdict for every one.
[`../docs/history/2026-09-11-reference-sweep.md`](../docs/history/2026-09-11-reference-sweep.md)
states **the depth reached per tree**. ⚠ Three of the eleven newest were read
at one pass, so check the depth before leaning on one.

⛔ **Do not re-mine them.** They are at captured commits with zero gaps, and
their trackers are in `api/`.

## The machine

⭐ The distribution `wsl-toolkit-podbox` persists on purpose, with a warm image
cache. Leave it.

⛔ `podman-machine-default` is not this project's. Measured at the end of this
session with `podman machine list`: **not running**, which is how it should be
left, and this session did not start it. ⚠ `./scripts/session-start.sh`'s probe
can start it, so check the state at the end rather than assuming.

⚠ **Job records from 2026-09-12 are still in the base**, including one whose
artifact pack failed and whose directory is therefore kept. Nothing needs them.
`wsl-toolkit --instance podbox gc --apply --older-than 24h` collects them from
2026-09-13.

⚠ **`eph-pgb` is not on this host.** `podman machine list` reports one machine
and that is the one above.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
Push straight to main. Create no branches.
```
