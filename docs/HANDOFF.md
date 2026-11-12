# Handoff

The state on 2026-11-12, at the end of the session that resumed podvm. A fresh
session with no memory reads this file first, then `AGENTS.md`,
`TODO/PROGRESS.md`, `TODO/RESUME.md` and `docs/decisions.md`. Remove this
file when each step below is done.

## Where things are

* The repository is PodUtils/podvm on `main`. No work lives in a local
  worktree or a branch. The branch `archive/podbox-worktree` is an
  archive; never merge or delete it.
* The owner resumed podvm on 2026-11-11 and answered Q1 to Q7
  (`docs/decisions.md`, rows of 2026-11-11). No question is open. The
  owner closed the session of 2026-11-12 for a restart in a fresh one.
* The newest release is `v0.2.0-beta.6` (2026-11-12, release run
  43241608767 on 860d6294; 42 assets with sums and sigstore bundles; its
  `podvm-aarch64` embeds the two aarch64 interposer objects). A release
  starts from `release.yml`, which needs a successful gate run on its
  commit; dispatch `nightly.yml` with `trial=true` on `main` first.
* Commit signing is off for this repository (`.claude/session-start.sh`,
  `docs/decisions.md`, "Commit signing"), so the stop hook of a cloud
  session asks for no rewrite of the author.
* podvm takes the crates of PodUtils/podcrates at `v0.32.0`:
  `pod-confine` and `Plan.confine` (T-1658), and the hosts and module sums
  of `pod-services` (T-1679, at `v0.24.0`).
* **The record is in the format of `pod-todo` since 2026-11-11** (T-1672,
  call C17). An entry is `# T-NNNN: Title`, the header fields
  `**Name:** value` (with `**Milestone:** none`), and the sections
  `## Problem` to `## Prove`, then `## Blocker`, `## Correction` and
  `## Done`. A row of `TODO/INDEX.md` has seven cells. `podvm-count --set
  T-NNNN STATUS` writes the `**Status:**` line and the row; a done entry
  needs `## Done` with a date. `podvm-gate` checks the record through a
  view of the old layout (`crates/podvm-gate/src/gate/legacy.rs`), and
  check 35 runs `podvm-todo check` (pod-todo's reader), which holds the
  record checks of form since T-1680.
* PodUtils/podrunner has a lead session since 2026-11-11. It writes to
  podvm by a COORD issue titled `(to podvm)`; the watcher reads it.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`;
  the machine tier runs TCG there.

## Open COORD issues

Each waits for a peer; podvm closes it when the peer answers.

* COORD-065 (#286, to podrunner): `v0.2.0-beta.6` carries the aarch64
  interposer for PodUtils/podrunner#52. Close it when podrunner pins the
  beta (PodUtils/podrunner#52 then closes), or act on what fails.
* COORD-064 (#285, to podcrates): podvm's take of `v0.32.0` and the trial
  of `pod-relay` (`15485d1b`). Close it when PodUtils/podcrates#93 closes.
* COORD-057 (#278), COORD-047 (#266), COORD-045 (#264): takes of
  `v0.28.0`, `v0.25.0` and `v0.24.0`; each answers a podcrates issue that
  is closed. Check them and close them.
* COORD-052 (#273, beta.5) and the trackers #271 (cfb's asks) and #272
  (the CI plan items): read them against the record before you close.
* COORD-041 (#260): `pod-netlab` for T-1662 (T-1663 and T-1664 wait with
  it). PodUtils/podcrates#93 says podcrates@T-207 (`pod-netlab`) is in
  work; its offline part is on podcrates `main` for the next tag.
* COORD-040 (#259): `pod-confine` for T-1658; T-1658 is done. Check and
  close.
* COORD-001 (#194, to podcrates): the crates that podvm hands over and
  takes. T-1678 (`pod-relay`) is done now.
* #239: `podvm probe --json` against the schema of pod-hostprobe (T-1714,
  blocked on podcrates@T-107).

## Next steps, in order

1. Start the session (`AGENTS.md`, section 8): the watcher of the
   organization, the last CI runs, `podvm-gate`. Read new messages first:
   podrunner may answer #286, podcrates may tag `v0.33.0` (`pod-netlab`).
2. No entry is open. The partial entries, with their remaining clauses in
   `TODO/`: T-1722 (incus: the ladder selects incus by configuration,
   `.EnteredRung` names it, `logs -f` and `inspect`), T-1667 (the doctor's
   check of labs in a guest waits for the network entries), T-1406
   (live-relay pairing; podcrates@T-330 plans podvm commands that make,
   check and stop a pair), T-1740 (a cheaper syscall tier), T-1350 (the
   wrapper of the parked Windows lane).
3. Blocked on podcrates: T-1662, T-1663, T-1664 (`pod-netlab`), T-1714
   (the schema of podcrates@T-107).
4. Done on 2026-11-12: T-1721 (aarch64 interposer; `arm.yml`), T-1741 and
   T-1742 (podcrates `v0.31.0` fixes found on arm64), T-1729 (runsc tier,
   experiment 436), T-1723 (9p volumes, `-t`, microvm measured: the guest
   keeps `-M pc`; experiments 412 and 440 in `kvm.yml`), T-1678
   (`pod-relay`), and the release `v0.2.0-beta.6`.
5. Before an entry is set done, run `cargo test -p podvm`: the test
   `each_listed_gap_names_an_entry_that_is_not_done` reads the record.
   pod-todo's reader reads the files on disk: an untracked directory can
   hide a citation that a clone of CI refuses (CI run 400).
6. CI: each push cancels the run before it. `release.yml` needs a
   successful gate run on its commit: do not push between the gate and
   the release. A crate of podvm that runs in a guest must build for
   aarch64 too (`c_char` is `u8` there): `cargo check --target
   aarch64-unknown-linux-musl -p podvm` before a push that touches one.
