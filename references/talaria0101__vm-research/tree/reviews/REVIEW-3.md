# REVIEW 3 — fresh-eyes structural pass (the stranger's read)

Date: 2026-09-09 · Posture: read the repository as a stranger who has never
seen the sessions: does the router route, does every statement trace, is the
tree lean, is the history sane.

## Structure

- `README.md` routes to `experiments/` (evidence), `docs/AGENTS.md`
  (methodology), `reviews/`, `patches/`. All four targets exist.
- `docs/AGENTS.md` is self-contained: census log first, methodology rules,
  subject conventions, review discipline. A new session can act on it alone.
- `experiments/README.md` carries one entry per experiment with its question,
  verdict, and exit code; the table numbering matches the filenames.
- `patches/` holds exactly two files, both pulled out of experiment 33's path:
  an environment-completion shim (`libfakepasswd.c`, not an upstream patch)
  and a real upstream patch (`lima-v2.2.0-root-guard.diff`, 5-line guard
  disabled, reason in situ). The distinction is stated in both files.
- 53 tracked files, ~120 KB of logs, binaries and images gitignored.

## Consistency checks

1. Verdict rows vs logs: re-grepped after REVIEW-1's fixes — 34- now matches
   its log (`rc(start)=153`, state `created`, exec `not running`), 33- cites
   committed evidence (`logs/33-lima-ha-stderr.log`).
2. Counting language: "four run end-to-end" — pc, microvm, smolBSD, nanos.
   Lima is described as reaching qemu but not completing; not counted.
3. Exit codes: the README reproduce block promises 0/1/2 semantics; all 22
   scripts implement them; the two cosmetic drifts are documented in
   REVIEW-1 §3.
4. No stale rows remain: grep for the superseded smolvm wording returns 0.
5. `lib.sh` is committed non-executable (it is sourced, never run) — correct.

## Leanness

Everything in the tree is either a question (script), an answer (log), a
distillation (README/index), a correction (reviews), or an unblock (patches).
No transcripts, no session narration, no artefacts a `vfetch` can rebuild.

## Verdict

The repository stands on its own for a reader with no context: one paragraph
of environment facts, one matrix where every row names its log, one index that
reproduces the work. Ready to push.
