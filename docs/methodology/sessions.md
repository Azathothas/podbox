# Sessions

The tree and repeatable evidence carry session state.
A chat summary does not establish a result.

## Start

`AGENTS.md`, section 8, gives the start procedure and its order:
the session hook, `docs/HANDOFF.md` when it exists, the progress record,
the decisions, the tree state, the CI runs and open issues, the watcher of
the organization, and the toolchain with the record tools. Then:

1. Read the task's required documents (`AGENTS.md`, section 13).
2. Inspect local changes before you edit them.
3. Read the current remote refs when the task involves publication.
4. Record the session start in UTC.

The session entry point selects the host procedure.
Do not select a base or container procedure from memory.
[Containers](../containers.md) gives the supported procedures.

## Reading receipt

For required orientation, report the file line count and last section heading.
Read the full bytes before you report a receipt.
A file name or heading listing is not a completed reading.

Do not reread unrelated corpus trees.
Read the task's named reference and captured revision.
State the depth reached during new research.

## Resume file

Overwrite `TODO/RESUME.md` when the next action changes.
It carries the assigned scope, next action, work in progress, tree state, and paste prompt.
It does not duplicate the progress work order.

Name uncommitted files and failing checks.
Name a running job and how to collect its result.
Keep each required input in a tracked procedure or an explicit operator parameter.
Do not require a file left in `.tmp/`.

## Work

Use the approved task scope.
A direct implementation or repair request supplies authorization for that scope.
A planning intake stays a planning task unless the operator authorizes implementation.

Keep the entry, index, and progress record with each change.
Run the acceptance clauses before completion.
Record incomplete clauses as partial or blocked.
A difficult task is not automatically blocked.
A blocked task names the external action that would let it proceed.

Use another authorized task while an external condition remains unavailable.
Do not repeat a completed proof without a changed input or unresolved finding.
Do not invent a new acceptance condition after the recorded condition passes.

## Stop and close

Complete the operator's requested scope.
If the operator requests a stop, stop implementation and save a precise handoff.
Do not continue implementation during the save.

Before a normal close:

1. Review each touched file under the [three lenses](reviews.md).
2. Run the full [gate](gate.md).
3. Correct the affected live documents.
4. Refresh progress and resume state.
5. Commit coherent work and perform authorized publication.
6. Verify checks on the published commit.
7. Save evidence before resource collection.
8. Make sure that no guest emulator of this session still runs.
9. On the parked Windows lane, collect this session's ended jobs by ID,
   restore any engine state that the session changed, and keep the base.

Do not remove another session's live resources.

## Summary

Print and save one measured table beside the record.

| Row | Evidence |
| --- | --- |
| Elapsed | Recorded start and end UTC times |
| Commits | Git log over the session range |
| Work | Assigned items and their final status |
| Changes | Git diff statistics |
| Size | Named line counter or Git shortstat |
| Checks | Starting and final gate results |
| CI | Check state on the final remote commit |
| Cost | Measured cost, or a dash |
| Health | Tree status and resource report |

Write a dash for an unmeasured value.
A result that did not improve still belongs in the table.

## Next prompt

Print the next prompt in chat inside a fenced block.
If assigned work remains, use a resume prompt.
Otherwise, point to the progress record for the next unit.
Do not copy the work order into a second file.
The resume file retains the immediate recovery instruction.

## Resume verification

Inspect Git status, local changes, the record, and current checks.
Compare any deployment with the committed tree.
Recover uncommitted work before you build on it.
Confirm that each claimed artifact exists.

## Waiting

Wait within the active session on the job or its bounded status command.
Keep waits short enough to report progress.
[Shell rules](../conventions/shell.md) own process and exit-code handling.
