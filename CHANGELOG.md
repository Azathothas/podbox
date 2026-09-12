# Changelog

This file records repository-level changes to podbox. The current package
version describes the CLI and runtime behavior; unfinished milestone work stays
under Unreleased.

## Unreleased

### 2026-09-12T06:45:00Z: a defect found while correcting one Prove line turns out to be 39 of them

**Record:** [`TODO/PROGRESS.md`](TODO/PROGRESS.md). No version bump and no
deployment.

[`TODO/gate.md`](TODO/gate.md) T-1209 is authored, and it is not implemented in
the same pass. A `Prove` line IS the acceptance, because
[`TODO/RULES.md`](TODO/RULES.md) section 5 closes an entry on that command
actually run, so the rule that keeps the acceptance off a quota-bearing registry
applies to every one of them. ⛔ **39 of the 132 break it**, in ten of the
eighteen files: 35 name an unqualified reference and five name `docker.io/`
outright, and the two sets overlap by one line. `alpine:latest` alone appears 50
times.

⚠ **[T-0206](TODO/image.md) moved every `experiments/` script off Docker Hub on
2026-09-09 and nobody swept the `Prove` lines**, because that entry's table names
scripts and a `Prove` line is not a script.

⭐ **Two lines are corrected already and the entry says why they are the
exceptions.** [`TODO/interpose.md`](TODO/interpose.md) T-0702 and T-0706 are what
the work order sent this session to run, and a `Prove` that cannot run is not a
`Prove`. ⚠ T-0706's row 3, the Go payload, has no acceptance as a result: every
row of `DISTRO_ROWS_M5` is a base distribution and none is a Go image, and
inventing a reference is what T-1209 refuses. Its line drives the static row
instead, through podbox's own musl binary, and says so rather than pretending to
cover both.

### 2026-09-12T05:26:00Z: three fd mechanisms closed for the store lock race, and not one of them moved the rate

**Record:** [`TODO/PROGRESS.md`](TODO/PROGRESS.md). No version bump and no
deployment.

[`TODO/image.md`](TODO/image.md) T-0215 had one candidate family left, and every
member of it said the same thing: a forked child holds a copy of a lock fd and
the `flock` outlives its holder. Three ways for that to happen were closed, one
after another, and the subject stayed in its band of 2 to 11 failures per pass
every time.

⭐ **`Lock::try_acquire` registers every lock it builds.** It is the only place a
`Lock` is made, so no caller is asked to remember, and eight of the nine
construction sites were registering nothing at all until now: a staging lock, an
index lock and a container lock were all inheritable by a fork.

⭐ **`sys::shed_after_fork` drains the shed table in a child libstd forked.**
`clone_fork` sheds before it returns in the child; `Command::spawn` forks inside
libstd and never reaches that code, and `O_CLOEXEC` acts at the `execve` and not
at the `fork`. `run_payload` is the one place podbox spawns with libstd and it
goes through the hook now.

⛔ **Neither is a fix for T-0215 and neither is written up as one.** Each is kept
because [`TODO/image.md`](TODO/image.md) T-0211 forbids a fork carrying a lock
away, whether or not that is what T-0215 is. A change kept on an invariant rather
than on a measurement is recorded as exactly that.

⚠ **The third closure never enters the tree.** The fd exists before it is
registered, so a fork in between leaves a child with a lock fd in no table.
Clause 8 widens that window to 200 us as a source mutation and restores the file
however the script ends, the way `scripts/plant.sh` works. The rate does not
follow it.

⭐ **An absence is not a zero, so the hook has a positive control that carries
its own negative leg.** `a_spawn_through_the_hook_sheds_a_registered_fd_and_one_without_it_does_not`
asks the child itself whether the descriptor arrived, and runs the same spawn
with no hook first. Without that leg, a null result from a hook that never fired
would read exactly like a null result from a hook that did.

⭐ **The first positive fact this entry has had: a real holder exists.**
`free_now` retries at the point of refusal instead of after the assertion, and
reports how long the refusal lasted. The older instrument runs while an assertion
message is being formatted, by which time every short holder has gone, which is
why it kept answering "nobody".

