# Git

## 1. Attribution

Attribute each commit, tag, and release note to the operator.
Do not add a tool or model credit.
Do not add an automatic coauthor trailer.
The [commit hook](../../.githooks/commit-msg) checks this rule.

Read the configured name and email.
Do not invent an identity.
Use the existing repository identity for commits.

## 2. Branch and publication

Work on `main`.
The repository push policy permits authorized publication to its configured `origin`.
[TODO rules](../../TODO/RULES.md) own that policy.
Session authorization takes precedence over a generic default.

Every other repository is read-only.
Read [remote operations](../security/remote-ops.md) before a remote change.
Fetch before push. Confirm that the remote is the intended repository.
Push without force. Verify the remote commit and its checks.

If protection refuses a direct push, inspect the current protection.
Use the approved temporary publish route only when necessary.
Integrate it into `main` and remove it after verification.
Do not continue work on a retained publish branch.

## 3. Commits

Make a commit for a coherent change.
Update its task and live record in the same commit.
Use an imperative subject that names the change.
Use a short body that explains the reason when needed.

Write multiline messages through a file.
Read [shell rules](shell.md) before passing prose to a process.
Run the full gate before commit.
Record an interrupted checkpoint honestly when the operator stops the work.

## 4. Contents

Do not commit credentials, local environment data, build output, or caches.
Keep captured reference source and required notices.
Check tracked files; an ignore rule does not remove a tracked file.
Save measurement scripts and their results.

## 5. History

Do not rewrite published history or force push.
Keep published commits intact.
A credential incident requires the operator's action under
[secret rules](../security/secrets.md).

## 6. CI

Do not add a CI skip directive to a commit message.
Wait for checks on the pushed commit.
A green run on an earlier commit does not prove the new commit.
Report skipped jobs as skipped.
Read the local and hosted toolchain conditions before comparing results.
