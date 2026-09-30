# Reference study

## Source and scope

Capture the commit before you remove Git metadata.
Read the license before you copy source.
A repository is read-only unless the operator explicitly names it as a write target.
For podbox, other repositories remain read-only.

Use the repository fetcher for a remote capture:

```sh
sh scripts/common/mine-repo.sh OWNER/REPO --out references
```

For a local study, read the current tree and record its Git state.
Keep a selected source capture when the findings depend on those bytes.
State every trim and the reading depth.
Do not run another project's setup or gate when the task authorizes reading only.

## Code passes

Read the source yourself.
Do not delegate reference reading to another agent.

1. Read the project shape and the problem it solves.
2. Inspect the construction at exact files and lines.
3. Inspect the failure and resource paths that matter to podbox.
4. Compare each candidate mechanism with podbox's existing implementation.

Use the existing code index without changing the source repository.
Confirm returned source against the captured commit.
A name match does not establish behavior.

## Tracker

Read issues and pull requests in both states.
Read comments, review comments, and available discussions.
The repository fetcher records unavailable data in its provenance file.
State a missing tracker pass in the findings.
A claim from a tracker describes intent until source or a run confirms it.

Use `gh` first. Bound network operations.
Use a read-only proxy when the direct route cannot answer.
Do not send credentials to a proxy.
[Remote rules](../security/remote-ops.md) define the routes and authority.

## Corpus

Keep source evidence under `references/`, tracked on `main`.
Keep original source paths and notices.
Do not include instruction files, credentials, build outputs, or caches.
Record the selected commit in `PROVENANCE.md`.

A selected capture is permitted for a focused local study.
It must name omitted areas and must not claim a full-tree review.
A future session must be able to inspect each cited line without session scratch.

## Findings

State what the study did not establish before recommendations.
Give each transferred mechanism a source, reason, and task.
Check whether podbox already implements it.
Use one of these outcomes:

| Outcome | Meaning |
| --- | --- |
| adopt | A named mechanism enters a named podbox task |
| confirms | Podbox already uses the mechanism |
| defect example | A specific implementation shows a reproducible failure |
| filed elsewhere | An existing task owns the work |
| refused | The constraints or license prevent the proposed use |

Keep the findings under `docs/history/references/`.
Keep a usable source map and current tasks in their live homes.
Update [the reference map](../../TODO/reference-map.md) in the same change.

## Measurements

A source reading does not establish performance or cross-host behavior.
Run a separate [experiment](experiments.md) for a measured claim.
Keep the script, conditions, result, and meaningful exit code.
State untested platforms and limits.

Transfer a mechanism that meets podbox's constraints.
Do not copy another project's architecture without a matching need.
