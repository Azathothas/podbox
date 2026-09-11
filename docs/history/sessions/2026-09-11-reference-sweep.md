# Session summary, 2026-09-11, the reference sweep

⚠ **A saved copy of the summary printed in chat**, per
[`../../methodology/sessions.md`](../../methodology/sessions.md). The record is
[`../../../TODO/PROGRESS.md`](../../../TODO/PROGRESS.md) and this file carries
no work order. The findings are
[`../2026-09-11-reference-sweep.md`](../2026-09-11-reference-sweep.md).

| row | before | after | how it was taken |
| --- | --- | --- | --- |
| Elapsed | `2026-09-11T17:32:02Z` | `2026-09-11T18:17:26Z` | both read with `date -u`; **45 m** |
| Commits | `dca0226` | `e965ff7`, 3 commits | `git log --oneline dca0226..HEAD` |
| Work | 0 references mined, 3 licences unruled | 11 mined with 0 gaps, 0 unruled | `PROVENANCE.md` per tree; [`reference-map.md`](../../../TODO/reference-map.md) |
| Entries | 125 | 131: 42 open, 4 partial, 0 blocked, 85 done | `./scripts/check-todo.py` |
| Changes | - | 1,218 files, +191,208, -131. Outside the corpus: **18 files, +1,190, -131** | `git diff --shortstat dca0226..HEAD` |
| Size | 48,381 | 49,333 code lines outside the corpus, **+952** | `scc`, excluding `references/`, `.git`, `.codegraph`, `target`, `.dev` |
| Corpus | 30 trees | 41 trees, 166 MB over 7,815 tracked files | `git ls-files references/ \| wc -l`, `du -sh references` |
| Checks | green | green on both host halves | the gate, run unpiped |
| CI | green | green on `main` | the hosted run |
| Cost | - | no money, no quota-bearing registry, no container job | the `gh` route and `du` |
| Health | 0 open pull requests | 0 open, tree clean, machine stopped | `gh pr list`, `git status --short`, `podman machine inspect` |

## What was done

⭐ **Eleven repositories were mined under
[`../../methodology/references.md`](../../methodology/references.md)**, each with
its tracker and its tree at a captured commit, each reporting zero gaps. Four of
them had been read at their URLs and kept outside the tree, which that procedure
refuses: an untracked corpus exists on one machine only.

Four licence questions the previous session left open were settled by the
operator, and the nix acceptance became milestone M8 with its acceptance rows
and a version pin that two upstream gates force.

## The three corrections that changed an answer

⛔ **Each one had been recorded the other way, and each was read at the source
to settle it.**

1. A tree recorded as unlicensed and not vendorable carries the full
   Zero-Clause BSD text in `tree/LICENSE`. ⭐ It is vendorable. The code host's
   badge reports `NOASSERTION` because the heading is not the canonical string
   its classifier matches.
2. A tree recorded as MIT and vendorable declares `GPL-2.0-or-later` in its
   manifest, inherited by both member crates. ⛔ It is refused.
3. **None of the five candidate machine-tier tools is a microVM manager for a
   capability-denied host.** Three require hardware acceleration, one composes
   command lines, and one is a plugin binding. That is the answer to the
   question the survey was given, and it is the opposite of what the candidate
   list assumed.

## What the gate found that no reader would have

⭐ **83 committed corpus logs were not in the tree.** Two unanchored
`.gitignore` rules matched every experiment log directory in the corpus, so
every citation into one resolved on this disk and would not have resolved in a
fresh clone. `scripts/check-todo.py` refused the citations and that is how it
surfaced.

## What was measured and moved

⛔ **Every closed entry was reconciled against
[`RULES.md`](../../../TODO/RULES.md) section 5.**
`experiments/156-closure-records.sh` is the instrument and it exits 0 now.

- **T-0408** was closed with a `Prove` line and nothing after it: no run, no
  output, no date. It is open again.
- **T-0211** was closed with no record, and **both tests its `Prove` names are
  in T-0215's measured-intermittent set** - the four store lock tests that
  failed 5 of 12 workspace runs. It is partial again, and it now waits on
  T-0215.

⚠ **A first count of the reconciliation was wrong by thirty entries**, because
it grepped for the closure marker with a trailing space and missed every record
written without a date. That error is why the measurement ships as a script.
