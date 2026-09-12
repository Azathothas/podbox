# Progress

## State

M0 through M5 are implemented. M6 is partial: the interposer builds for glibc
and musl and the shipped binary now EMBEDS both, but placement, the complete
entry-point set, ownership-memo policy and end-to-end acceptance remain open. M7 packaging has not started. ⭐ **M8 is the
nix acceptance**, [milestones.md](milestones.md) T-1111, and it drives the
shipped binary, so it is the last gate rather than an early one. The machine
tier, [podvm.md](podvm.md), is specified and not started.

132 entries: 42 open, 4 partial, 0 blocked, 86 done.

## Baseline

The source tree was captured at `ea5b671b`.
Its hosted gate was green. In an isolated WSL environment using the exact
`rust:1.98.1-bookworm` image, bootstrap, formatting, workspace clippy, the
x86_64-musl release build, TODO consistency, and marker checks passed.

The workspace test run found one real portability defect: when `mknod` was
denied, device completion removed an existing shim before recreating it. The
replacement now creates a sibling candidate and renames it atomically only
after successful device creation. The focused regression test passes in the
same rootless environment. The complete migrated-tree validation is recorded
in [`docs/history/migration-2026-09-11.md`](../docs/history/migration-2026-09-11.md).

⛔ **That baseline is known to be incomplete in one way that matters.**
`cargo test --workspace` is not deterministic, and the rate itself is unstable:
passes of 12 runs over 2026-09-11 and 2026-09-12 have read anywhere from **2 to
10 failures**, across **six** store lock tests. A single green run is not
evidence for that suite, and neither is a single control.
[T-0215](image.md) carries the series, the mechanism it refutes, the one it
leaves open, and what has to be established before anything is changed.
⭐ **Every failure captured so far reads a lock as HELD when nothing holds it**,
which is the safe direction. A wrong `false`, the one that would delete a
running container's blobs, has not been observed at all.

The raw source-era record, including all measurements and resolved questions,
is preserved at
[`docs/history/source-progress-ea5b671.md`](../docs/history/source-progress-ea5b671.md).

## Where the work runs

⭐ **Three host lanes, and `./scripts/session-start.sh` picks one.** A Linux
host and a container run everything directly. A Windows host runs the record and
document checks on the host, and every Linux step in a disposable container
inside the distribution `wsl-toolkit-podbox`.
[`docs/containers.md`](../docs/containers.md) holds the procedure, the
exclusions and the traps each lane has.

⛔ **`wsl.exe` is never called.** `sh scripts/windows/run-in-base.sh` is the
Windows half of `./scripts/dev.sh check`, and it ran the complete check in
**1 m 19 s** on 2026-09-11 and **1 m 28 s** on 2026-09-12, both warm.
⭐ **A job hands its evidence back through `/out`** now, named by
`PODBOX_ARTIFACTS`, because the container is removed when it exits. ⚠ The pack
step failed once on 2026-09-12 and the report was recovered from the job's own
stdout, which the scripts print before they copy.

⭐ **`main` takes a direct push now.** `enforce_admins` was turned off on
2026-09-11 and a direct push was verified. The four required checks still run
and still have to be green. [RULES.md](RULES.md) section 2 carries it, and a
publish branch is the fallback if protection is ever restored.

## What the last session did

⛔ **THREE WAYS FOR A FORK TO CARRY A LOCK AWAY WERE CLOSED, AND THE STORE LOCK
RACE DID NOT MOVE.** [T-0215](image.md) had one candidate family left and its
`Premise` names each member and what took it away. The subject stayed inside
its 20-run band of 5 to 12 through all three closures, and
`experiments/results/store-lock-race.txt` carries every clause in one run.

⭐ **The two closures that are KEPT are kept on an invariant, not on a
measurement, and the entry says so in those words.** `Lock::try_acquire`
registers every lock it builds, where eight of the nine construction sites
registered nothing before; and `sys::shed_after_fork` drains the shed table in a
child libstd forked, which `clone_fork`'s own shed could never reach. Each closes
a way a fork can carry a lock away, which [T-0211](image.md) forbids whether or
not it is what T-0215 is.

⛔ **The third closure is a source MUTATION and it never enters the tree.**
Clause 8 widens the window between `Lock::open` and the registration to 200 us
and restores the file however the script ends, the way `scripts/plant.sh` works.
Clause 9 does the same to take the second closure back out, so both legs of an
A and B land in one evidence file under one conditions block.

