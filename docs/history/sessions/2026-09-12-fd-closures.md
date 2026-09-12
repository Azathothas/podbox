# Session summary, 2026-09-12: three fd closures, and the rate never moved

⚠ Saved beside the record because `docs/methodology/sessions.md` asks for the
table in chat AND on disk. [PROGRESS.md](../../../TODO/PROGRESS.md) is the
record; this is the one-screen version of what moved.

Ran `04:22:08Z` to `07:20Z`, four minutes after the previous session ended.

| what | before | after | taken by |
| --- | --- | --- | --- |
| entries open / partial / blocked / done | 41 / 4 / 0 / 86 | **42 / 4 / 0 / 86** | `scripts/check-todo.py` |
| ⭐ candidate mechanisms for the race | 1 refuted, 1 open | **4 closed, none moved the rate** | `experiments/153-store-lock-race.sh` |
| `Lock` sites that register for shedding | 1 of 9 | **9 of 9, from one funnel** | `crates/podbox-image/src/store.rs` |
| forks in this tree that drain the shed table | `clone_fork` only | **`clone_fork` and podbox's own libstd spawn** | `crates/podbox-probe/src/sys.rs` |
| the shed hook's own positive control | none | **passes, and goes red when the hook is gutted** | clause 10, exit 101 |
| how long a refusal lasts | unmeasured | **52 to 2827 us, one surviving 48 attempts** | `free_now`, clause 1 |
| a holder the kernel still lists | 1 capture, in a superseded run | **2, in the committed run** | the same |
| clauses in the instrument | 8 | **11** | the script |
| ⭐ clauses that mutate the SOURCE | 0 | **3, none reaching the tree** | clauses 8, 9 and 10 |
| the subject's rate, per 20 runs | 2 readings | **16 passes, 5 to 12** | clause 1 |
| ⭐ `Prove` lines pulling from Docker Hub | unknown | **39 of 132, in ten files** | counted over `TODO/*.md` |
| entries owning that defect | none | **[T-1209](../../../TODO/gate.md)** | the entry |
| Windows-lane traps recorded | 7 | **8** | [`containers.md`](../../containers.md) |

## The five findings worth carrying forward

1. ⛔ **CLOSING A MECHANISM IS NOT THE SAME AS REFUTING IT, AND THE ENTRY GRADES
   THEM.** Three ways for a fork to carry a lock fd away were closed and the
   rate did not move. Two of the three are graded as refutations, because each
   has a clause that takes the closure back out or amplifies it. The third,
   registering every `Lock`, has no such clause, so the entry says it is the
   subject before and after rather than an isolated control, and names the
   clause that would fix that.
2. ⛔ **A CHANGE KEPT WITHOUT A MEASUREMENT BEHIND IT SAYS SO IN THOSE WORDS.**
   Two closures are in the tree although neither moved the rate. Each removes a
   real way a fork can carry a lock away, which
   [T-0211](../../../TODO/image.md) forbids whether or not it is what T-0215 is.
   ⚠ Written any other way, a later reader finds a change in the history and
   concludes the race was fixed.
3. ⛔ **AN ABSENCE IS NOT A ZERO, AND A NULL RESULT FROM AN UNPROVEN HOOK IS
   WORTH NOTHING.** The new spawn hook has a positive control that carries its
   own negative leg, and clause 10 guts the hook and asserts the control goes
   red. It exits 101, so the control is watching the hook and not the air.
   ⭐ Without that, "the hook changed nothing" and "the hook never ran" are the
   same sentence.
4. ⛔ **EVIDENCE THAT NAMES WHAT WAS MUTATED AND NOT WHAT IT BECAME CANNOT BE
   CHECKED.** A review pass reading the COMMAND rather than the label found that
   a mutating clause printed the line it matched and not the line it wrote. It
   prints both now. ⚠ That is the same defect as a skip list short of the code,
   which cost this entry three wrong verdicts, in a different costume.
5. ⛔ **STOPPING A JOB FROM WINDOWS DOES NOT STOP IT IN THE GUEST.** A killed
   wrapper's container ran to completion nine minutes later and kept writing to
   the log it had inherited. A second job interleaved with it, and the
   conditions block of one run was read beside the tail of the other, which
   nearly put a figure from the wrong tree into the entry.

## What is still open, and it is the same P0

[T-0215](../../../TODO/image.md) is not closed and its `Problem` has not moved:
several threads in one process are necessary, a concurrent fork is necessary,
and nothing about an inherited descriptor explains it.

⭐ **What the session added is a lead the entry can act on in one line.** Across
the last two takings the `clone_fork` path alone produced 0 failures in 80 runs
and the `Command::spawn` path alone produced 2 and 3 of 20. The spawn that
survives in that clause is the test's own bare `/bin/sh -c 'echo ready; sleep 2'`,
which carries no shed hook on purpose. `Approach` step 4d gives that spawn the
hook as a mutation and re-runs the subject.