⚠ **`experiments/153-store-lock-race.sh` gained four repairs and every one is
about a reading that was not what it claimed.** The subject is taken twice now,
because the moment a candidate fix is in the tree the subject becomes the claim.
Clauses 4 and 5 skipped one test name each and left two forking paths in both,
so neither isolated the path it named. The conditions block says when a reading
was taken on a modified tree, which every clause measuring a change is. And a
mutating clause prints the line it wrote as well as the line it matched: naming
what was mutated and not what it became cannot be checked by a reader, which is
the same defect as a label that outruns its command.

⭐ **It also gained clause 9 and clause 10.** Clause 9 takes the spawn hook back
out inside the script, so both legs of an A and B land in one file under one
conditions block. Clause 10 measures no rate at all: it guts the hook and
asserts the hook's own control goes RED, because a control nobody has seen fail
is not a control.

⛔ **Stopping a job from Windows does not stop it in the guest.** A killed
wrapper's container ran to completion nine minutes later and kept writing to the
log it had inherited, so a second job interleaved with it and the conditions
block of one run was read beside the tail of the other.
[`docs/containers.md`](docs/containers.md) carries it.

### 2026-09-12T03:58:37Z: the interposer objects are embedded, by copying rather than by a nested cargo

**Record:** [`TODO/PROGRESS.md`](TODO/PROGRESS.md). No version bump and no
deployment.

[`TODO/interpose.md`](TODO/interpose.md) T-0702's remaining fork was which shape
embeds the two objects, and the entry recommended a `build.rs` that runs
`scripts/build-interpose.sh`. That is a cargo inside a cargo, which can wait on
a lock its own parent holds.

⭐ **A fourth shape was taken and it is not one of the three the entry listed.**
`crates/podbox-cli/build.rs` copies what the script left behind and writes an
empty file where an object is absent, so `include_bytes!` always compiles and a
fresh clone with no zig still builds. ⚠ The hazard was measured rather than
assumed: `experiments/158-interpose-embedding.sh` ran a nested build in two
shapes and both completed, so it does not fire here. The shape is refused
anyway, because it would fire on somebody else's machine and copying cannot.

⛔ **An empty object must never read as a working interposer**, so
`podbox system info` now prints which objects the binary carries, including the
state where it carries neither.

⛔ **The step order had to move with the shape.** `scripts/dev.sh` check and the
gate workflow both built the binary BEFORE the objects, so this embedding would
have put two placeholders in it and both would have passed. The interposer step
runs first in each now.

⚠ **The placement half is untouched and the entry is `partial`.** The object
still has to be written inside the rootfs before the chroot, with `LD_PRELOAD`
set to the path the payload will see.

### 2026-09-12T03:44:30Z: T-0211 closes on a pass count, and the fork control was short of the code

**Record:** [`TODO/PROGRESS.md`](TODO/PROGRESS.md). No version bump and no
deployment.

[`TODO/image.md`](TODO/image.md) T-0211 was reopened for recording no run at
all, and its two tests live in the suite T-0215 measures as intermittent.
`experiments/157-lock-inheritance-prove.sh` is its closing measurement: each
test alone, **30 attempts, 30 of 30**, and each of the two mutations reddens
exactly one of them, which is what makes the fork defence and the exec defence
independent. The script asserts a mutation landed before it reads either test,
and it restores the file from a copy rather than with `git checkout --`.

⛔ **T-0215's fork control was short of the code three times, and completing it
reversed the answer.** `pull` calls `probe_cache::resolve` once it is past the
transport policy, so one `pull` test forks under a name that says nothing about
forking. With all four paths skipped the failure does not arrive at all, **0 of
20 in each of two passes**, so a concurrent fork IS necessary and the fork-shed
table is not refuted. ⚠ The three earlier readings were red because a fork was
still in the run.

⭐ **The filesystem is ruled out.** Clause 7 moves every lock from this
container's `overlayfs` to a `tmpfs` through `TMPDIR` and changes nothing else.
The failure arrives at 6 and 10 of 20, and again at 9 and 7 of 20. A negative
result, and it saves the next session the run.

