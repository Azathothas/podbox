# Session summary, 2026-09-11

⚠ **A saved copy of the summary printed in chat**, per
[`../../methodology/sessions.md`](../../methodology/sessions.md). The record is
[`../../../TODO/PROGRESS.md`](../../../TODO/PROGRESS.md) and this file carries
no work order.

| row | before | after | how it was taken |
| --- | --- | --- | --- |
| Elapsed | `2026-09-11T16:00:38Z` | `2026-09-11T17:15:27Z` | both read with `date -u`; **1 h 14 m** |
| Commits | 1 | 6 | `git log --oneline` |
| Work | 4 operator questions open, 2 entries blocked | 0 open, 0 blocked | the entries |
| Entries | 114 | 125 | `./scripts/check-todo.py` |
| Changes | - | 34 files, +1,888, -161 | `git diff --shortstat b27b3d9..HEAD` |
| Size | - | 48,381 code lines outside the corpus | `scc`, excluding `references/`, `.git`, `.codegraph`, `target`, `.dev` |
| Checks | host gate green, unusable on Windows | green on all three lanes | the gate, run unpiped |
| CI | green | green on all four jobs | the hosted run on `main` |
| Cost | - | no money, no quota-bearing registry | the jobs ran against `docker.io/library/rust` and the base's own cache |
| Health | 4 corpus pull requests open | 0 open, corpus fenced | the code host's own listing |

## What was done

Three host lanes, and `scripts/session-start.sh` picks one. The Windows lane
runs every Linux step in a disposable container inside `wsl-toolkit-podbox`,
and `wsl.exe` is never called.

Four rules the operator settled for this home were written into the files that
own them: no tool attribution, amend in place, Simplified Technical English,
CodeGraph before `grep`, the Windows route, and the two triggers that end a
session.

## The three defects that made a check wrong rather than red

⭐ **Each one reported success over a subject it had not examined**, which is the
class the gate exists for.

1. `scripts/check-todo.py` compared a host-separated relative path against
   `git ls-files` output. Measured on 2026-09-11: **203 of the 248 links the
   check then resolved** read as untracked on Windows, so the gate could not run
   there at all.
2. `scripts/todo-count.py` wrote tracked documents in the host's line ending.
   `.gitattributes` declares `eol=lf`, so git normalised on read and
   `git status` stayed clean while the bytes on disk were wrong.
   `check-one-home` then joined two short sentences across a carriage return
   into one of thirteen words, and reported a duplicated fact that does not
   exist. It failed on Linux and passed on Windows over the same commit.
3. `references/` had no end-of-line rule of its own. Some committed blobs there
   carry CRLF while the rules above them declare those extensions LF-only, so a
   `git add` on a machine with a cold stat cache would have rewritten evidence.

## What was measured and not fixed

⛔ **`cargo test --workspace` is not deterministic.** Measured in the same image
CI uses: **5 of 12 runs failed**, across four store lock tests. The same test
alone passed 30 of 30, and its own suite passed 15 of 15 in both thread modes,
so the trigger is cross-binary concurrency. `TODO/image.md` T-0215 carries the
measurement, the two candidate mechanisms and the rule that neither may be
written down as the cause before it is measured.

⚠ **A single green run had been read as evidence for that suite**, including by
the migration record. It is not.
