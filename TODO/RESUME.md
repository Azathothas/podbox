## The task

Correct the Windows lane, current docs, and house checks. Resolve podbox
issue 68. Review pull requests 66 and 67 and record the work that remains.
Run the full gate, publish the result, and check the published behavior.

## The resume point

The Windows lane, source records, and SSH entries are in commit `251b230`
on `main`. A live plant call found that the wrapper used `sh` for a Bash
caller. T-1345 and the wrapper correction are in the worktree. The host
strict gate and Linux check are running against this correction. Save
their results. Commit the correction, run the plant suite in the clean
base copy, repair any finding, then push `main` and check remote CI.
Close issue 68 after the remote proof passes. Update PROGRESS and the
review record with the final results.

## In flight

The local record check and document check pass at 178 entries. The
full plant result is pending. The host cleanup report was empty after
the earlier live Windows drive. A final session summary table is not
yet saved.

## The paste

```text
Read AGENTS.md, TODO/PROGRESS.md, and TODO/RESUME.md. Continue the
2026-09-28 Windows lane correction on main. Finish the running gates,
commit T-1345, run the full plant suite, then save the review and session
record. Push main, check remote CI, and close issue 68 if it is green.
```
