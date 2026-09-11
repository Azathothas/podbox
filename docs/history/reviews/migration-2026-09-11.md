# Migration review, 2026-09-11

## Scope

Reviewed the complete release-candidate tree after migrating the source project
to `Azathothas/podbox`. The review covered behavior, security boundaries,
failure handling, documentation, automation, license and attribution, and
preservation of committed evidence.

## Material finding

The isolated full test run found that `Root::mknod_char` unlinked an existing
device shim before attempting `mknodat`. On a host that denies device creation,
the fallback recreated a regular file and reported `Created` on every pass. A
denial between unlink and recreation could also leave the rootfs damaged.

The implementation now creates a unique sibling device candidate, applies its
mode, and atomically renames it over the old entry only after creation succeeds.
Failure cleans the candidate and preserves the prior entry. The focused
idempotency test passes in the same rootless container where it failed before.

## Repository findings resolved

- The landing page described only M0 through M3 although M5 was complete.
- The router lived under `docs/` while the current template requires it at the
  root, and many links encoded the old location.
- The live progress file had become a 453-line session history. It is preserved
  under history and replaced by a compact current record.
- Hosted automation did not run workspace tests and used moving action tags.
- The excluded interposer crate was not linted or tested by the hosted gate.
- Security, human-judgment, architecture, code-map, changelog, remote-item, and
  dependency-update surfaces were absent.
- Package metadata still pointed to the source repository.
- The imported PowerShell document check mis-normalized valid paths containing
  three parent traversals. Both twins now use component-aware normalization.

## Remaining tracked limits

The implementation backlog remains open by design: 22 open, 3 partial, and 2
blocked entries. The current work order is in `TODO/PROGRESS.md`; this review
does not reclassify unfinished product milestones as migration defects.
