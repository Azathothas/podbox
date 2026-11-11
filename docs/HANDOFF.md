# Handoff

The state on 2026-11-11, in the session that resumed podvm. A fresh
session with no memory reads this file first, then `AGENTS.md`,
`TODO/PROGRESS.md`, `TODO/RESUME.md` and `docs/decisions.md`. Remove this
file when each step below is done.

## Where things are

* The repository is PodUtils/podvm on `main`. No work lives in a local
  worktree or a branch. The branch `archive/podbox-worktree` is an
  archive; never merge or delete it.
* The owner resumed podvm on 2026-11-11 and answered Q1 to Q7
  (`docs/decisions.md`, rows of 2026-11-11). No question is open.
* The newest release is `v0.2.0-beta.3` (2026-11-11, release run
  42980701550 on c62643d), the beta that podrunner asked for (issue 53 of
  PodUtils/podrunner). A release starts
  from `release.yml`; dispatch `nightly.yml` with `trial=true` on `main`
  first.
* podvm takes 12 crates of PodUtils/podcrates at `v0.21.0` (`ada3699`),
  the last podcrates tag (`pod-incus` of T-1727, PodUtils/podcrates#65).
* PodUtils/podrunner has a lead session since 2026-11-11 (DR-1 and DR-2
  are yes). It writes to podvm by a COORD issue of podrunner titled
  `(to podvm)`; the watcher of the organization reads it.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`.

## Open COORD issues

* COORD-001 (#194, to podcrates): the takes of `pod-relay` (T-1678,
  blocked: no release), `pod-services` (T-1679, open) and `pod-todo`
  (T-1680, blocked on T-1672).
* COORD-024 (#242) and COORD-025 (#243, to podcrates): the take of
  `v0.15.0`, and the answer to PodUtils/podcrates#56.
* COORD-038 (#257, to podcrates): podvm takes `v0.21.0` (T-1727).
* COORD-039 (#258, to podrunner): each container has its own rootfs
  (T-1709).
* COORD-040 (#259, to podcrates): `pod-confine` and `Plan.confine` for
  T-1658. podcrates answered (PodUtils/podcrates#69, COORD-044): it writes
  them for the tag after `v0.22.0`. T-1658 is blocked until that tag.

## Next steps, in order

1. The work order of `TODO/PROGRESS.md`, step 8: the broker of
   [T-1665](../TODO/net.md) (the rewrite half is done), then
   [T-1662](../TODO/net.md), [T-1663](../TODO/net.md),
   [T-1664](../TODO/net.md), the profiles [T-1659](../TODO/cli.md),
   [T-1661](../TODO/cli.md) and [T-1660](../TODO/cli.md), and
   [T-1668](../TODO/gate.md).
2. Done on 2026-11-11 in this order: T-1604, T-0211, T-0204, T-1709,
   T-1728, T-1654, T-1657, T-1666, T-1730; partial: T-1667 (the `gh`
   fallback waits for an installer), T-1665. New open entries: T-1729
   (`runsc`), T-1731 (a dynamic program under the handler).
3. Before an entry is set done, run `cargo test -p podvm`: the test
   `each_listed_gap_names_an_entry_that_is_not_done` reads the record, and
   CI run 387 failed on it.
4. CI: the runners of the organization queued each run for a long time on
   2026-11-11, and each push cancels the run before it. Let one run finish
   between pushes where it can.
