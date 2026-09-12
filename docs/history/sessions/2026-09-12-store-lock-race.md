# Session summary, 2026-09-12: the store lock race, measured rather than fixed

⚠ Saved beside the record because `docs/methodology/sessions.md` asks for the
table in chat AND on disk. [PROGRESS.md](../../../TODO/PROGRESS.md) is the
record; this is the one-screen version of what moved.

Ran `02:35:53Z` to `04:15Z`. The operator called the end-of-session protocol.

| what | before | after | taken by |
| --- | --- | --- | --- |
| entries done | 85 | **86** | `scripts/check-todo.py` |
| entries open / partial / blocked | 42 / 4 / 0 | **41 / 4 / 0** | the same |
| ⭐ store lock tests known to fail | 4 | **6** | `experiments/153-store-lock-race.sh` |
| candidate mechanisms for the race | 2, neither measured | **1 refuted, 1 open** | the same, and the instrument |
| the suite's failure rate, per 12 runs | one reading, 5 | **2 to 10 across seven passes** | the same |
| a wrong `false` from `in_use`, ever observed | unknown | **never** | the same |
| threads in one process, as a condition | unmeasured | **necessary**, 0 failures in five control passes | the same, clause 2 |
| the filesystem under the locks | unconsidered | **ruled out** | the same, clause 7 |
| T-0211's closing record | none at all | **30 of 30 per test, both mutations caught** | `experiments/157-lock-inheritance-prove.sh` |
| interposer objects in the shipped binary | 0 | **2, or 2 named placeholders** | `crates/podbox-cli/build.rs` |
| a nested cargo from a `build.rs` | assumed to deadlock | **completes here, and refused anyway** | `experiments/158-interpose-embedding.sh` |
| experiments in the tree | 34 | **37** | `experiments/README.md` |
| decisions the next session had to take | 2 | **0** | T-1208 and T-1302 |
| Windows-lane traps recorded | 5 | **7** | [`containers.md`](../../containers.md) |

## The five findings worth carrying forward

1. ⛔ **A CONTROL'S SKIP LIST IS A CLAIM ABOUT THE CODE, and this session's was
   wrong three times.** It began as two test names, gained `probe_cache::`, and
   was still short: `pull` forks through `probe_cache::resolve` once it is past
   the transport policy, under a test name that says nothing about forking.
   ⭐ The third error was caught by a review pass that read the COMMAND LINE in
   the committed evidence instead of the label above it.
2. ⛔ **A control run once is a coincidence.** The same clause read 3 of 12 and
   then 0 of 12: one pass refuting a mechanism and the next supporting it.
   `experiments/153-store-lock-race.sh` now takes every control twice and
   reports a disagreement as ruling nothing.
3. ⭐ **AN INSTRUMENT THAT ANSWERS "NOBODY" IN EVERY CASE IS BLIND, NOT RIGHT.**
   Ten captured failures all reported no holder, including one where a holder
   was expected. A positive control settled it: with a lock held the same
   instrument prints the descriptor and the kernel's own `FLOCK` row, so the
   absences are real.
4. ⛔ **AN EXPERIMENT CAN MEASURE ITS OWN BUG AND REPORT IT AS A FINDING.**
   `158`'s first run referred to `$3` after a `shift`, so its fixture never
   compiled and the clause reported "no shape completed", which would have
   refuted a mechanism for no reason. It carries a guard now that asserts the
   fixture was written whole.
5. ⚠ **`Store::hold` is the only one of nine `Lock` sites that registers its fd
   for shedding.** The single capture that named a holder pointed at a staging
   lock, which is one of the eight that do not. Recorded as a lead, not a cause.

## What is still red

⛔ **`cargo test --workspace` is still intermittent and T-0215 is still P0 and
open.** What it now has: the series, a refuted mechanism, an instrument with a
positive control, and the filesystem ruled out. What it does not have: the
holder's name. The entry's `Approach` step 4b names the measurement that would
give it, a sampler in a second process, because the holder is gone before the
failing thread can look.