⚠ **One capture names a holder and it is the session's best lead.** A staging
lock read as held with no descriptor in this process and a `/proc/locks` row
carrying this process's own pid, which is what a child holding an inherited
description looks like. `StagedFile::create` takes that lock and never registers
it for shedding; `Store::hold` is the only one of the nine `Lock` sites in the
tree that does. ⛔ Recorded as a lead, not as a cause.

### 2026-09-12T03:25:36Z: the store lock race is measured, and one candidate cause is refuted

**Record:** [`TODO/PROGRESS.md`](TODO/PROGRESS.md). No version bump and no
deployment.

[`TODO/image.md`](TODO/image.md) T-0215 asked which of two mechanisms makes four
store lock tests fail intermittently, and ordered the blast radius established
before anything was changed. `experiments/153-store-lock-race.sh` is the
measurement: one suite, one thing changed per clause, and every control taken
twice.

⛔ **One candidate mechanism is refuted and the other is where every
measurement points.** A misdirected `close` would leave the lock's own
description open: at every captured failure this process held no description on
that inode, and a second `flock` attempt microseconds later succeeded. ⭐ **With
all four of the binary's forking paths removed the failure does not arrive at
all**, 0 of 20 in two passes, so a concurrent fork is a necessary condition.
Several threads in one process is one too, green in five control passes, and the
process that holds an image lock is measured single-threaded.

⚠ **Two lessons about controls, both paid for in this session.** A control that
disagrees with itself rules nothing: at twelve runs the fork control read 3 and
then 0. And a control's skip list is a claim about the code: that list was short
three times, the last because `pull` forks through `probe_cache::resolve` once
it is past the transport policy, under a test name that says nothing about
forking. Every control is now taken twice, and the skip list is derived from the
call graph rather than from the names.

⭐ **The failing set is six tests and not four**, and the two new ones are about
the sweep rather than `in_use`. ⭐ **Every failure captured so far is in the safe
direction**: a lock reads as held when nothing holds it. A wrong `false`, which
is what would delete a running container's blobs, has never been observed.

⚠ **The instrument ships with a positive control**, because an instrument that
reports "nobody holds this" in every case is blind rather than right.
`the_t_0215_instrument_sees_a_lock_that_is_held` holds the lock and asserts the
instrument sees it.

`scripts/windows/run-in-base.sh` gained two repairs the measurement needed: the
live CodeGraph sidecars are excluded, because a file that grows during the
workspace copy stops it with `archive/tar: write too long`, and a job can hand
its evidence back through `/out`.

Two decisions were taken that needed no operator:
[`TODO/gate.md`](TODO/gate.md) T-1208 rules one shape for a closure record, and
[`TODO/podvm.md`](TODO/podvm.md) T-1302 rules that `podvm` is both a flag and an
argv0 alias, with the flag as the primitive.

### 2026-09-11T18:05:26Z: the references are mined, and two licence badges were wrong

**Record:** [`TODO/PROGRESS.md`](TODO/PROGRESS.md). No version bump and no
deployment.

Eleven repositories were mined with `scripts/common/mine-repo.sh`, each with its
tracker and its tree at a captured commit, each reporting zero gaps. Four had
been read at their URLs and kept out of the corpus; they are tracked now, which
is what [`docs/methodology/references.md`](docs/methodology/references.md)
section 4 requires. [`docs/history/2026-09-11-reference-sweep.md`](docs/history/2026-09-11-reference-sweep.md)
is the write-up and it opens with what the sweep did not establish.

⛔ **Two licence determinations were taken from a code host's badge and both
were wrong.** One tree reported `NOASSERTION` and carries the full Zero-Clause
BSD text, so it is vendorable. One reported `MIT` and declares
`GPL-2.0-or-later` in its manifest, so it is refused. A licence is read in the
tree.

