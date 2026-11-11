# Project rules

[INDEX](INDEX.md) lists the entries. [PROGRESS](PROGRESS.md) gives the
state and the only work order. [RESUME](RESUME.md) gives the cold start.
[reference-map](reference-map.md) records source and licence decisions.
`AGENTS.md` has the rules of each session; it has priority over this
page. [docs/decisions.md](../docs/decisions.md) has each ruling of the
owner since 2026-11-06.

## 1. Start

Do the steps of `AGENTS.md`, section 8. Read the required orientation
pages in full. Run the record gate before changes. Read each applicable
row of the task routing table (`AGENTS.md`, section 13).
Use CodeGraph first for source discovery when a `.codegraph/` exists.
Confirm its answer in the file.

## 2. Branch and remote

Work on `main`. Push to `origin main` without force (`AGENTS.md`,
section 3). Do not rewrite published history. Do not skip a required
check. The protection of `main` is removed (`docs/decisions.md`,
2026-11-06, "GitHub"), so no publish branch is necessary.

⭐ 2026-10-02. The owner grants read and write on this repository. That
covers `git tag`, `git push` of a version tag, the GitHub release it
publishes, and the dispatch of the workflows of this repository
(`docs/decisions.md`, 2026-11-09, "Workflow dispatch"). An agent does not
stop to ask about those. The grant does not cover a force-push, a history
rewrite, or a deleted tag or release. A beta release is permitted when it
fits; a crates.io release is not (`docs/decisions.md`, 2026-11-09,
"Releases").

Other repositories are read-only. A message to a session of another
repository of PodUtils is a COORD issue in this repository (`AGENTS.md`,
section 9).

A branch can have equivalent changes without being an ancestor of `main`.
Compare commits, patch identities, source, and proof before integration.
Do not apply an equivalent patch twice. Never merge or delete the branch
`archive/podbox-worktree`.

## 3. Session end

Finish the scope that the owner gave, or stop when the owner says to
stop. An entry count does not override either instruction.
Use [sessions.md](../docs/methodology/sessions.md) for closure.
Save a precise handoff in `docs/HANDOFF.md` when work remains.

Update task records in the same change as the work.
Save the review, proof, summary, and machine state.
The next prompt belongs in chat only.

## 4. Counts

Use `./target/x86_64-unknown-linux-musl/release/podvm-count --set T-NNNN
STATUS` to set a status. Run `podvm-count` with no argument to derive the
counts. It rewrites the Counts block of `TODO/INDEX.md`. Do not edit a
count by hand. Copy the totals line that it prints into the State of
`TODO/PROGRESS.md`, as `N entries: ...`; pod-todo's reader compares the
two (`podvm-todo check`, and check 35 of the gate).
Run `./target/x86_64-unknown-linux-musl/release/podvm-gate` to check the
result independently.

## 5. Entry closure

Keep the header fields and the sections that
[authoring.md](../docs/methodology/authoring.md) defines, in the format
of `pod-todo`. Run the entry's proof. Save the result and its limits.
A done entry has a section `## Done`, and its first paragraph starts with
bold Done and a date. `./target/x86_64-unknown-linux-musl/release/podvm-todo check`
reads the record as `pod-todo` reads it.

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
A defect in a file that podcrates imported goes to podcrates as a COORD
issue (`AGENTS.md`, section 9, item 4).

## 8. Resources and liveness

Check free blocks and inodes before large writes.
Bound external commands, child waits, and network operations.
Use non-interactive commands. Read each process exit code without a pipe.
A resource option accepted by an engine does not prove enforcement.
Bound each KVM guest or emulator with a timeout. Make sure that no
emulator stays after the run.

### Post-task cleanup, after every task

Save each job's output under `experiments/results/` before collection.
Remove only scratch that this session owns. Verify its resolved path first.
Keep any input needed for a pending proof outside temporary scratch.
Do not remove the base, shared image cache, or another session's files.

The parked Windows lane (`docs/containers.md`) adds these steps.
Collect a persistent job: `wsl-toolkit --instance podvm gc --job <id> --apply`.
An ephemeral lane job has already removed its record.
The report and cleanup record are committed together.
At session end, inspect `wsl-toolkit --instance podvm gc --json`.
Use `gc --apply` only when every listed resource belongs to this session.
Otherwise collect each owned job by id.
Check 29 holds the procedure and rejects retained lane resources.

## 9. Prose

Use [prose.md](../docs/conventions/prose.md), which is ASD-STE100.
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

These decisions are of 2026-10-02 and before. They stay in force where no
ruling of [docs/decisions.md](../docs/decisions.md) supersedes them. Each
ruling since 2026-11-06 is there, not here.

