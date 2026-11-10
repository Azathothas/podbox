# Reference study

## Source and scope

Capture the commit before you remove Git metadata.
Read the license before you copy source.
A repository is read-only unless the operator explicitly names it as a write target.
For podvm, other repositories remain read-only.

Use the repository fetcher for a remote capture, into a directory outside
the repository. The fetcher refuses a directory that an ignore rule of the
repository covers.

```sh
sh scripts/common/mine-repo.sh OWNER/REPO --out "${TMPDIR:-/tmp}/references"
```

For a local study, read the current tree and record its Git state.
Record the commit read and the cited lines in the reference issue
([Corpus](#corpus)). State every trim and the reading depth.
Do not run another project's setup or gate when the task authorizes reading only.

## Code passes

Read the source yourself.
Do not delegate reference reading to another agent.

1. Read the project shape and the problem it solves.
2. Inspect the construction at exact files and lines.
3. Inspect the failure and resource paths that matter to podvm.
4. Compare each candidate mechanism with podvm's existing implementation.

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

Since 2026-11-06 the corpus is not kept on `main` (the owner's ruling, [decisions](../decisions.md)).
Each studied repository is tracked in a GitHub issue with the label
`reference`. The issue names the upstream, the commit read, the licence
determination, what podvm took from it, and the entries that cite it.
Read a source at its upstream, at the commit the issue names.

The 46 trees captured before that date stay in the history of `main`.
`references/MANIFEST.tsv` lists each of their files with its line count, at
the commit named in its header, and `references/README.md` maps each tree to
its issue. The record gate resolves a citation into `references/` against the
manifest, so the existing citations stay checked. Read a captured file with
`git show COMMIT:PATH`, the commit from the manifest header.

Do not capture a tree into `references/` again. Cite a new source by its
upstream permalink at the commit read, and add its facts to the issue.

## Findings

State what the study did not establish before recommendations.
Give each transferred mechanism a source, reason, and task.
Check whether podvm already implements it.
Use one of these outcomes:

| Outcome | Meaning |
| --- | --- |
| adopt | A named mechanism enters a named podvm task |
| confirms | Podvm already uses the mechanism |
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

Transfer a mechanism that meets podvm's constraints.
Do not copy another project's architecture without a matching need.