⭐ **The `.gitignore` was excluding the evidence the sweep rests on.** Two
unanchored rules, `*.log` and `logs/`, matched every experiment log directory in
the corpus and kept **83 committed logs** out of the tree. A citation into one
resolved on this disk and would not have resolved in a fresh clone.
`scripts/check-todo.py` caught it.

⭐ **The TCG cost is a range, not a number.** Four measurements on one host
class span 3x to 21x across four workloads, and a document in the corpus states
one of them as the general figure. [`TODO/podvm.md`](TODO/podvm.md) T-1308 is the
entry, and podbox will never print a bare multiplier.

All 86 closed entries were reconciled against
[`TODO/RULES.md`](TODO/RULES.md) section 5.
`experiments/156-closure-records.sh` is the instrument. Two entries moved back:
one was closed with no run recorded at all, and one was closed on two tests that
[`TODO/image.md`](TODO/image.md) T-0215 has since measured as failing 5 of 12
runs.

Four licence questions were settled by the operator, and the nix acceptance is
now milestone M8 with its acceptance rows and its version pin.

### 2026-09-11T16:49:16Z: the machine tier is specified, and nothing is blocked

**Record:** [`TODO/PROGRESS.md`](TODO/PROGRESS.md). No version bump and no
deployment.

Four external repositories were studied and none is vendored.
[`TODO/reference-map.md`](TODO/reference-map.md) carries the licence
determination for each, and two of them carry no licence, so nothing may be
copied from them.

⭐ The finding that shapes the new work: the two restricted runtimes studied do
not have the same walls. One permits `clone` with every namespace flag and the
new mount API; the other refuses `unshare` outright and permits only `chroot`.
So neither set of walls may be compiled in, and the machine tier probes exactly
as the other tiers do. [`TODO/podvm.md`](TODO/podvm.md) is the new category.

Eleven entries were authored and none was implemented in the same pass. The
largest is [`TODO/image.md`](TODO/image.md) T-0215: `cargo test --workspace`
failed 5 of 12 runs, across four store lock tests, and a single green run had
been read as evidence for that suite.

Two entries that carried `blocked` no longer do. Neither met the definition:
nobody outside a session had to act for either to proceed, and each entry now
records what was wrongly inferred.

The four questions that waited on the operator are ruled and written into the
entries that own them.

### 2026-09-11T16:29:09Z: three host lanes, and the gate runs on all of them

**Record:** [`TODO/PROGRESS.md`](TODO/PROGRESS.md). No version bump and no
deployment.

podbox is now developed on a Linux host, in a hosted Linux container and on a
Windows host, and `scripts/session-start.sh` reads the machine and picks the
lane. The Windows lane runs every Linux step in a disposable container inside
the distribution `wsl-toolkit-podbox`, and `wsl.exe` is never called.

Fixed a defect that made the project gate unusable on Windows:
`scripts/check-todo.py` compared a host-separated relative path against
`git ls-files` output. The separators differ there, so, measured on
2026-09-11, **203 of the 248 links the check then resolved** were reported as
untracked and the gate could not be run on Windows at all.

Added the rule set the operator settled for this home: Simplified Technical
English for every word written anywhere, CodeGraph before `grep`, the
executable-bit repair a Windows checkout needs, and the two triggers that end a
session.

Took `sha2` to 0.11 and `ruzstd` to 0.9. The digest hex is now produced here
rather than through the hash crate's `LowerHex`, which 0.11 removed with the
move to `hybrid-array`.

Fenced `references/` off from every dependency updater. Four pull requests had
already been opened against the corpus, and a bump there moves a line under a
citation the gate asserts.

### 2026-09-11T06:01:52Z: migrated and hardened repository home

**Record:** [`docs/history/migration-2026-09-11.md`](docs/history/migration-2026-09-11.md).
No version bump and no deployment.

Migrated the source project into the current repository template, made the
root documentation and work state current, added security and architecture
maps, imported maintained cross-platform checks, pinned GitHub Actions, and
added dependency-update coverage. Fixed device completion so a denied `mknod`
cannot destroy an existing shim before replacement succeeds. The source and
template revisions, corpus identities, reviews, and validation commands are in
the migration record.
