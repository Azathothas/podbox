# The untracked working material, archived on 2026-11-06

Until 2026-11-06 the project was worked in one Windows checkout,
`C:\Users\AjamX\Downloads\podbox`. Some files there were never tracked on
`main`, and the record cites several of them. When the project became podvm
and moved to cloud sessions, that checkout was deleted. Its untracked material
of value is on the branch `archive/podbox-worktree`, at the original paths.
The branch has no common history with `main` and is never merged.

| Path cited on `main` | What it is |
| --- | --- |
| `refactor/` | The shell-retirement audit and plan of 2026-09-30 to 2026-10-01. The record cites it on about 204 lines. T-1607 holds the plan rows that are filed nowhere else. |
| `.dev/*.sh`, `.dev/jobs/*.sh`, `.tmp/job-*.sh` | The lane job scripts behind lane proofs, for example `.dev/t0708-drive.sh` (T-0708) and `.tmp/job-ci-lint.sh` |
| `.dev/**`, `.tmp/**` text files | Proof logs, return codes and CI logs behind closed entries |
| `github-2026-11-06/` | The recovered hidden issues and a snapshot of every issue and pull request, see [the hidden issues](github-hidden-issues-2026-11-06.md) |

Read a cited file from the branch:

```sh
git fetch origin archive/podbox-worktree
git show origin/archive/podbox-worktree:refactor/recon-c.md | sed -n 177p
```

The branch README lists the contents, and `MANIFEST.sha256` holds the
SHA-256 of each file. A second copy is a tarball outside the repository,
`D:\_WORK_\PodUtils\archive\podbox-worktree-2026-11-06.tar.gz`, on the
operator's host.

These cited files were already missing from the old checkout and are lost:
`.tmp/artifacts-mutate/mutation-epipe.txt`, `.tmp/diag390/`,
`.tmp/probe-build.sh`, `.tmp/podbox-kvm/podbox` and `.dev/build-state.json`.

`refactor/` stays out of `main` for the reason that kept it untracked: its own
files cite scripts and experiments that no longer exist, and the record gate
resolves the citations of every tracked Markdown file.
