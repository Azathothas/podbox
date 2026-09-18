# Progress

## State

M0 through M5 are implemented. M6 is partial: the interposer builds for glibc
and musl, the shipped binary EMBEDS both, and since 2026-09-18 it PLACES the
selected object inside the rootfs and CLASSIFIES the payload with a named
decline. The complete entry-point set, the ownership-memo move to the host,
the identity measurement, and end-to-end acceptance remain open. M7 packaging has not started. ⭐ **M8 is the
nix acceptance**, [milestones.md](milestones.md) T-1111, and it drives the
shipped binary, so it is the last gate rather than an early one. The machine
tier, [podvm.md](podvm.md), is specified and not started.

132 entries: 40 open, 3 partial, 0 blocked, 89 done.

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

⭐ **THAT GAP IS CLOSED. `cargo test --workspace` is deterministic again**, as
of 2026-09-12. It read between 2 and 10 of 12 and between 5 and 12 of 20 across
twenty-two passes over two days, and it reads **0 of 30 in each of two passes**
now. [T-0215](image.md) carries the mechanism, the captures that named it, the
one-syscall fix and the clause that reddens the fix on demand.
⛔ **`close(2)` is not a release while anything else references the same open
file description**, and a `fork` makes exactly that.

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

Session of 2026-09-18. It closed the work order's item 1: [T-0702](interpose.md)
placement with [T-0706](interpose.md) classification, both `done`. The
[T-0215](image.md) record it replaced lives in the entries and in
[`docs/history/sessions/`](../docs/history/sessions/).

⭐ **The tier now reaches a dynamic payload and declines the rest by name.**
`crates/podbox-cli/src/interpose.rs` classifies the payload ELF once
(`PT_INTERP`, Go markers, architecture, T-0709's version assertion through
the decline channel), writes the selected object to `/.podbox/interpose.so`
inside the rootfs before the chroot, and sets `LD_PRELOAD` with podbox's
object first. `run`, `exec` on both paths, and `create` share the one call.
A decline never fails the run. `experiments/159-interpose-placement.sh`
carries the acceptance and `experiments/results/interpose-placement.txt`
the run: placed and preloaded for `sh`, declined naming the static payload
with exit 0, Go markers proved by unit test with no image invented.

⭐ **The first shape declined every dynamic payload, and the mechanism is
the entry's own subject.** Alpine's `/bin/sh` is an absolute link to
`/bin/busybox`, which names the host's file when read from outside and the
image's from inside. The resolver now walks the guest path the way the
guest kernel would. Three entry `Prove` lines were unrunnable as written
and are amended with the reason in the entries: `-v` is refused so the
static binary stages by `extract` plus copy plus `run`, and the chroot has
no `/proc` so the environ is read through `env`.

⚠ **One red run that is not this change.** `cargo test --workspace` failed
three `podbox-image` lock-table tests with "already holds 16 locks" on a
tree whose new tests hold no store lock, then passed clean on re-run of the
same tree. That is the flaky class [T-1204](gate.md)'s neighbourhood owns:
one pass is not evidence for a racy suite. It is recorded here and not
acted on.

## Current work order

1. [T-0703](interpose.md): implement and verify the complete entry-point set and
   `*at` path-resolution rules.
2. [T-1110](milestones.md): run M6 acceptance across the libc matrix, including
   deliberate wrong-object selection and a named static-binary decline.
   [T-0712](interpose.md) is the rule its matrix has to obey.
3. [T-0710](interpose.md) and [T-0711](interpose.md): implement the two rulings
   the operator settled on 2026-09-11. Both are written into the entries.
4. [T-1209](gate.md): the 37 remaining `Prove` lines that pull from Docker Hub,
   the mapping that decides each replacement, and the check and plant that stop
   the next one. ⚠ Take the mapping and the sweep before the check: the check
   goes green only once nothing violates it.
5. [T-0805](cli.md) and [T-0808](cli.md): finish four-part diagnostics and drive
   every parity row through the shipped binary. [T-0809](cli.md) adds the row
   that makes an ambiguous spawn failure readable.
6. [T-0408](complete.md): run the zypper row and record it. It is one container
   run, and it is the only entry reopened for having no evidence at all.
7. [T-1207](gate.md) and [T-1208](gate.md): the excluded interposer crate's gate
   coverage, and the check that a closed entry carries its recorded run.
   ⭐ T-1208's shape is RULED now, so what is left is the check, its plant, and
   converting the four prose records that entry names.
8. [T-1108](milestones.md): package M7 only after M6 acceptance is green.
9. [T-1111](milestones.md): M8, the nix acceptance, after M7.
10. [podvm.md](podvm.md) T-1301 first, because every other entry there depends
    on the probe. ⭐ T-1302's shape is ruled, so its implementation is a flag,
    a third `ALIASES` entry and the collision rule.

## In progress

No implementation entry is half-written and nothing is uncommitted. Three entries
are `partial`: [T-0503](enter.md), [T-0704](interpose.md) and
[T-1109](milestones.md), each carrying its remaining conditions in its own file.

[T-0408](complete.md) is `open` rather than `done` for the simpler reason that
reopened it: it carried a `Prove` line and nothing after it.

⛔ **Read the CONDITIONS BLOCK of a reading before quoting its figures.**
`experiments/results/store-lock-race.txt` prints whether the tree was modified
and what differed from the commit, because every clause in that script measures
a change and the commit alone names a state that was not run. The command line
above each clause's figures is still the authority on what they measured, and a
mutating clause prints the line it WROTE as well as the line it matched.

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

⭐ **The fork this section carried is settled by measurement.** It asked whether
the eight unregistered `Lock` sites were a product defect or a test-shape one.
They are registered, the rate did not move, and [T-0215](image.md) records the
change as kept on [T-0211](image.md)'s invariant. The race itself is closed and
was neither of those things.

⭐ **The Go row is settled the second way.** [T-0706](interpose.md)'s row 3 is
proved by unit test over the four Go section names, with no image invented,
and the entry records that as the answer. What is left of
[T-1209](gate.md)'s mapping question is the general rule for future rows,
not this case.

One is open and it needs no operator:

- ⚠ **Whether the gate should report a rate rather than a pass or a fail.**
  T-0215 is closed, so nothing is red today, but CI reported green for a suite
  that failed two runs in five and a single run is still not evidence for a racy
  one. That is [T-1204](gate.md)'s neighbourhood and it needs a ruling before
  the next intermittent check arrives. This session added one more reading for
  it: three `podbox-image` lock-table tests failed once with "already holds 16
  locks" and passed on re-run of the same tree.
