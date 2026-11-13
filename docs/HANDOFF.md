# Handoff

The state on 2026-11-13, in the session that took podcrates `v0.34.0` and
fixed T-1743. A fresh session with no memory reads this file first, then
`AGENTS.md`, `TODO/PROGRESS.md`, `TODO/RESUME.md` and `docs/decisions.md`.
Remove this file when each step below is done.

## Where things are

* The repository is PodUtils/podvm on `main`. No work lives in a local
  worktree or a branch. The branch `archive/podbox-worktree` is an
  archive; never merge or delete it.
* **A cloud session cannot start a workflow run** (`TODO/PROGRESS.md`,
  State). Each run is an issue titled `[gh action] requested` in the form
  of `AGENTS.md`, section 9, item 5. While a request that checks the head
  of `main` is open, push nothing to `main`.
* The newest release is `v0.2.0-beta.6`. The request for
  `v0.2.0-beta.7` (the gate, `arm.yml`, the nightly trial and the release,
  in one block, on `2edace91`) is issue 287; read its answer first. beta.7
  carries T-1743 for PodUtils/podrunner@T-096. The commit that maps
  issues 287 to 290 waits on the local `main` until issue 287 has run.
* podvm takes the crates of PodUtils/podcrates at `v0.34.0`.
* The record is in the format of `pod-todo` (T-1672, call C17).
  `podvm-count --set T-NNNN STATUS` writes the status line and the row;
  a done entry needs `## Done` with a date. A foreign entry id carries its
  repository (`PodUtils/podrunner@T-096`, `podcrates@T-207`), or check 35
  refuses it.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`;
  the machine tier runs TCG there.

## Open COORD issues

Each waits for a peer; podvm closes it when the peer answers.

* COORD-066 (#288, to podcrates): the take of `v0.34.0`. Close it when
  PodUtils/podcrates#104 records it.
* COORD-067 (#289, to podcrates and podrunner): T-1743. Write the release
  of beta.7 into it; close it when podrunner pins beta.7.
* COORD-068 (#290, to podcrates): the asks of podswarm are T-1745 to
  T-1752. Close it when podswarm or podcrates reads it.
* COORD-041 (#260): `pod-netlab` for T-1662. The offline part is in
  `v0.33.0`; T-1662 is open for the host side. Close it when podcrates
  releases the guest parts (podcrates@T-345 to podcrates@T-347).
* COORD-001 (#194, to podcrates): the crates that podvm hands over and
  takes.
* #239: `podvm probe --json` against the schema of pod-hostprobe (T-1714,
  blocked on podcrates@T-107).
* The trackers #271 (cfb's asks: 6 of 9 done) and #272 (the CI plan
  items: 2 of 3 done) stay open for their blocked items.

## Next steps, in order

1. Start the session (`AGENTS.md`, section 8). Read the answer on
   issue 287: on success, record the release (the run ids in
   `TODO/PROGRESS.md`), push the local commit, amend COORD-067 (#289) with
   the release, and close issue 287. On a failure, fix it, close issue 287
   and open a new request.
2. The work order of `TODO/PROGRESS.md`: T-1667 (partial), T-1662 (open:
   `podvm lab` on the host with `pod-netlab`'s offline part; the first
   clause of its Prove), then T-1722, T-1744 (its proof needs `arm.yml`),
   T-1406.
3. Blocked: T-1663 and T-1664 (they run in T-1662's lab guest), T-1714
   (podcrates@T-107).
4. After podvm's own order: the podrunner part, then the asks of podswarm
   (T-1746 and T-1748 first, P2).
5. Before an entry is set done, run `cargo test -p podvm`: the test
   `each_listed_gap_names_an_entry_that_is_not_done` reads the record.
   The record tests of `podvm-todo` read the files on disk, so do not edit
   `TODO/` while `cargo test --workspace` runs.
6. The interposer: a new entry point needs its name in `interpose.map`
   (`podvm-interpose-build` compares the exports). Its `build.rs` names the
   map, so cargo links the objects again when the map changes (T-1743).