⭐ **An absence is not a zero, so the new hook has a positive control that
carries its own negative leg.**
`a_spawn_through_the_hook_sheds_a_registered_fd_and_one_without_it_does_not` asks
the child whether the descriptor arrived, and runs the same spawn with no hook
first. Without that leg a null result from a hook that never fired would read
exactly like one from a hook that did.

⭐ **The first positive fact this entry has had: a real holder exists.**
`free_now` retries at the point of refusal rather than after the assertion, and
the refusals last 17 to 4163 us, several of them surviving 11 to 31 consecutive
`flock` calls. Twice the kernel still listed the lock at the assertion, both
times on a `*.partial` staging lock and both times attributed to the test process
while that process held no descriptor on the inode.

⚠ **`experiments/153-store-lock-race.sh` was repaired in several places and
every repair is about a reading that was not what it claimed.**
[CHANGELOG.md](../CHANGELOG.md) carries what each one was. ⛔ **What matters to the work order is that clauses 4
and 5 still rule nothing even repaired**: clause 4 read 1 and 1 of 20 in one
taking and 0 and 0 in the next, so the two takings disagree and the entry quotes
neither as a verdict.

⭐ **[T-1209](gate.md) was authored from a defect found while correcting one
line.** 39 of the 132 `Prove` lines in `TODO/` pull from Docker Hub, which is the
one registry the acceptance may not use. [T-0206](image.md) moved every
`experiments/` script off the Hub in 2026-09-09 and nobody swept the `Prove`
lines, because that entry's table names scripts and a `Prove` line is not a
script.

⚠ **[T-0702](interpose.md) and [T-0706](interpose.md) had theirs corrected**,
because the work order sent this session to run them. T-0706's row 3, the Go
payload, has no acceptance now: no row of `DISTRO_ROWS_M5` ships Go, and
inventing a reference is what T-1209 refuses.

⛔ **A Windows-lane trap cost this session a wrong reading and it is recorded.**
Stopping a job from Windows does not stop it in the guest: a killed wrapper's
container ran to completion nine minutes later and kept writing to the log it had
inherited, so a second job interleaved with it and the conditions block of one
run was read beside the tail of the other.
[`../docs/containers.md`](../docs/containers.md) carries it.

## Current work order

1. [T-0215](image.md): **name the holder.** ⛔ Still P0 and still open. What is
   measured: several threads in one process ARE necessary, a concurrent fork IS
   necessary, the filesystem is not the cause, and every mechanism about an
   inherited descriptor has now been closed without the rate moving. ⭐ The
   entry's `Approach` step 4c is the next reading and the two kernel captures
   are what make it cheap: `free_now` already loops at the point of refusal, so
   it reads `/proc/locks` and every live child's `/proc/<pid>/fd` on EACH
   attempt and names the child that has the descriptor.
2. [T-0702](interpose.md): **the placement half.** The embedding is built and
   the entry is `partial`. What is left is this: write the selected object
   INSIDE the rootfs before the chroot, and set `LD_PRELOAD` to the path the
   payload will see. ⚠ It needs [T-0706](interpose.md)'s payload classification,
   which is `open` and effort S, so take that first or together. ⭐ Both `Prove`
   lines are corrected now and both can be run.
3. [T-0703](interpose.md): implement and verify the complete entry-point set and
   `*at` path-resolution rules.
4. [T-1110](milestones.md): run M6 acceptance across the libc matrix, including
   deliberate wrong-object selection and a named static-binary decline.
   [T-0712](interpose.md) is the rule its matrix has to obey.
5. [T-0710](interpose.md) and [T-0711](interpose.md): implement the two rulings
   the operator settled on 2026-09-11. Both are written into the entries.
6. [T-1209](gate.md): the 37 remaining `Prove` lines that pull from Docker Hub,
   the mapping that decides each replacement, and the check and plant that stop
   the next one. ⚠ Take the mapping and the sweep before the check: the check
   goes green only once nothing violates it.
7. [T-0805](cli.md) and [T-0808](cli.md): finish four-part diagnostics and drive
   every parity row through the shipped binary. [T-0809](cli.md) adds the row
   that makes an ambiguous spawn failure readable.
8. [T-0408](complete.md): run the zypper row and record it. It is one container
   run, and it is the only entry reopened for having no evidence at all.
9. [T-1207](gate.md) and [T-1208](gate.md): the excluded interposer crate's gate
   coverage, and the check that a closed entry carries its recorded run.
   ⭐ T-1208's shape is RULED now, so what is left is the check, its plant, and
   converting the four prose records that entry names.
