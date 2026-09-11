# Changelog

This file records repository-level changes to podbox. The current package
version describes the CLI and runtime behavior; unfinished milestone work stays
under Unreleased.

## Unreleased

### 2026-09-11T18:05:26Z: the references are mined, and two licence badges were wrong

**Record:** [`TODO/PROGRESS.md`](TODO/PROGRESS.md). No version bump and no
deployment.

Eleven repositories were mined with `scripts/common/mine-repo.sh`, each with its
tracker and its tree at a captured commit, each reporting zero gaps. Four had
been read at their URLs and kept out of the corpus; they are tracked now, which
is what [`docs/methodology/references.md`](docs/methodology/references.md)
section 4 requires. [`docs/history/2026-09-11-reference-sweep.md`](docs/history/2026-09-11-reference-sweep.md)
is the write-up and it opens with what the sweep did not establish.

⛔ **Two licence determinations were taken from a code host's badge and both
were wrong.** One tree reported `NOASSERTION` and carries the full Zero-Clause
BSD text, so it is vendorable. One reported `MIT` and declares
`GPL-2.0-or-later` in its manifest, so it is refused. A licence is read in the
tree.

⭐ **The `.gitignore` was excluding the evidence the sweep rests on.** Two
unanchored rules, `*.log` and `logs/`, matched every experiment log directory in
the corpus and kept **83 committed logs** out of the tree. A citation into one
resolved on this disk and would not have resolved in a fresh clone.
`scripts/check-todo.py` caught it.

⭐ **The TCG cost is a range, not a number.** Four measurements on one host
class span 3x to 21x across four workloads, and a document in the corpus states
one of them as the general figure. [`TODO/podvm.md`](TODO/podvm.md) T-1308 is the
entry, and podbox will never print a bare multiplier.

All 86 closed entries were reconciled against
[`TODO/RULES.md`](TODO/RULES.md) section 5.
`experiments/156-closure-records.sh` is the instrument. Two entries moved back:
one was closed with no run recorded at all, and one was closed on two tests that
[`TODO/image.md`](TODO/image.md) T-0215 has since measured as failing 5 of 12
runs.

Four licence questions were settled by the operator, and the nix acceptance is
now milestone M8 with its acceptance rows and its version pin.

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
`git ls-files` output. The separators differ there, so, measured on
2026-09-11, **203 of the 248 links the check then resolved** were reported as
untracked and the gate could not be run on Windows at all.

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
