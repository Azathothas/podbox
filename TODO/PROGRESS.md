# Progress

## State

M0 through M5 are implemented. M6 is partial: the interposer builds for glibc
and musl, but placement, the complete entry-point set, ownership-memo policy and
end-to-end acceptance remain open. M7 packaging has not started. ⭐ **M8 is the
nix acceptance**, [milestones.md](milestones.md) T-1111, and it drives the
shipped binary, so it is the last gate rather than an early one. The machine
tier, [podvm.md](podvm.md), is specified and not started.

131 entries: 42 open, 4 partial, 0 blocked, 85 done.

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
`cargo test --workspace` is not deterministic: measured on 2026-09-11,
**5 of 12 runs failed**, across four store lock tests. A single green run is not
evidence for that suite. [T-0215](image.md) carries the measurement and what has
to be established before anything is changed.

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
**1 m 19 s** on 2026-09-11.

⭐ **`main` takes a direct push now.** `enforce_admins` was turned off on
2026-09-11 and a direct push was verified. The four required checks still run
and still have to be green. [RULES.md](RULES.md) section 2 carries it, and a
publish branch is the fallback if protection is ever restored.

## What the last session did

⭐ **A reference sweep, done under
[`docs/methodology/references.md`](../docs/methodology/references.md) rather than
by reading at a URL.** Eleven repositories were mined with
`scripts/common/mine-repo.sh`, each with its tracker and its tree at a captured
commit, each reporting zero gaps. The write-up is
[`docs/history/2026-09-11-reference-sweep.md`](../docs/history/2026-09-11-reference-sweep.md)
and it opens with what the sweep did **not** establish.

⛔ **Two licence determinations had been taken from a code host's badge, and
both were wrong.** One tree reported `NOASSERTION` and carries the full
Zero-Clause BSD text, so it is vendorable. One reported `MIT` and declares
`GPL-2.0-or-later` in its manifest, so it is refused. A licence is read in the
tree. [reference-map.md](reference-map.md) carries all 41 determinations and all
41 verdicts.

⛔ **83 committed corpus logs were not in the tree, and the gate found it.**
The repair is in `.gitignore` and
[`../CHANGELOG.md`](../CHANGELOG.md) carries the mechanism.

Every closed entry was reconciled against [RULES.md](RULES.md) section 5, with
`experiments/156-closure-records.sh` as the instrument. Two entries moved back,
and both are named under **In progress** below.

Six entries were authored and none was implemented in the same pass: T-0414,
T-0415, T-0712, T-1208, T-1307 and T-1308. T-1111 became milestone M8.

## Current work order

1. [T-0215](image.md): establish whether the store lock race is reachable
   outside the test harness. ⛔ It is P0, it sits above M6 because `prune`
   asks `in_use` before it deletes blobs a running container needs, and
   [T-0211](image.md) now waits on it as well.
2. [T-0211](image.md): once T-0215 is understood, run its `Prove` **in a loop**
   and record the pass count out of the attempts.
3. [T-0702](interpose.md): place the correct per-libc interposer inside the
   rootfs and set `LD_PRELOAD` only when that rung is selected.
4. [T-0703](interpose.md): implement and verify the complete entry-point set and
   `*at` path-resolution rules.
5. [T-1110](milestones.md): run M6 acceptance across the libc matrix, including
   deliberate wrong-object selection and a named static-binary decline.
   [T-0712](interpose.md) is the rule its matrix has to obey.
6. [T-0710](interpose.md) and [T-0711](interpose.md): implement the two rulings
   the operator settled on 2026-09-11. Both are written into the entries.
7. [T-0805](cli.md) and [T-0808](cli.md): finish four-part diagnostics and drive
   every parity row through the shipped binary. [T-0809](cli.md) adds the row
   that makes an ambiguous spawn failure readable.
8. [T-0408](complete.md): run the zypper row and record it. It is one container
   run, and it is the only entry reopened for having no evidence at all.
9. [T-1207](gate.md) and [T-1208](gate.md): the excluded interposer crate's gate
   coverage, and the check that a closed entry carries its recorded run.
10. [T-1108](milestones.md): package M7 only after M6 acceptance is green.
11. [T-1111](milestones.md): M8, the nix acceptance, after M7.
12. [podvm.md](podvm.md) T-1301 first, because every other entry there depends
    on the probe.

## In progress

No implementation entry is half-written. Four entries are `partial`:

- [T-0503](enter.md), [T-0704](interpose.md) and [T-1109](milestones.md) carry
  their remaining conditions in their own files;
- ⭐ [T-0211](image.md) was moved from `done` on 2026-09-11 and the reason is a
  contradiction inside [image.md](image.md): it recorded no run, and **both
  tests its `Prove` names are in [T-0215](image.md)'s measured-intermittent
  set**. The implementation is not in doubt. The closing measurement is.

[T-0408](complete.md) was moved from `done` to `open` in the same pass, for the
simpler reason: it carried a `Prove` line and nothing after it.

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

## Two decisions the next session should take, and neither needs the operator

- [T-1302](podvm.md) asks whether `podvm` is a flag, an argv0 alias, or both.
  Both are defensible and the entry says so; make the call, record the rejected
  one, and continue. ⭐ The escape-hatch half of that entry **is** settled: the
  repeatable-flag shape, because a corpus tree ships the defect the
  split-on-whitespace shape invites.
- [T-1208](gate.md) asks which shape a closure record takes. 81 closed entries
  use a bold `Done` paragraph and 4 record the run as prose after `Prove`. Both
  satisfy [RULES.md](RULES.md) section 5 as written, which is the defect: the
  rule is not machine-checkable. Pick one, convert the four if the stricter one
  wins, and add the check with its plant.
