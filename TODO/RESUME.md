# RESUME

⛔ **A dead man's switch, not a record.** This file is overwritten at the start
of every session and refreshed while the session runs. It carries what is in
flight right now and nothing else. [PROGRESS.md](PROGRESS.md) is the record and
the work order.

## The task

Session of 2026-09-12, which started at `02:35:53Z`. The work order's item 1,
[T-0215](image.md), and item 2, [T-0211](image.md).

## The resume point

[PROGRESS.md](PROGRESS.md)'s work order. ⭐ [T-0211](image.md) is `done` and
[T-0215](image.md) is still `open`, so the next item is
[T-0702](interpose.md) unless T-0215 is taken further first.

⛔ **T-0215 is P0 and it is NOT fixed.** What it now has is a measurement, an
instrument with a positive control, and two refutations. What it does not have
is the holder's name. `experiments/153-store-lock-race.sh` clause 6 is the fork
control and its skip list was short of the code twice; the entry's `Approach`
step 4 names the next measurement, which is a sampler in a second process.

## In flight

Nothing is half-written. Every file is committed and the gate is green.

## The state of the tree

Clean, on `main`, level with `origin/main`. `./scripts/check-todo.py` is green,
`sh scripts/common/check-gate.sh --fast` is green on this host, and the hosted
gate was green on all four checks at `17a022f`.

⭐ **`main` accepts a direct push.** `enforce_admins` was turned off on
2026-09-11 and a push was verified again on 2026-09-12. The four required
checks still run. [RULES.md](RULES.md) section 2 carries the rule and the
fallback. ⛔ Create no branches.

⚠ **One check is intermittent and it is not fixed.**
`cargo test --workspace` failed between 3 and 10 of 12 runs across seven passes
on 2026-09-11 and 2026-09-12, over six store lock tests.
[T-0215](image.md) carries the measurement. A red run of
`workspace and interposer tests` is that race until somebody proves otherwise,
and the operator's standing instruction is to re-run it. ⭐ The hosted gate
happened to be green at `17a022f`, which is luck rather than evidence.

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

⛔ `podman-machine-default` is not this project's. ⚠ Check its state at the end
of a session rather than assuming: `./scripts/session-start.sh`'s probe can
start it.

⚠ **`eph-pgb` is not on this host.** `podman machine list` reports one machine
and that is the one above.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
Push straight to main. Create no branches.
```
