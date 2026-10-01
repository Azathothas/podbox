# Project rules

[INDEX](INDEX.md) lists the entries. [PROGRESS](PROGRESS.md) gives the work
order. [RESUME](RESUME.md) gives the cold-start handoff.
[reference-map](reference-map.md) records source and licence decisions.

## 1. Start

Run the session start command. Read the required orientation pages in full.
Run the record gate before changes. Read each applicable router row.
Use CodeGraph first for source discovery. Confirm its answer in the file.

## 2. Branch and remote

Work on `main`. The operator permits a normal push to `origin/main`.
Do not force-push, rewrite published history, or skip required checks.
Other repositories are read-only.

A branch can have equivalent changes without being an ancestor of `main`.
Compare commits, patch identities, source, and proof before integration.
Do not apply an equivalent patch twice.
Delete an obsolete branch only after the retained work is verified.

If protection refuses a direct push, use the documented publish-branch
fallback in [git.md](../docs/conventions/git.md).
Keep its scope narrow. Remove it after integration.

## 3. Session end

Finish the operator's scope, or stop when the operator says to stop.
An entry count does not override either instruction.
Use [sessions.md](../docs/methodology/sessions.md) for closure.
Save a precise handoff when work remains.

Update task records in the same change as the work.
Save the review, proof, summary, and machine state.
The next prompt belongs in chat only.

## 4. Counts

Use `./target/release/podbox-count` to set a status and derive the counts.
Do not edit count tables by hand.
Run `./target/release/podbox-gate` to check the result independently.

## 5. Entry closure

Keep the ten entry fields defined by
[authoring.md](../docs/methodology/authoring.md).
Run the entry's proof. Save the result and its limits.
For a done entry, the first paragraph after Prove starts with bold Done.

Keep the title when a premise is disproved. Put the correction below it.
A blocked entry names the blocker and the condition that clears it.
A partial entry names the implemented part and the remaining acceptance.
An earlier Done record proves only its stated conditions.

## 6. Measurements

Use a tracked script with pinned inputs and printed conditions.
Save results under `experiments/results/`.
Exit 0 means matched, 1 means tested and failed, and 2 means could not run.
Commit negative results. Do not call a missing test a denial.
Label an estimate each time it appears. Use a dash for an unknown value.

## 7. Third-party material

Read [vendoring.md](../docs/methodology/vendoring.md) before a copy or patch.
Check the licence before use. Keep notices and captured revision.
Fix a vendored defect here. Do not write to another repository.

## 8. Resources and liveness

Check free blocks and inodes before large writes.
Bound external commands, child waits, and network operations.
Use non-interactive commands. Read each process exit code without a pipe.
A resource option accepted by an engine does not prove enforcement.

### Post-task cleanup, after every task

Save each job's output under `experiments/results/` before collection.
Then collect that job: `wsl-toolkit --instance podbox gc --job <id> --apply`.
The report and cleanup record are committed together.
This order lets the record gate reject retained job resources at commit.

Remove only scratch that this session owns. Verify its resolved path first.
Keep any input needed for a pending proof outside temporary scratch.
Do not remove the base, shared image cache, or another session's files.

At session end, inspect `wsl-toolkit --instance podbox gc --json`.
Use `gc --apply` only when every listed resource belongs to this session.
Otherwise collect each owned job by id.
Check 29 holds the procedure and rejects retained lane resources.

## 9. Prose

Use [prose.md](../docs/conventions/prose.md).
Write short technical sentences. Correct current text in place.
Keep earlier evidence in history. Avoid session narrative in live pages.

## 10. Reviews

Review each touched file with three questions:

1. Does source or a saved run support each claim?
2. Do the interfaces, records, links, and counts agree?
3. Can a new agent follow the work without prior context or temporary files?

Record the scope, result, and limits of each pass.
Do not invent a finding to satisfy a review quota.

## 11. Standing decisions

| Decision | Authority |
| --- | --- |
| Rust; static musl release by default | captured TOOL.md section 3; T-1002 |
| 0BSD for project-owned work | LICENSE |
| Tracked source corpus | reference-map.md |
| No tool attribution in commits or releases | git.md section 1 |
| No upstream writes | vendoring.md |
| Refuse the docker name beside a reachable daemon unless explicitly selected | T-0803 |
| Windows route is wsl-toolkit, instance podbox | containers.md |
