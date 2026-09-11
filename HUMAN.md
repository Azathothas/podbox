# Human notes

podbox is built for automated callers, but several decisions still require an
operator rather than a heuristic.

## Start here

The current state and ordered work are in
[`TODO/PROGRESS.md`](TODO/PROGRESS.md). The complete backlog and its checked
counts are in [`TODO/INDEX.md`](TODO/INDEX.md). Each entry carries its own
premise, decision, and acceptance command.

## Decisions automation must not invent

- Whether the ownership memo may live inside a payload-visible rootfs.
- Whether identity-changing calls should fail honestly or succeed as a
  compatibility lie in the interposer rung.
- Whether a target without a usable `/dev/ptmx` should refuse terminal mode.
- Whether host CA injection is acceptable for a particular execution.

These questions remain visible in the live progress record until an operator
settles them. A missing answer is reported as unknown or unsupported, never
filled with a convenient default.

## What automation does cover

The local and hosted gates check formatting, lint, tests, TODO consistency,
documentation links, action pins, sensitive material, binary constraints, and
the separate interposer crate. Experiment scripts print their inputs and use
exit 2 for a measurement the environment cannot perform.
