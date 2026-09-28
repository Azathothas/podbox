## The task

Correct the Windows lane, current docs, and house checks. Resolve podbox
issue 68. Review pull requests 66 and 67 and record the work that remains.
Run the full gate, publish the result, and check the published behavior.

## The resume point

The Windows lane, source records, and SSH entries are in local commits on
`main`. A live plant call found that the wrapper used `sh` for a Bash
caller. That correction is in commit `9afeb90`. The next plant call
found that the script resolved the checkout from `/in/job.sh`. T-1346
and its path correction are in the worktree. The host strict gate and
Linux check passed on the corrected tree. Commit T-1346, run the plant
suite in the clean base copy, and repair any finding. Then push `main`
and check remote CI. Close issue 68 after the remote proof passes.
Update PROGRESS and the review record with the final results.

## In flight

The local record check and document check pass at 179 entries. The
full plant result is pending. The host cleanup report was empty after
the live Windows drives. A final session summary table is not yet saved.
