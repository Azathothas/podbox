# Progress

## State

M0 through M5 are implemented. M6 is partial: the interposer builds for glibc
and musl and the shipped binary now EMBEDS both, but placement, the complete
entry-point set, ownership-memo policy and end-to-end acceptance remain open. M7 packaging has not started. ⭐ **M8 is the
nix acceptance**, [milestones.md](milestones.md) T-1111, and it drives the
shipped binary, so it is the last gate rather than an early one. The machine
tier, [podvm.md](podvm.md), is specified and not started.

131 entries: 41 open, 4 partial, 0 blocked, 86 done.

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

⭐ **The store lock race was MEASURED and it is still not fixed.**
[T-0215](image.md) asked which of two mechanisms makes the lock tests fail
intermittently and ordered the blast radius established first.
`experiments/153-store-lock-race.sh` is the instrument and its own header states
the shape it holds still; `experiments/results/store-lock-race.txt` is the run.

⛔ **The candidate that a misdirected `close` leaves a description open is
refuted by observation**, not by a rate: at every captured failure the process
held no description on that inode, and a second `flock` taken microseconds
later succeeded. ⭐ **The other candidate is where every measurement points**: a
run with none of the binary's four forking paths in it does not fail at all,
0 of 20 twice, so a concurrent fork is a necessary condition.

⛔ **THE FORK CONTROL WAS WRONG THREE TIMES, AND A SKIP LIST IS WHY.** It
started as two test names, gained `probe_cache::`, and was still short of the
code: `pull` forks through `probe_cache::resolve` once it is past the transport
policy, under a test name that says nothing about forking. ⭐ The third reading
was caught by a review pass reading the COMMAND LINE in the committed evidence
rather than the label above it. A skip list is a claim about the call graph and
it is checked against the call graph.

⭐ **The instrument ships with a positive control, and one of its two halves
does not.** `who_holds` reads `/proc/self/fd` and `/proc/locks` at a failing
assertion. `the_t_0215_instrument_sees_a_lock_that_is_held` holds a lock and
asserts the instrument sees it, so a report of "nobody" is an absence rather
than a blind probe. ⚠ The children half has no such control and says so: the
control would be a test that forks, which would put a fork into the very
control that must have none.

⭐ **Two more findings fell out of the same runs.** The failing set is **six**
tests and not four, and the two new ones are about the sweep rather than about
`in_use`. And every failure captured so far reads a lock as HELD when nothing
holds it, which is the safe direction; a wrong `false`, the one that would
delete a running container's blobs, has never been observed.

✅ **[T-0211](image.md) closed on a pass count rather than a green run.**
`experiments/157-lock-inheritance-prove.sh`: each test alone, 30 attempts, 30 of
30, and each of the two mutations reddens exactly one of them.

⭐ **[T-0702](interpose.md)'s embedding fork was ruled and half built.** A
fourth shape was taken, and it is not one of the three that entry listed:
`crates/podbox-cli/build.rs` COPIES what `scripts/build-interpose.sh` left
behind and writes an empty file where an object is absent. ⚠ The recommended
shape, a `build.rs` that RUNS the script, is a cargo inside a cargo;
`experiments/158-interpose-embedding.sh` measured that it completes here and the
shape was refused anyway, because it would not elsewhere. ⛔ The step order
moved with it: both `scripts/dev.sh` check and the gate workflow built the
binary BEFORE the objects, so the embedding would have carried two placeholders
and passed.

⭐ **Two decisions were taken and neither needed the operator.**
[T-1208](gate.md) rules one shape for a closure record, the bold `Done`
paragraph, and its own counts were wrong in three ways until the instrument
settled them. [T-1302](podvm.md) rules that `podvm` is BOTH a flag and an argv0
alias, with the flag as the primitive and the name changing only its default.

⚠ **Two Windows-lane traps were repaired in place.** A file that grows during
the workspace copy stops it at `archive/tar: write too long`, and a job could
not hand its evidence back out of a container that is removed when it exits.
[`../docs/containers.md`](../docs/containers.md) carries both.

