# Handoff

The state at the close of 2026-11-10. A fresh session with no memory
reads this file first, then `AGENTS.md`, `TODO/PROGRESS.md`,
`TODO/RESUME.md` and `docs/decisions.md`. Remove this file when each
step below is done.

## Where things are

* The repository is PodUtils/podvm on `main`. The tree is clean and
  pushed. No work lives in a local worktree or a branch. The branch
  `archive/podbox-worktree` is an archive; never merge or delete it.
* The newest release is `v0.2.0-beta.2` (2026-11-10), the first with the
  name podvm. The tags `v0.1.0-beta.13` and `v0.2.0-beta.1` have no
  release. A release starts from `release.yml`; dispatch `nightly.yml`
  with `trial=true` on `main` first.
* The last green run of each workflow on 2026-11-10: `gate` 42848168402
  (at `f7287aa`), `secrets` 42842844364, `record-issues` 42843004466,
  `nightly` 42809531320 (the tag `v0.2.0-beta.2`), `release` 42809429283,
  `remote-items` 42637008662 (2026-11-09). Read the runs of the last
  commits first: a red run that the record does not know is the first
  task (`AGENTS.md`, section 8).
* podvm takes 11 crates of PodUtils/podcrates at `v0.12.0` (5e5629a).
  `v0.14.0` is the last podcrates tag and needs no podvm change
  (PodUtils/podcrates#50). The next tag (planned `v0.15.0`) carries the
  `pod-enter` change of podcrates T-155: podvm takes it with T-1716
  (PodUtils/podcrates#52, COORD-027).
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`.
  It has `/dev/fuse` (mode 0600, root): the live FUSE entry of the ladder
  (`experiments/358-ladder-rungs.sh`) is not measured on it yet
  (`docs/limits.md`).
* A cloud session runs `sh scripts/cloud-setup.sh`, puts
  `$HOME/.local/podvm/zig-x86_64-linux-0.16.0` on `PATH`, and builds the
  record tools before the gate (`TODO/RESUME.md`).

## What the session of 2026-11-10 did

* CI: the boundary test of `podvm-release` and the plant of
  `record-issues` were red on `53465ef`; both are fixed (`2148874`).
* Issues: the COORD issues #196, #221, #226, #227 and #228 are closed with
  their answers. 43 of the 46 `reference` issues are closed under the
  owner's ruling "References closed"; #135, #149 and #162 stay open.
  The 16 `scope` issues stay open, because their entries are open.
* Documents and record: `TODO/PROGRESS.md` has one work order; the old
  layers are in `docs/history/progress-before-2026-11-10.txt` and
  `resume-before-2026-11-10.txt`. README, SECURITY, `docs/*` and
  `TODO/RULES.md` name the musl paths, the cloud lane and the rulings of
  2026-11. The record tools write the musl paths (`9659861`).
* podrunner: the owner's design review of PodUtils/podcrates#47 is filed.
  PodUtils/podrunner has 43 issues under the index PodUtils/podrunner#1.
  podvm's part is T-1704 to T-1715 (issues #229 to #240) and clauses in
  eight entries. podcrates recorded its asks as T-160 to T-165
  (PodUtils/podcrates#51); podnet recorded T-229 to T-232
  (PodUtils/podnet#422). Each waits for the owner's DR-1 and DR-2. The
  review itself is in no repository; the owner holds it.

## Open COORD issues

* COORD-001 (#194, to podcrates): the takes of `pod-relay` (T-1678,
  blocked: no release), `pod-services` (T-1679, open) and `pod-todo`
  (T-1680, blocked on T-1672).
* From podcrates to podvm: PodUtils/podcrates#52 (COORD-027, take with
  T-1716) and #53 (COORD-028, the hand-off before the pause; this file
  answers it). PodUtils/podnet#422 asks podvm and podcrates to read it;
  podvm read it.

## Next steps, in order

1. Get the owner's answers to the questions of `docs/decisions.md`
   ("Questions for the owner", Q1 to Q7). Record each answer as a ruling.
2. If the owner resumes podvm (Q2): the work order of
   `TODO/PROGRESS.md`, from item 1 (T-1712 and T-1713).
3. When the podcrates tag with T-155 exists: T-1716.
4. If the owner answers yes to DR-1 and DR-2 (Q1): the podrunner part,
   item 13 of the work order, and the podrunner issues from #1.

## Questions for the owner

Seven, in `docs/decisions.md`, each with a recommendation. The owner
answers them in the next session.
