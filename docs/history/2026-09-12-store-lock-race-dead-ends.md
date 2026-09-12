# The store lock race: five dead ends, and the reading that ended them

⛔ **This is the superseded `Premise` of [T-0215](../../TODO/image.md), moved
here in its original words on 2026-09-12 when that entry closed.** The entry
keeps the mechanism, the fix and the guards, which is what a reader needs to use
the lock code correctly today. Everything below is what was believed on the way
there, and it is kept because
[`../methodology/history.md`](../methodology/history.md) says a dead end with
what it cost belongs somewhere a later session can find it.

⚠ **Nothing here is a live rule and nothing here is the work order.**
[`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md) is the record.

---

## What it cost, in one paragraph

⭐ **Three sessions, five closed mechanisms and roughly forty measured passes**,
and every one of the five was about WHICH descriptor a forked child holds. The
answer was that no child held anything: `Lock::drop` released by closing, and
closing is not releasing while anything else references the same open file
description. ⛔ **The blocker was the instrument, not the theory.** Every
reading was taken from the failing assertion, by which time the refusal was
over, so every reading said "nobody" and every reading was right.

⚠ **The one general lesson**, and it is why this page exists rather than a
summary: when a probe answers "nothing is there" over and over, the next thing
to doubt is where the probe is standing.

---

## The superseded Premise, verbatim

⭐ **The shape is measured and it points at concurrency, not at any
one test.** Same host, same image, same binary:

| how it was run | failures |
| --- | --- |
| the named test alone, single thread, 30 runs | 0 |
| the whole `podbox-image` lib suite, default threads, 15 runs | 0 |
| the whole lib suite, single thread, 15 runs | 0 |
| `cargo test --workspace`, 12 runs | ⛔ **5** |

⚠ **Only the workspace run reproduces it**, which is the only shape
that runs several crates' test binaries at once. So the trigger is
load or cross-binary timing, and not an ordering inside one suite.

⭐ **MEASURED BY `experiments/153-store-lock-race.sh`, WHICH HOLDS
EVERYTHING STILL EXCEPT ONE THING PER CLAUSE.** Same host, same
image, and `experiments/results/store-lock-race.txt` is the record.
⚠ Clauses 8 and 9 change the SOURCE rather than the test selection,
and they are read in the opposite direction from the others:
clause 8 makes a candidate window longer, and clause 9 takes a fix
back out.

| the clause | failures, per pass |
| --- | --- |
| 1. the suite as the gate runs it | 3, 10, 6, 4, 9, 3, 2, 4 of 12, and twenty passes of 20 over ten tree states on 2026-09-12: 5, 5, 6, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7, 8, 8, 9, 10, 11, 11, 12 |
| 2. one test thread per binary | ⭐ **0, 0, 0 of 12 and 0, 0 of 20** |
| 3. two of store.rs's forks removed | 4, 1, 5 of 12 and 10, 5, 2, 2 of 20 |
| 4. the ONLY fork left is `clone_fork` | 1, 1 then 0, 0 then 0, 0 then ⭐ **0, 0 of 20** |
| 5. the ONLY fork left is `Command::spawn` | 1, 0 then 1, 0 then 2, 3 then 0, 2 of 20 |
| 6. EVERY forking test removed | ⭐ **0, 0 in each of four takings of 20** |
| 7. every lock on `tmpfs` instead of `overlayfs` | 6, 10, 9, 7, 6, 8 of 20 |
| 8. the pre-registration window widened to 200 us | 3, 4 then 3, 4 then 1, 1 then 4, 3 of 20 |
| 9. the libstd spawn hook taken back out | 10, 11 then 7, 8 then 9, 10 of 20, against subjects of 10 and 12, 7 and 8, and 5 and 6 |
| 10. the hook gutted, and its own control asserted red | no rate: the control exits 101, so it is watching the hook |
| 11. the last bare spawn given the hook as well | 4, 4 then 6, 6 of 20, against subjects of 7 and 6, and 5 and 6 |

⛔ **THE SUBJECT'S OWN RATE IS UNSTABLE, and that governs how much
any control can carry.** Twenty-nine passes over two days read
between **2 and 10 of 12** and between **5 and 12 of 20**, and the
two denominators are kept apart because a range across both is a
rate with no denominator at all. ⚠ The tree was not the same for
all twenty of the 20-run passes, because each measured a candidate
closure; clauses 8, 9, 10 and 11 are the states that matter and
each has its own row, taken in the same run as the subject it is
read against. ⚠ **A control
taken once rules nothing here**, and clause 6 is the proof: at
twelve runs it read 3 and then 0, which is one pass refuting a
mechanism and the next supporting it. Every clause in the script is
now taken twice, THE SUBJECT INCLUDED, and a disagreement is
reported as ruling nothing. ⭐ The subject was taken once until
2026-09-12; the moment a candidate fix is in the tree the subject
becomes the claim, and a subject reading 0 once would say no more
than a control reading 0 once.

⛔ **THE FORK CONTROL WAS WRONG THREE TIMES, AND ITS SKIP LIST WAS
SHORT OF THE CODE EVERY TIME.** Clause 3 skips store.rs's own two
forking tests and was read as "no fork"; but `probe_cache` is a
module of `podbox-image`, so its tests fork in the same process.
Clause 6 then added `probe_cache::` and was still one short: `pull`
calls `probe_cache::resolve` once it is past the transport policy,
so `naming_the_registry_insecure_gets_past_the_policy` forks under
a name that says nothing about forking.
⚠ **A skip list is a claim about the code, and it is checked by
reading the code rather than the test names.** The four paths to a
fork in `podbox-image`'s test binary, as of 2026-09-12, are
`clone_fork` and `Command::spawn` in
`crates/podbox-image/src/store.rs`, and `podbox_probe::run` reached
through `probe_cache::measure` from the `probe_cache` tests and
from `crates/podbox-image/src/pull.rs`.
⭐ **With all four skipped the failure does not arrive at all, 0 of
20 in each of two passes, so A CONCURRENT FORK IS A NECESSARY
CONDITION** on this host. ⚠ Clause 3 keeps two of the four paths
and stays red at 2 and 2 of 20, which is what a necessary condition
looks like from the other side.

⛔ **AND NEITHER CLAUSE 4 NOR CLAUSE 5 RULES ANYTHING, WHICH IS
THE SCRIPT'S OWN VERDICT RATHER THAN A DISAPPOINTMENT.** Each keeps
exactly one forking path, and each has two passes that disagree:
clause 4 reads 1 and 1 of 20 in the first taking and 0 and 0 in
each of the next two; clause 5 reads 1 and 0 twice and then 2 and
3. ⚠ **The first taking of each disagrees with what followed**, so
neither is a verdict.
⭐ **BUT THE TWO HAVE SEPARATED, AND THAT IS THE LEAD THIS ENTRY
LEAVES.** Across the last two takings the `clone_fork` path alone
produced 0 failures in 80 runs, and the `Command::spawn` path alone
produced 2 and 3 of 20, which is the first taking of clause 5 whose
two passes agree. ⛔ **The surviving spawn in clause 5 is the
TEST'S BARE ONE**, `a_spawned_process_does_not_inherit_the_lock`'s
`/bin/sh -c 'echo ready; sleep 2'`, which carries no shed hook ON
PURPOSE because its own subject is `O_CLOEXEC`. Its child lives two
seconds, and `O_CLOEXEC` takes the inherited fds away only at that
child's `execve`, so for those two seconds it was the one fork in
the process that shed nothing.
⛔ **CLAUSE 11 GAVE IT THE HOOK AND THE FAILURE STAYED**, at 4 and
4 of 20 against a subject of 7 and 6, and at 6 and 6 against 5 and
6 in the next taking. Level with the subject and nowhere near zero.
⭐ **So every fork this process makes now drains the shed table,
every lock is in it, and the failure still arrives.** The
descriptor family is out of moves that a shed can make.
⭐ What the pair does support, read against clause 6's FOUR zeroes
and not against clause 1: one surviving fork of either kind brings
the failure back at least sometimes, so the mechanism is not
particular to one of the two ways this tree forks. ⚠ Both sit far
below the subject, and the reason is printed by the script rather
than left to a reader: removing three paths of four removes most of
the fork VOLUME as well.
⛔ Those two clauses skipped ONE name each until 2026-09-12, which
left two forking paths in both of them, so neither isolated
anything it claimed to. That is the skip-list defect again, in the
clause built to catch it.

⛔ **THREE CANDIDATE MECHANISMS WERE CLOSED ON 2026-09-12 AND NONE
OF THEM MOVED THE RATE. ALL THREE ARE ABOUT AN INHERITED
DESCRIPTOR.**
1. `crates/podbox-probe/src/sys.rs` holds `FORK_CLOSE`, a
   PROCESS-GLOBAL array of sixteen slots naming fds to shed in a
   forked child, and only `Store::hold` registered one: eight of
   the nine `Lock` construction sites took no slot at all.
   ⛔ **Closed, and the subject did not move.**
   `Lock::try_acquire` is the only place a `Lock` is built and it
   registers every one of them now. The subject stayed inside
   its 20-run band of 5 to 12, at 10 and 12 of 20 in the run of
   record.
   ⚠ **This one is weaker than the two below it**, and the entry
   grades it rather than levelling them: there is no clause that
   takes this closure back out the way clause 9 does for the next,
   so the reading is the subject before and after rather than an
   isolated control. ⭐ That clause is one more anchor in
   `mutate_and_measure` and it belongs in the next taking.
2. libstd forks inside `Command::spawn` and never passes through
   `clone_fork`, so the shed list could not reach that child at
   all, and `O_CLOEXEC` acts at the `execve` and not at the `fork`.
   ⛔ **Refuted by closing it.** `sys::shed_after_fork` installs a
   `pre_exec` hook that drains the table in the child, and
   `run_payload` is the one place podbox spawns with libstd. The
   subject did not move, and clause 9 takes the hook back out and
   reads 10 and 11 of 20 against a subject of 10 and 12 in the
   same run. ⚠ **The hook is PROVED to fire**, by
   `a_spawn_through_the_hook_sheds_a_registered_fd_and_one_without_it_does_not`,
   which asks the child itself and runs the same spawn with no hook
   as its own negative leg. An absence is not a zero, and this
   closure's null result would be worth nothing without it.
3. The fd exists before it is registered, so a fork between
   `Lock::open` and `sys::close_in_children` leaves a child holding
   a lock fd that is in no table and that nothing can shed.
   ⛔ **Refuted by amplifying it.** Clause 8 widens that window
   from a few instructions to 200 us and the rate does not
   follow: 3 and 4 of 20 against 11 and 6 in one run, and 3 and 4
   against 10 and 12 in the next. A window that is the mechanism
   gets worse when it is made longer.
4. One fork in the process shed nothing at all, on purpose: the
   bare `Command::spawn` in
   `a_spawned_process_does_not_inherit_the_lock`, whose own subject
   is whether `O_CLOEXEC` alone takes the fd away at the exec. Its
   child lives two seconds.
   ⛔ **Closed by clause 11 and the failure stayed**, at 4 and 4
   of 20 against a subject of 7 and 6, and at 6 and 6 against 5 and
   6. ⚠ The mutation lives in the script and never in the tree,
   because shedding in that child is exactly what that test must
   not do.
⛔ **And a fifth, from before this series: `Lock`'s `Drop` ignores
the result of `close`, so a descriptor closed twice would release
somebody else's file.** Refuted twice over, and neither refutation
is a rate. A `close` sent to the wrong descriptor would leave the
lock's own description OPEN: at every captured failure
`/proc/self/fd` showed no description on that inode, and a second
`flock` attempt taken microseconds later succeeded. A leaked
description would still be refusing.

⭐ **AND THE ANSWER IS UNDER ALL OF THEM: THE REFUSAL IS NOT A
CONFLICT AT ALL.** Every candidate above asks WHICH descriptor a
child holds. The measurement that ended the entry asks whether
anything holds one, and the `Done` record below carries it: at the
`EWOULDBLOCK` itself there is no row in `/proc/locks`, no
descriptor on the inode in any process on the host, and the SAME
descriptor succeeds on the next attempt a microsecond later.
⛔ `Lock::drop` released by closing, and closing is not releasing
while anything else references the same open file description.
