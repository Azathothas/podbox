# Handoff

The state on 2026-11-13, after the release `0.2.0-beta.10` (issue 308).
`0.2.0-beta.8` (issue 302) is the complete release for the beta testers
(`docs/decisions.md`, "Complete release first"); beta.9 and beta.10 serve
podrunner's kinds. A fresh
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
to T-1775. The merged capture gives 583 PASS and 0 FAIL: the reruns
took the fixes of podcrates `v0.38.0` (T-1760, T-1770) and of T-1774 (the
host's devices on the userland rung under a user namespace).

## Other open COORD issues

* COORD-068 (#290): the asks of podswarm are T-1745 to T-1752.
* COORD-069 (#291): the reader of the probe report for T-1746.
* COORD-071 (#294): an overlay root for T-1749.
* COORD-076 (#299): podvm's take of `v0.37.0`.
* COORD-041 (#260): `pod-netlab` for T-1662.
* COORD-001 (#194): the crates that podvm hands over and takes.

## Next steps, in order

1. Start the session (`AGENTS.md`, section 8).
2. podrunner takes `0.2.0-beta.10` for its `microvm` and `incus` kinds:
   close COORD-077 (#303) and COORD-079 (#307) when it says so.
3. Close COORD-080 (#309) when podcrates records the take of `v0.39.0`.
4. The rest of T-1751 (podswarm's shield `guest`: no host file in the
   guest).
5. A tool of the record (`podvm-gate`, `podvm-count`) finds the repository
   from its own path: in a worktree, build it in the worktree's own
   `target/`, or it reads and writes the main checkout. Commit no
   symbolic link: issue 305 stopped on one.
6. Before an entry is set done, run `cargo test -p podvm`. Do not edit
   `TODO/` while `cargo test --workspace` runs, and do not edit
   `experiments/443-qa-surface.sh` while it runs.
