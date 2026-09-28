## The task

Correct the Windows lane, current docs, and house checks. Resolve podbox
issue 68. Review pull requests 66 and 67 and record the work that remains.
Run the full gate, publish the result, and check the published behavior.

## The resume point

The Windows lane, source records, and SSH entries are in local commits on
`main`. T-1345 and T-1346 fixed the Bash caller and the plant script's
input path. The full plant suite then caught 42 cases and missed 27a:
check 27 took shipped milestone status from a former PROGRESS sentence.
T-1347 and the check correction are in the worktree. The host fast gate
passed; the Linux check is running. Commit the correction after it passes,
then rerun the full plant suite and host strict gate. Repair any finding.
Push `main`, check remote CI, and close issue 68 after remote proof.
Update PROGRESS and the review record with the final results.

## In flight

The local record and document checks pass at 180 entries. The full plant
repeat is pending. The host cleanup report was empty after the prior
plant run. A final session summary table is not yet saved.