| Decision | Authority |
| --- | --- |
| Build compatibility is not a goal. Users take a published binary. Take whatever toolchain feature unlocks the tasks, nightly included. An agent fixes a toolchain rejection instead of asking the owner to pin. | Operator, 2026-10-02, over the `invalid_runtime_symbol_definitions` question on `crates/podvm-interpose/src/lib.rs`; the owner's ruling "Static, C, Rust" of 2026-11-05 |
| Dead code is a fault of the reader until proven otherwise. It is missing or unlooked-for use 99 times in 100. Make it needed, or find where it is needed and prove it is not needed there. Never delete a field to quiet a lint. | Operator, 2026-10-02, over `crates/podvm-image/tests/common/registry.rs` |
| No deferrals. Work that needs a human becomes a tracked task with a named clearing condition, not a note in an untracked file. Batch for a later queue if needed, but finish it. | Operator, 2026-10-02 |
| Read and write on this repository is authorized, including tags and releases. See section 2. | Operator, 2026-10-02 |
| Rust; static musl release by default | captured TOOL.md section 3; T-1002; the owner's ruling "Static, C, Rust" of 2026-11-05 |
| 0BSD for project-owned work | LICENSE |
| No tool attribution in commits or releases | git.md section 1 |
| No upstream writes | vendoring.md |
| Refuse the docker name beside a reachable daemon unless explicitly selected | T-0803 |
| Everything the KVM proof of the parked Windows lane needs is installed into `wsl-toolkit-podvm`. The Windows host qemu is never touched, used, or invoked, even where a host binary would answer the question. | Operator, 2026-10-02; T-1610 |
| ReactOS is descoped, not deferred. Proving podvm against a beta operating system is proving against a moving target, and it is low priority. T-1112 closes on its KVM leg. | Operator, 2026-10-02; T-1112 |
| Work unattended to completion. Use subagents and swarms where they help; do not stop to ask about anything already settled in section 11, section 2 or `docs/decisions.md`. Take the work order, finish it, and leave the record green. | Operator, 2026-10-02 |

### Superseded

The table "Superseded" of `docs/decisions.md` records each replacement.

| Decision of 2026-10-02 or before | Replaced by |
| --- | --- |
| Windows route is wsl-toolkit, instance podvm (`containers.md`). | Cloud sessions (2026-11-06); the parked lane uses the instance `podvm` (C4). |
| Tracked source corpus (`reference-map.md`). | References (2026-11-06): each reference is a GitHub issue with the label `reference`. |
| KVM and Windows-guest work is deferred; no session starts a KVM guest. | KVM (2026-11-09). |
| An unattended KVM guest run is permitted on this host only, with a watchdog outside the guest that removes an emulator that outlives its bound. | KVM (2026-11-09): a session can start a KVM guest where `/dev/kvm` exists, with a timeout and no emulator left after the run. |

## 12. Subagents and parallel work

Subagents are the normal way to take this project, not an exception.
Nothing in this section is optional and nothing in it needs the owner.

**One writer for the record.** `podvm-count` rewrites the whole Counts
block of `TODO/INDEX.md` from every row, and `check_tree` then reads the
whole tree. Two agents editing `TODO/INDEX.md` are both green and the
merge is wrong. The same is true of any file whose whole content one
tool rewrites: `docs/runtime-state.md` is written by `document-state`,
and `TODO/PROGRESS.md` carries the only work order. Pick one writer per
file per batch and let the others read.

**Dispatch on disjoint files.** Take the open rows from
`TODO/INDEX.md` and group them by the files their Approach names. Two
entries whose files are disjoint run in parallel, each agent in its own
worktree under .claude/worktrees (`AGENTS.md`, section 3). Two entries
that edit the same file, or prove on the same check, run in series behind
one writer.

**A subagent reports, it does not assume.** Give it the exact path or
command, not a description of where to look. Read its conclusion; do
not paste its intermediate output into the record as fact. Every claim
it makes is a claim until something reads the artefact.

**Proof is the parent's job.** A subagent may run a gate or a check to
answer a question. Closing an entry, writing its Done paragraph, and
running `podvm-count` belong to one writer at the end. Two agents
closing two entries at once is how a count line and a row disagree.

**Never `git add -A` while a tool is writing the tree.** `podvm-plant`
and the `--script` family plant defects into tracked files and restore
them at the end. Staging during that window commits a defect. Stage
explicit paths. This happened once on 2026-10-02 and `246b462` reverted
it.

**One cargo build at a time.** A build can use all the memory of the
machine (`AGENTS.md`, section 7). This holds for subagents too.

**A KVM guest needs a brief that names it.** A subagent starts a KVM
guest or an emulator only when its brief says so. One guest runs at a
time, with a timeout (`AGENTS.md`, section 7).
