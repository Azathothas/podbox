## The task

Correct the Windows lane, current docs, and house checks. Resolve podbox
issue 68. Review pull requests 66 and 67 and record the work that remains.
Run the full gate, publish the result, and check the published behavior.

## The resume point

The Windows lane, source records, and SSH entries are in local commits on
`main` through `c838759`. T-1345 and T-1346 fixed the Bash caller and
the plant script's input path. T-1347 made check 27 read the milestone
entries. The Linux check passed, and the full plant repeat caught all 43
cases with no miss. The Windows drive passed again and collected its jobs.
The final session record is in the worktree. Run the host strict gate,
commit the record, push `main`, and check remote CI. Close issue 68 after
remote proof. Then record the final CI and machine state in PROGRESS and
the session summary and publish that record.

## In flight

The local record and document checks last passed at 180 entries before
the final record edit. The host cleanup report is empty. The summary
table is saved with pending fields that must be filled after publication.
