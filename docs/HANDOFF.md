# Handoff

The state on 2026-11-14, after the release `0.2.0-beta.18` (issue 368).
`0.2.0-beta.8` (issue 302) is the complete release for the beta testers
(`docs/decisions.md`, "Complete release first"); beta.9 to beta.14 serve
podrunner's kinds and podswarm's V-2 and V-7; beta.15 gives the spare tree
and the fixes of the first beta reports (issues 329 to 336); beta.16 the
eight P0 findings of the external QA report (T-1800 to T-1819); beta.17
the glibc 2.34 object of PodUtils/podrunner#100 (T-1880), T-1816 and
T-1805; beta.18 podcrates `v0.44.0` and the fixes of issues 364 and
PodUtils/podrunner#102. After beta.18, `main` takes podcrates `v0.45.0`
(`4a02d45e`, CHANGELOG "Unreleased"); no release carries it yet.
A fresh
session with no memory reads this file first, then `AGENTS.md`,
`TODO/PROGRESS.md`, `TODO/RESUME.md` and `docs/decisions.md`. Remove this
file when each step below is done.

## Where things are

* The repository is PodUtils/podvm on `main`. No work lives in a local
  worktree or a branch. The branch `archive/podbox-worktree` is an
  archive; never merge or delete it.
* **A cloud session cannot start a workflow run** (`TODO/PROGRESS.md`,
  State). Each run is an issue titled `[gh action] requested` in the form
  of `AGENTS.md`, section 9, item 5. While a request that checks the head
  of `main` is open, push nothing to `main`. The owner's local agent ran
  issue 287 in about 50 minutes, and it reported the run ids.
* The record is in the format of `pod-todo` (T-1672, call C17).
  `podvm-count --set T-NNNN STATUS` writes the status line and the row;
  a done entry needs `## Done` with a date, and a partial entry also puts
  its state under `## Done`. A foreign entry id carries its repository
  (`PodUtils/podrunner@T-096`), or check 35 refuses it. A path of another
  repository in an entry is named in words, not in backticks.
* incus runs on the cloud host (`docs/decisions.md`, "Missing tools"). The
  host has no `/dev/kvm`; the machine tier runs TCG there.

## The QA pass

Experiment 443 (`experiments/443-qa-surface.sh`) drives the CLI on the
namespace rung and on the chroot-denied fixture (`ul`). The first pass
gave 518 PASS and 64 FAIL, and T-1753 to T-1771; the reruns found T-1772
to T-1775. One full run on the published `v0.2.0-beta.14` (2026-11-14) gives 583
PASS, 0 FAIL and 13 INFO, with no merge of earlier runs.

## Open COORD issues

* COORD-041 (#260): `pod-netlab` for T-1662.
* COORD-103 (#350): the seven asks of the Windows guest. podcrates
  `v0.45.0` carries items 1 to 6 (podcrates@T-590 to podcrates@T-595),
  and podvm takes that tag; the warm start (podcrates@T-596) is open.
* COORD-112 (#366): the answer to PodUtils/podrunner#102 (T-1889, T-1890).
* COORD-114 (#369): podvm took podcrates `v0.45.0` in `4a02d45e`.

Each other COORD issue is closed; `TODO/issues.md` gives the state of
each one.

## Next steps, in order

1. Start the session (`AGENTS.md`, section 8).
2. podrunner closed each of its asks to podvm; their answers are closed.
   podcrates read COORD-068 (#290) on PodUtils/podcrates#124, and podvm
   closed it on 2026-11-14. The asks of podswarm that stay are T-1747,
   T-1749 and T-1750 (`TODO/PROGRESS.md`, work order, item 5).
3. A tool of the record (`podvm-gate`, `podvm-count`) finds the repository
   from its own path: in a worktree, build it in the worktree's own
   `target/`, or it reads and writes the main checkout. Commit no
   symbolic link: issue 305 stopped on one.
4. Before an entry is set done, run `cargo test -p podvm`. Do not edit
   `TODO/` while `cargo test --workspace` runs, and do not edit
   `experiments/443-qa-surface.sh` while it runs.
