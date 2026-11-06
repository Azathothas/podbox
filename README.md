# archive/podbox-worktree

This branch is an archive. It is not merged into `main` and it has no common
history with `main`. Nothing on it builds or runs.

It holds the untracked working material of the old podbox checkout
(`C:\Users\AjamX\Downloads\podbox`, commit `452d792760abec03caa5c5c905be8d7efe6d4de5`),
copied on 2026-11-06 when the project became podvm and moved to
`D:\_WORK_\PodUtils\podvm` and to cloud sessions. The old checkout was
deleted after the copy.

Every file is at its original path. A citation on `main` such as
`refactor/recon-c.md:177` or `.dev/t0708-drive.sh` reads here:

```sh
git fetch origin archive/podbox-worktree
git show origin/archive/podbox-worktree:refactor/recon-c.md | sed -n 177p
```

## Contents

| Path | What | Why it is kept |
| --- | --- | --- |
| `refactor/` | The shell-retirement audit and plan of 2026-09-30 to 2026-10-01 (46 files) | The record on `main` cites it on about 204 lines; T-1607 holds 29 plan rows that live only here. It was never tracked on `main`, because its own dead citations turn the record gate red. |
| `.dev/`, `.tmp/` scripts | 110 lane job scripts (`job-*.sh` and others) | The recipes behind lane proofs. `.dev/t0708-drive.sh` and `.tmp/job-ci-lint.sh` are cited on `main`. |
| `.dev/`, `.tmp/` text | Proof logs, return codes, CI logs, review text (`.log .txt .json .md .out .err .rc .tsv .csv`) | Raw evidence behind closed entries. `main` keeps the summaries under `experiments/results/`. |
| `experiments/results/sweep163/*.log` | Three result logs | The unanchored `*.log` rule hid them. `main` tracks them from the same date. |
| `github-2026-11-06/` | Snapshot of the GitHub side on 2026-11-06 | See below. |

Left out on purpose: build output (`target/`), the CodeGraph index, the
experiment scratch directories `experiments/.*/`, built binaries, kernels,
third-party source tarballs and trees, and a 169 MB debug dump. Each one can be
built again or downloaded again.

Already missing on the old disk, so not here: `.tmp/artifacts-mutate/mutation-epipe.txt`,
`.tmp/diag390/`, `.tmp/probe-build.sh`, `.tmp/podbox-kvm/podbox`,
`.dev/build-state.json`.

## github-2026-11-06/

GitHub hid 41 issue and pull request numbers of PodUtils/podvm (formerly
Azathothas/podbox) together with their author's account, `talaria0101`. They
answer 404, not 410, so they are hidden and not deleted.

| File | What |
| --- | --- |
| `recovered-issues.md`, `recovered-issues.json` | What came back: 20 issues and 2 pull requests exact (from the issue events API), 5 issues reconstructed from mentions, 14 numbers with no trace, and the map to TODO entries |
| `issues.json`, `pulls.json` | Every issue and pull request that still resolved on 2026-11-06, with bodies and comments, in case GitHub hides more |
| `issue-114.md` | The body of the errand-host dossier, #114 |

## Integrity

`MANIFEST.sha256` lists the SHA-256 of every other file on this branch.

```sh
sha256sum -c --quiet MANIFEST.sha256
```

Some files name paths on the operator's Windows host. They are evidence and
are kept as they were written.
