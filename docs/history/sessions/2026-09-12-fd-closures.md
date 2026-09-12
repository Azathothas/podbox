# Session summary, 2026-09-12: the store lock race, closed

⚠ Saved beside the record because `docs/methodology/sessions.md` asks for the
table in chat AND on disk. [PROGRESS.md](../../../TODO/PROGRESS.md) is the
record; this is the one-screen version of what moved.

Ran `04:22:08Z` to `07:40Z`, four minutes after the previous session ended.

| what | before | after | taken by |
| --- | --- | --- | --- |
| ⭐ [T-0215](../../../TODO/image.md), P0, open since 2026-09-11 | open | **done** | `experiments/153-store-lock-race.sh` |
| ⭐ the subject, per 20 runs | 5 to 12 across twenty passes | **0 of 20, twice** | clause 1 |
| the same suite with the fix deleted | - | **20 of 20** | clause 12 |
| entries open / partial / blocked / done | 41 / 4 / 0 / 86 | **41 / 4 / 0 / 87** | `scripts/check-todo.py` |
| candidate mechanisms closed with no effect | 1 | **5** | the entry |
| ⭐ what the refusal is | a holder, assumed | **the tail of an unfinished release** | the capture at the refusal |
| the guard on the defect | none | **deterministic, no fork and no timing** | the regression test |
| clauses in the instrument | 8 | **13**, five of them source mutations | the script |
| ⭐ `Prove` lines pulling from Docker Hub | unknown | **39 of 132, in ten files** | counted over `TODO/*.md` |
| entries owning that defect | none | **[T-1209](../../../TODO/gate.md)** | the entry |
| Windows-lane traps recorded | 7 | **8** | [`containers.md`](../../containers.md) |

## The mechanism, in one paragraph

⛔ **Closing a descriptor is not a release while anything else references the
same open file description.** `Lock::drop` released a lock by closing. A `fork`
makes that second reference, so after one the holder's own close no longer
completes the release: the record goes when the LAST reference goes, and where
that is a child it happens asynchronously with respect to this process's next
`flock`. ⭐ `flock(fd, LOCK_UN)` before the close removes the record in the
releasing thread, whatever the reference count is.

## The five findings worth carrying forward

1. ⛔ **THE INSTRUMENT WAS THE BLOCKER, NOT THE THEORY.** Three sessions of
   readings were taken from the failing assertion, and every one found the
   holder already gone: the committed captures show the refusal over on the next
   attempt, 1 to 4 us later. Moving the capture into the `EWOULDBLOCK` arm
   answered the question on the first run.
   ⭐ When every reading says "nobody", suspect where you are standing before
   you suspect the world. ⚠ And the capture is kept, behind
   `PODBOX_T0215_CAPTURE`, with clause 13 to re-take it: a figure a reader
   cannot re-take is a figure on trust.
2. ⛔ **AN INSTRUMENT THAT CAN FIND ITSELF WILL.** The first capture at the
   refusal named the holder as this process, fd 7, which was the probe's own
   descriptor, opened microseconds earlier by the very call being measured. It
   is excluded by number now.
3. ⛔ **CLOSING A MECHANISM IS NOT REFUTING IT, AND FIVE CLOSURES MOVED
   NOTHING.** Every lock registered for shedding, podbox's own libstd spawn
   drained the table, the pre-registration window was widened, and the one spawn
   that was bare by design was hooked. The rate never moved, because the defect
   was not about which descriptors a child holds. ⭐ Two of the five are kept,
   on [T-0211](../../../TODO/image.md)'s invariant and NOT on this measurement,
   and the entry says so in those words.
4. ⛔ **A DETERMINISTIC GUARD BEATS A RATE.** `F_DUPFD_CLOEXEC` gives a second
   reference to one open file description, which is exactly what a fork gives a
   child, so the regression test needs no second process, no thread and no
   timing. The defect was one run in two; its guard is one run.
5. ⛔ **STOPPING A JOB FROM WINDOWS DOES NOT STOP IT IN THE GUEST.** A killed
   wrapper's container ran to completion nine minutes later and kept writing to
   the log it had inherited, so a second job interleaved with it and the
   conditions block of one run was read beside the tail of the other.

## What this does not claim

⚠ **The blast radius is unchanged and the product could not reach this.**
Several threads in one process are a necessary condition, clause 2 at 0 of 12
three times and 0 of 20 twice, and the process that holds an image lock for a
container's life is measured single-threaded. ⭐ The fix is in the product
anyway: a `prune` in a future threaded caller would have hit it, and every
observed failure was in the safe direction by luck rather than by design.
