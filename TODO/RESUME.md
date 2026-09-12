## The task

Session of 2026-09-12, which started at `04:22:08Z`, four minutes after the
previous one ended. It closed the work order's item 1 and took the authoring
half of item 2.

## The resume point

[PROGRESS.md](PROGRESS.md)'s work order, item 1: [T-0702](interpose.md), the
placement half. ⭐ **[T-0215](image.md) is CLOSED**, so the work order has moved
up by one and the store lock race is no longer the first thing a session reads.

⭐ **The suite is deterministic again.** `cargo test --workspace` reads 0 of 30
in each of two passes, where it read 5 to 12 of 20 across twenty passes over two
days, and `experiments/153-store-lock-race.sh` exits 0 for the first time. ⛔ A red `workspace and interposer tests` is NOT T-0215 any more, and it is
not to be re-run and shrugged at: it is a new finding and it is investigated.

## In flight

Nothing is half-written and nothing is uncommitted.

## The state of the tree

Clean, on `main`. `./scripts/check-todo.py` is green and
`sh scripts/common/check-gate.sh --fast` is green on this host, all ten checks.
`sh scripts/windows/run-in-base.sh` ran the complete `dev.sh check` and it
exited 0.

⭐ **`main` accepts a direct push.** `enforce_admins` was turned off on
2026-09-11 and a push was verified on 2026-09-12.
[RULES.md](RULES.md) section 2 carries the rule and the fallback. ⛔ Create no
branches, and force-push is still refused.

⚠ **`experiments/153-store-lock-race.sh` has twelve clauses and four of them
MUTATE THE SOURCE.** Each restores the file however the script ends, the way
`scripts/plant.sh` works, but a killed run is worth a `git status` afterwards.
⛔ Read the conditions block before quoting any figure from it: every clause
measures a change, so the tree is modified while it runs and the report says so.

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

⛔ **Stopping a job from Windows does NOT stop it in the guest, and this session
paid for that.** A killed wrapper's container ran to completion nine minutes
later and kept writing to the log it had inherited, so a second job interleaved
with it and the conditions block of one run was read beside the tail of another.
⭐ Give every job its own log path and its own `PODBOX_ARTIFACTS`, read the
artifact rather than the console, and check
`wsl-toolkit --instance podbox resources` for a container still `Up` before
believing a job has ended. [`../docs/containers.md`](../docs/containers.md)
carries it.

⚠ **The artifact pack failed once more on 2026-09-12**, so a job record is kept
in the base. The scripts print the report to stdout before they copy, which is
how that run was recovered.

⛔ `podman-machine-default` is not this project's. Measured at the end of this
session with `podman machine list`: **stopped**, which is how it should be left,
and this session did not start it.

⚠ **`eph-pgb` is not a podman machine and it IS a registered WSL distribution**,
and the two readings look like a contradiction until the tool that took each is
named. `podman machine list` reports one machine and it is
`podman-machine-default`. `wsl-toolkit --instance podbox resources` lists
`eph-pgb` under what WSL has registered, stopped, and never touched. ⛔ Neither
is this session's to start or remove.

⚠ **Two job records from 2026-09-12 are kept in the base**, 2.4 GiB, both from
runs whose artifact pack failed. No container is running and no job record is
open. Nothing needs them, and
`wsl-toolkit --instance podbox gc --apply --older-than 24h` collects them from
2026-09-13.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
Push straight to main. Create no branches.
```
