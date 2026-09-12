# RESUME

⛔ **A dead man's switch, not a record.** This file is overwritten at the start
of every session and refreshed while the session runs. It carries what is in
flight right now and nothing else. [PROGRESS.md](PROGRESS.md) is the record and
the work order.

## The task

Session of 2026-09-12, which started at `04:22:08Z`, four minutes after the
previous one ended. It took the work order's item 1 in depth and the authoring
half of item 2.

## The resume point

[PROGRESS.md](PROGRESS.md)'s work order, item 1: [T-0215](image.md), which is
still `open` and still P0.

⛔ **Read the entry before the number, and read the CONDITIONS BLOCK before the
figures.** Every clause of `experiments/153-store-lock-race.sh` measures a
change, so the tree is modified while it runs, and the report now says so and
prints the diff. The command line above each clause's figures is still the
authority on what they measured, and a mutating clause now prints the line it
WROTE as well as the line it matched.

⛔ **THE NEXT MOVE IS NOT ANOTHER SHED, and the entry says why.** Every fork in
the process drains the shed table now and every lock is in it, and the failure
still arrives at 4 of 20 twice. A shed runs in the child, so the window between
the fork and the shed is the one thing it cannot close.

⭐ **Two readings are named, in the entry's `Approach` steps 4c and 4e.** 4c is
small: `free_now` reaches the two `in_use` tests alone, and both captures where
the kernel still listed a holder came from
`opening_a_store_sweeps_what_a_killed_process_left`, which has no duration line
at all. 4e is a SECOND process sampling `/proc/locks` through the whole run,
because the instrument in the failing thread arrives after the holder has gone.

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

⚠ **Job records from 2026-09-12 are in the base.** Nothing needs them.
`wsl-toolkit --instance podbox gc --apply --older-than 24h` collects them from
2026-09-13.

## The paste

```text
Read AGENTS.md and follow it. Run ./scripts/session-start.sh first.
The record is TODO/PROGRESS.md and it carries the work order.
Push straight to main. Create no branches.
```
