# Changelog

This file records repository-level changes to podbox. The current package
version describes the CLI and runtime behavior; unfinished milestone work stays
under Unreleased.

## Unreleased

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
`git ls-files` output, so all 203 links in the tree were reported as untracked.

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
