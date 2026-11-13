# Handoff

The state on 2026-11-13, at the end of the session that released
`v0.2.0-beta.7` (T-1743) and took podcrates `v0.35.0`. A fresh session
with no memory reads this file first, then `AGENTS.md`,
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
* Issue 292 asks for the gate, `secrets.yml` and `arm.yml` on the head of
  `main` (the commit that maps issue 292). No run of CI has checked the
  commits after `2edace91` yet: `podvm lab` (T-1662), the open flags on
  aarch64 (T-1744), the foreground payload (T-1748) and podcrates
  `v0.35.0`. Each passed `cargo test --workspace`, clippy, fmt and the gate
  on the cloud host.
* The record is in the format of `pod-todo` (T-1672, call C17).
  `podvm-count --set T-NNNN STATUS` writes the status line and the row;
  a done entry needs `## Done` with a date. A foreign entry id carries its
  repository (`PodUtils/podrunner@T-096`, `podcrates@T-207`), or check 35
  refuses it.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`;
  the machine tier runs TCG there.

## Open COORD issues

Each waits for a peer; podvm closes it when the peer answers.

* COORD-067 (#289, to podcrates and podrunner): T-1743, released in
  beta.7. Close it when podrunner pins beta.7.
* COORD-068 (#290, to podcrates): the asks of podswarm are T-1745 to
  T-1752. Close it when podswarm or podcrates reads it.
* COORD-069 (#291, to podcrates): the reader of the probe report for
  T-1746. podcrates answered (PodUtils/podcrates#128): podcrates@T-485 in
  the next tag. Close it when podvm takes that tag.
* COORD-070 (#293, to podcrates): the take of `v0.35.0`, the answer to
  PodUtils/podcrates#127. Close it when podcrates records the take.
* COORD-071 (#294, to podcrates): an overlay root in `pod-enter`'s
  namespace rung for T-1749. Close it when a tag holds it.
* COORD-041 (#260): `pod-netlab` for T-1662. Close it when podcrates
  releases the guest parts (podcrates@T-345 to podcrates@T-347).
* COORD-001 (#194, to podcrates): the crates that podvm hands over and
  takes.
* #239: `podvm probe --json` against the schema of pod-hostprobe (T-1714,
  blocked on podcrates@T-107).
* The trackers #271 (cfb's asks: 6 of 9 done) and #272 (the CI plan
  items: 2 of 3 done) stay open for their blocked items.

## Next steps, in order

1. Start the session (`AGENTS.md`, section 8). Read the answer on issue
   292. Its `arm.yml` run 43351820031 passed (T-1744 is done with it), and
   secrets passed; the gate run 43351811176 was queued at the end of the
   session. On its success, close issue 292 with the run ids. On a
   failure, fix it, close issue 292 and open a new request.
2. The work order of `TODO/PROGRESS.md`: T-1667 (partial), T-1662
   (partial: the guest parts wait for podcrates@T-345 to podcrates@T-347),
   then T-1722 (needs incus, which the cloud host lacks), T-1406.
3. Blocked: T-1663 and T-1664 (they run in T-1662's lab guest), T-1714
   (podcrates@T-107), T-1746 (podcrates@T-485).
4. After podvm's own order: the asks of podswarm, T-1747 (needs a host
   with a delegated cgroup2 subtree), T-1749 (partial; COORD-071 asks
   podcrates for the overlay root), T-1750 and T-1751 (P3). T-1745, T-1748
   and T-1752 are done.
5. Before an entry is set done, run `cargo test -p podvm`: the test
   `each_listed_gap_names_an_entry_that_is_not_done` reads the record.
   The record tests of `podvm-todo` read the files on disk, so do not edit
   `TODO/` while `cargo test --workspace` runs.
6. The interposer: a new entry point needs its name in `interpose.map`
   (`podvm-interpose-build` compares the exports). Its `build.rs` names the
   map, so cargo links the objects again when the map changes (T-1743).
   Each open flag comes from `oflags.rs` (T-1744).
