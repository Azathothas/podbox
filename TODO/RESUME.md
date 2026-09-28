## The task

Audit and correct the documentation, scripts, and Windows lane. Read and
resolve podbox issue 68. Review the open pull requests and make task entries
for work that remains. Run the full gate, publish the result, and verify it.

## The resume point

The Windows fix and SSH task entries are staged. Run the full record and
Linux gates, run the plant suite, repair any findings, and complete three
review passes. Then commit and push `main`, verify live checks, and close
issue 68 with proof.

## In flight

The progress page has been rewritten. The reference captures, Windows
experiment, wrapper, gate and task entries are staged except for the most
recent edits. The full gate has not run on this changed tree.

## State

The initial tree was clean on `main` at `5701a8b`. The worktree is
dirty by design. The local TODO gate last reported four document/count
errors; the cited paths and count were then corrected, but the gate has
not been rerun. The Linux gate has not run. The lane cleanup report was
empty after the Windows experiment.

## The paste

```text
Read AGENTS.md, then TODO/PROGRESS.md and TODO/RESUME.md. Continue the
2026-09-28 Windows lane and documentation change on main. Run and repair
the full gate and plant suite, review every changed file three ways, then
commit, push, and verify issue 68 and main. Keep proof in tracked files.
```
