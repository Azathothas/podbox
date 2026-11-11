# Git

## 1. Attribution

Attribute each commit, tag, and release note to the operator.
Do not add a tool or model credit.
Do not add an automatic coauthor trailer.
The [commit hook](../../.githooks/commit-msg) checks this rule.

`.claude/session-start.sh` sets the identity of `AGENTS.md`, section 3, and the hooks path.
Read the configured name and email.
Do not invent an identity.

## 2. Branch and publication

Work on `main`. Push no other branch, except a branch that the owner names.
`AGENTS.md`, section 3, owns that policy.
Agents that work in parallel use local worktrees under `.claude/worktrees/`
on local branches. Their work lands on `main` as normal commits.
Session authorization takes precedence over a generic default.

A podvm session writes to no other repository.
A message to another session of PodUtils is a COORD issue in this
repository (`AGENTS.md`, section 9).
Read [remote operations](../security/remote-ops.md) before a remote change.
Fetch before push. Confirm that the remote is the intended repository.
Push without force. Verify the remote commit and its checks.
`main` has no branch protection since 2026-11-06 ([decisions](../decisions.md)).

## 3. Commits

The commit procedure is in [maintenance](../maintenance.md#commits).

## 4. Contents

Do not commit credentials, local environment data, build output, or caches.
Keep captured reference source and required notices.
Check tracked files; an ignore rule does not remove a tracked file.
Save measurement scripts and their results.

## 5. History

Do not rewrite published history or force push.
Keep published commits intact. Do not delete a tag or a release.
Never merge or delete the branch `archive/podbox-worktree`.
A credential incident requires the operator's action under
[secret rules](../security/secrets.md).

## 6. CI

Do not add a CI skip directive to a commit message.
Wait for checks on the pushed commit.
A green run on an earlier commit does not prove the new commit.
Report skipped jobs as skipped.
Read the local and hosted toolchain conditions before comparing results.
