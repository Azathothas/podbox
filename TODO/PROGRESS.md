# Progress

## State

M0 through M5 are implemented. M6 is partial: the interposer builds for glibc
and musl, but placement, the complete entry-point set, ownership-memo policy,
identity-call policy, and end-to-end acceptance remain open. M7 packaging has
not started.

114 entries: 22 open, 3 partial, 2 blocked, 87 done.

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

The raw source-era record, including all measurements and resolved questions,
is preserved at
[`docs/history/source-progress-ea5b671.md`](../docs/history/source-progress-ea5b671.md).

## Current work order

1. [T-0702](interpose.md): place the correct per-libc interposer inside the
   rootfs and set `LD_PRELOAD` only when that rung is selected.
2. [T-0703](interpose.md): implement and verify the complete entry-point set and
   `*at` path-resolution rules.
3. [T-1110](milestones.md): run M6 acceptance across the libc matrix, including
   deliberate wrong-object selection and a named static-binary decline.
4. [T-0710](interpose.md) and [T-0711](interpose.md): settle memo placement and
   identity-call behavior with operator decisions.
5. [T-0805](cli.md) and [T-0808](cli.md): finish four-part diagnostics and drive
   every parity row through the shipped binary.
6. [T-1207](gate.md): close the excluded interposer crate's independent gate
   coverage with its mutation proof.
7. [T-1108](milestones.md): package M7 only after M6 acceptance is green.

## In progress

No implementation entry is half-written. T-0503, T-0704, and T-1109 remain
partial with their remaining conditions recorded in their entry files.

## Operator questions

- Is `/dev/ptmx` usable on the intended target? Run
  `podbox probe --json | jq .ptmx` there.
- May the interposer ownership memo live inside the payload-visible rootfs, or
  must it be host-side?
- Should `setuid`, `setgid`, and `setgroups` fail honestly or succeed as a
  compatibility lie when the process already has uid 0?
- Is host CA injection acceptable for each intended environment, or should
  deployments default to `--no-host-cas`?

T-0606 and T-0909 remain blocked. Their entry files state what external fact
would clear each blocker.