10. [T-1108](milestones.md): package M7 only after M6 acceptance is green.
11. [T-1111](milestones.md): M8, the nix acceptance, after M7.
12. [podvm.md](podvm.md) T-1301 first, because every other entry there depends
    on the probe. ⭐ T-1302's shape is ruled, so its implementation is a flag,
    a third `ALIASES` entry and the collision rule.

## In progress

No implementation entry is half-written and nothing is uncommitted. Four entries
are `partial`:

- [T-0503](enter.md), [T-0704](interpose.md) and [T-1109](milestones.md) carry
  their remaining conditions in their own files;
- ⭐ [T-0702](interpose.md) is `partial` as of 2026-09-12. The embedding half is
  built, tested in both the bare-tree and the built state, and ruled in the
  entry. The PLACEMENT half is untouched, and
  `crates/podbox-cli/src/interpose.rs` says so in its own header so a reader of
  the code cannot mistake one for the other.

[T-0408](complete.md) is `open` rather than `done` for the simpler reason that
reopened it: it carried a `Prove` line and nothing after it.

⛔ **Read the CONDITIONS BLOCK of a reading before quoting its figures.**
`experiments/results/store-lock-race.txt` now prints whether the tree was
modified and what differed from the commit, because every clause in that script
measures a change and the commit alone names a state that was not run. The
command line above each clause's figures is still the authority on what they
measured.

## Operator questions

⭐ **None is open.** Every ruling is written into the entry that owns it, which
is where an implementer reads it.

| question | ruled on 2026-09-11 | where it lives |
| --- | --- | --- |
| where the ownership memo lives | on the host, beside the container record | [T-0710](interpose.md) |
| what `setuid` does on a uid 0 payload | honest failure by default, and a flag turns on the lie | [T-0711](interpose.md) |
| whether host CA injection stays on | yes, unchanged: inject on an announcement, mark degraded, `--strict` refuses | [T-0407](complete.md) |
| what to do with a corpus dependency bump | close it, and fence `references/` off from every updater | `.github/dependabot.yml` |
| the two `memfd-exec` trees, which declare MIT in a manifest and ship no licence file | do not vendor either; the operator maintains a 0BSD crate that does the job | [reference-map.md](reference-map.md), [T-0909](deps.md) |
| `dockless`, which states no licence at all | keep the tree, study it, copy nothing, re-implement where useful | [reference-map.md](reference-map.md) |
| `VHSgunzo/userland-execve`, which does not exist | keep the row as a corrected citation, not a deletion | [reference-map.md](reference-map.md) |
| whether an unlicensed research tree may be tracked here | yes, track the whole tree; the corpus rule wins and nothing may be copied from it | [reference-map.md](reference-map.md) |

⚠ **`/dev/ptmx` on the target is a measurement, not a ruling**, and it belongs
to [T-0503](enter.md). [T-0414](complete.md) is the probe leg for it, and that
entry also carries the second denial only one instance of the class has shown:
`readdir("/")` answering `EACCES`.

⛔ **Nothing is blocked.**

## What the next session should decide, and neither needs the operator

⭐ **The fork this section carried is SETTLED by measurement rather than by a
decision.** It asked whether the eight `Lock` sites that took no
`close_in_children` registration were a product defect or only a test-shape one.
They are registered now, the rate did not move, and the entry records the change
as kept on [T-0211](image.md)'s invariant rather than on this measurement. There
was nothing to decide once it was measured.

Two are open and each is defensible either way:

- ⛔ **[T-0215](image.md) has run out of candidates about an inherited
  descriptor, and the fork under it is what to do about the gate meanwhile.**
  Three closures moved nothing, a fork and several threads are both still
  necessary, and the suite is red about one run in two. ⚠ The tempting fix is to
  serialise the lock tests, which the entry refuses because it answers nothing.
  What is NOT refused and is not yet decided is whether the gate should report
  the rate instead of a pass or a fail, so a racy check stops reading as a green
  one. That is [T-1204](gate.md)'s neighbourhood and it needs a ruling.
- ⚠ **[T-1209](gate.md) decides how a `Prove` line names an image, and the Go
  row is the case that has no answer yet.** The mapping the entry states is that
  every reference is a row of `DISTRO_ROWS_M5`, named by its fully qualified
  reference. No row ships Go, so [T-0706](interpose.md)'s row 3 has no
  acceptance. Either a row is added for a Go image, or the classification's Go
  case is proved some other way. ⛔ Inventing a reference is refused.