## Current work order

1. [T-0215](image.md): **name the holder.** ⛔ Still P0 and still open. What is
   measured: several threads in one process ARE necessary, a concurrent fork IS
   necessary, the filesystem is not the cause, and a misdirected `close` is
   refuted. ⭐ The entry's `Approach` step 4b is the cheap decisive move that
   the fork result justifies: register the eight `Lock` sites that take no
   `sys::close_in_children` and re-run the subject. Applied, measured, and
   reverted if it changes nothing.
2. [T-0702](interpose.md): **the placement half.** The embedding is built and
   the entry is `partial`. What is left is this: write the selected object
   INSIDE the rootfs before the chroot, and set `LD_PRELOAD` to the path the
   payload will see. ⚠ It needs [T-0706](interpose.md)'s payload classification,
   which is `open` and effort S, so take that first or together.
   ⛔ **And T-0702's `Prove` names `alpine:latest`**, which resolves to a
   quota-bearing registry. `scripts/common/distro-matrix.sh` and T-0206 rule
   that the acceptance uses `public.ecr.aws` and the distributions' own; the
   `Prove` line needs correcting before it is run.
3. [T-0703](interpose.md): implement and verify the complete entry-point set and
   `*at` path-resolution rules.
4. [T-1110](milestones.md): run M6 acceptance across the libc matrix, including
   deliberate wrong-object selection and a named static-binary decline.
   [T-0712](interpose.md) is the rule its matrix has to obey.
5. [T-0710](interpose.md) and [T-0711](interpose.md): implement the two rulings
   the operator settled on 2026-09-11. Both are written into the entries.
6. [T-0805](cli.md) and [T-0808](cli.md): finish four-part diagnostics and drive
   every parity row through the shipped binary. [T-0809](cli.md) adds the row
   that makes an ambiguous spawn failure readable.
7. [T-0408](complete.md): run the zypper row and record it. It is one container
   run, and it is the only entry reopened for having no evidence at all.
8. [T-1207](gate.md) and [T-1208](gate.md): the excluded interposer crate's gate
   coverage, and the check that a closed entry carries its recorded run.
   ⭐ T-1208's shape is RULED now, so what is left is the check, its plant, and
   converting the four prose records that entry names.
9. [T-1108](milestones.md): package M7 only after M6 acceptance is green.
10. [T-1111](milestones.md): M8, the nix acceptance, after M7.
11. [podvm.md](podvm.md) T-1301 first, because every other entry there depends
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

⛔ **One measurement is worth re-taking before it is leaned on.**
`experiments/results/store-lock-race.txt` carries clause 6's figures, and the
command line printed above them is the authority on what they measured. Read it
before quoting the number.

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

⭐ **Both decisions this section carried are TAKEN**, on 2026-09-12, and each is
written into the entry that owns it rather than here: [T-1208](gate.md) rules
one shape for a closure record, and [T-1302](podvm.md) rules that `podvm` is
both a flag and an argv0 alias. What is left in each is implementation.

Two forks are open and defensible either way:

- ⛔ **[T-0215](image.md) step 4b is a measurement, not a fork**, but the fork
  underneath it is: whether the eight `Lock` sites that take no
  `close_in_children` registration are a product defect or only a test-shape
  one. ⚠ `Store::hold` is the only one of nine that registers, and the single
  capture that named a holder pointed at a staging lock, which is one of the
  eight. The entry says this is a lead and not a cause.
- ⚠ **[T-0702](interpose.md) needs its `Prove` corrected before it can be run.**
  It names `alpine:latest`, which resolves to a quota-bearing registry, and the
  acceptance rule is `public.ecr.aws` and the distributions' own. Correcting a
  `Prove` line is authoring, so do it under
  [`../docs/methodology/authoring.md`](../docs/methodology/authoring.md) and not
  in the same pass as the implementation.
