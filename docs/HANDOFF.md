# Handoff

The state on 2026-11-13, in the session that worked the defects of the QA
pass (experiment 443) for the release `0.2.0-beta.8`, the complete release
for the beta testers (`docs/decisions.md`, "Complete release first"). A
fresh session with no memory reads this file first, then `AGENTS.md`,
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
to T-1775. The merged capture gives 578 PASS and 5 FAIL. The five:

* T-1760 (partial): `tty` on the namespace rung needs a bind of
  `/dev/pts`, the half of COORD-073 (#296) that podcrates `v0.37.0` does
  not hold.
* T-1770 (blocked): COORD-075 (#298) asks for three fixes of the knob
  flags of `pod-netlab`.
* T-1774 (open, P3): busybox `head -c` of `/dev/urandom` on the userland
  rung.

## Other open COORD issues

* COORD-068 (#290): the asks of podswarm are T-1745 to T-1752.
* COORD-069 (#291): the reader of the probe report for T-1746.
* COORD-071 (#294): an overlay root for T-1749.
* COORD-076 (#299): podvm's take of `v0.37.0`.
* COORD-041 (#260): `pod-netlab` for T-1662.
* COORD-001 (#194): the crates that podvm hands over and takes.

## Next steps, in order

1. Start the session (`AGENTS.md`, section 8).
2. The release `0.2.0-beta.8` is the commit "release: version
   0.2.0-beta.8". Its gate, `secrets.yml`, `arm.yml`, the nightly trial
   and `release.yml` run through one `[gh action] requested` issue in the
   form of issue 287, where the session cannot dispatch them itself. Push
   nothing to `main` while that request is open. When the release run
   passes, record its run ids in `TODO/PROGRESS.md`.
3. Announce the release by COORD issues to podrunner and podcrates; the
   beta testers get the README's Install and "Report a problem".
4. Take each podcrates tag that answers COORD-073 (/dev/pts) or COORD-075,
   then run the rows that the entries name. Close COORD-076 (#299) when
   podcrates records the take of `v0.37.0`.
5. Before an entry is set done, run `cargo test -p podvm`. Do not edit
   `TODO/` while `cargo test --workspace` runs: the record tests read the
   files on disk. Do not edit `experiments/443-qa-surface.sh` while it
   runs: `sh` reads each later section from its byte offset.
6. The interposer: a new entry point needs its name in `interpose.map`.
   It exports the lookups of users, groups and hosts (T-1753), and a
   constructor keeps podvm's variables for a process that clears its
   environment (T-1756).
