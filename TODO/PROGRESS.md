# Progress

## State

M0 through M5 are implemented. M6 is partial: the interposer builds for glibc
and musl, but placement, the complete entry-point set, ownership-memo policy and
end-to-end acceptance remain open. M7 packaging has not started. The machine
tier, [podvm.md](podvm.md), is specified and not started.

125 entries: 35 open, 3 partial, 0 blocked, 87 done.

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

⛔ **That baseline is now known to be incomplete in one way that matters.**
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

## Current work order

1. [T-0215](image.md): establish whether the store lock race is reachable
   outside the test harness. ⛔ It is P0 and it sits above M6 because `prune`
   asks `in_use` before it deletes blobs a running container needs.
2. [T-0702](interpose.md): place the correct per-libc interposer inside the
   rootfs and set `LD_PRELOAD` only when that rung is selected.
3. [T-0703](interpose.md): implement and verify the complete entry-point set and
   `*at` path-resolution rules.
4. [T-1110](milestones.md): run M6 acceptance across the libc matrix, including
   deliberate wrong-object selection and a named static-binary decline.
5. [T-0710](interpose.md) and [T-0711](interpose.md): implement the two rulings
   the operator settled on 2026-09-11. Both are written into the entries.
6. [T-0805](cli.md) and [T-0808](cli.md): finish four-part diagnostics and drive
   every parity row through the shipped binary. [T-0809](cli.md) adds the row
   that makes an ambiguous spawn failure readable.
7. [T-1207](gate.md): close the excluded interposer crate's independent gate
   coverage with its mutation proof.
8. [T-1111](milestones.md): the nix acceptance. ⭐ It is the first entry that
   drives a payload a consumer actually wanted, and podbox already ships every
   piece its hand-built solution needed except two.
9. [T-1108](milestones.md): package M7 only after M6 acceptance is green.
10. [podvm.md](podvm.md) T-1301 first, because every other entry there depends
    on the probe.

## In progress

No implementation entry is half-written. T-0503, T-0704 and T-1109 remain
partial with their remaining conditions recorded in their entry files.

## Operator questions

⭐ **None is open. Every ruling is written into the entry that owns it**, which
is where an implementer reads it. They are listed here so a reader can see that
nothing waits on the operator.

| question | ruled on 2026-09-11 | where it lives |
| --- | --- | --- |
| where the ownership memo lives | on the host, beside the container record | [T-0710](interpose.md) |
| what `setuid` does on a uid 0 payload | honest failure by default, and a flag turns on the lie | [T-0711](interpose.md) |
| whether host CA injection stays on | yes, unchanged: inject on an announcement, mark degraded, `--strict` refuses | [T-0407](complete.md) |
| what to do with a corpus dependency bump | close it, and fence `references/` off from every updater | `.github/dependabot.yml` |

⚠ **`/dev/ptmx` on the target is a measurement, not a ruling**, and it belongs
to [T-0503](enter.md). The probe already answers it; nobody has run it on the
intended target. The entry does not wait on the operator for that.

⛔ **Nothing is blocked.** T-0606 and T-0909 both carried `blocked` and neither
met the definition: nobody outside a session had to act for either to proceed.
Each entry's status note records what was wrongly inferred.

## Two decisions the next session should take, and neither needs the operator

- [T-1302](podvm.md) asks whether `podvm` is a flag, an argv0 alias, or both.
  Both are defensible and the entry says so; make the call, record the rejected
  one, and continue.
- [T-1111](milestones.md) asks whether the nix acceptance pins the known-good
  version pair or tracks the current one. The first measures podbox and the
  second measures the wall.
