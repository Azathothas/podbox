# Session summary: SSH reconcile, partial, machine arm, KVM unblock

Date: 2026-09-28. Lane: Windows (`wsl-toolkit 6.0.0`, base `podbox`).
Start instant: 2026-09-28T12:11:43Z. End: 2026-09-28T13:37:34Z.

## Summary table

| row | before | after | how |
| --- | --- | --- | --- |
| Elapsed | 2026-09-28T12:11:43Z | 2026-09-28T13:37:34Z (1h 26m) | `date -u`, both ends read from a clock |
| Commits | `a431cb7` | 5 commits to `2bd3c35`, pushed to `main` | `git log --oneline a431cb7..HEAD` |
| Work | the operator's 6-clause pipeline | all 6 done; entries stay open on named proofs | this file and TODO/PROGRESS.md |
| Changes | clean tree at `a431cb7` | 68 files, +4578, -41 (corpus is most of it) | `git diff --shortstat a431cb7..HEAD` |
| Size | - | 7,765 files, 3,113,544 lines by `scc` | `scc` (names the counter; blank/comment counting differs) |
| Checks | todo ok; fast gate 10 passed, 1 skipped | full Linux gate 10 passed, 0 failed; fast gate 9 passed, 0 failed, 2 environmental skips; todo, markers, one-home, docs, secrets x2 green | lane and host runs, exit codes read unpiped |
| CI | hosted gate green on `81fec0c` | 4 jobs queued on the push, no conclusions yet | code host's own report |
| Cost | - | ~910 MB ValidationOS extent on host disk (outside tree); ephemeral lane downloads discarded | tool output |
| Health | 4 open, 1 partial, 0 blocked, 175 done | same counts; tree clean; ledger empty; base kept | `check-todo`, `git status`, `gc --json` |

## What the numbers rest on

- `experiments/386-podssh-partial.sh` <!-- known-absent --> HOLDS (deleted
  2026-10-01 as stale per VC-1; historical): 24 unit plus 5 proxy
  tests, 3 live-SSH e2e (exact bytes, empty stderr, exit 42
  passthrough), 8 machine-surface rows through the built binary.
- `experiments/385-kvm-open.sh` HOLDS: `/dev/kvm` opens, API version
  12, virtual machine creates.
- Full Linux gate green on the pushed tree, including clippy
  `-D warnings`, `cargo fmt --check`, and `check-todo: ok`.
- Four review verdicts (PR 66, PR 67, faketty, fakepty) verified against
  recorded heads and mined corpus trees before use.

## Findings for the next session

- `THIRD_PARTY.md` license table is stale: it names neither
  `talaria0101__sandssh`/`talaria0101__dropssh` nor the two new trees.
  The determinations live in `TODO/reference-map.md`.
- Host lane traps, measured: `python3` is a store stub (use `py`);
  Windows Python and Git Bash disagree on `/tmp` (resolve with
  `cygpath -w`); a `_IO` ioctl with a buffer argument returns EINVAL
  where the null shape returns the version.
- The `probe-child` stderr lines in probe tests are libtest's negative
  output, not a failure.
