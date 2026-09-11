# Changelog

This file records repository-level changes to podbox. The current package
version describes the CLI and runtime behavior; unfinished milestone work stays
under Unreleased.

## Unreleased

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
