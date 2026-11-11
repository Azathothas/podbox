# Maintenance

The procedures of the repository over time, in the form of the page of the
same name in PodUtils/podcrates (TODO/gate.md T-1685). The rules for code,
tests and the shell are in [development](development.md).

## Commits

Make a commit for a coherent change.
Update its task and live record in the same commit.
Use an imperative subject that names the change.
Use a short body that explains the reason when needed.

Write multiline messages through a file.
Read [shell rules](development.md#shell) before passing prose to a process.
Run the full gate before commit.
Record an interrupted checkpoint honestly when the operator stops the work.

The other rules of git (the branch, the identity, the remote) are in
[git](conventions/git.md).

## Releases

A release starts from `.github/workflows/release.yml`; dispatch
`nightly.yml` with `trial=true` on `main` first ([handoff](HANDOFF.md)).

## Dependencies and vendoring

podvm takes the crates of PodUtils/podcrates by tag (`AGENTS.md`,
section 9). Carried source follows [vendoring](methodology/vendoring.md)
and the [third-party notice](../THIRD_PARTY.md).
