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
* The newest release is `v0.2.0-beta.2` (2026-11-10). A release starts
  from `release.yml`; dispatch `nightly.yml` with `trial=true` on `main`
  first.
* podvm takes 11 crates of PodUtils/podcrates at `v0.15.0` (`27a4ccd`),
  the last podcrates tag, with the `pod-enter` change of podcrates T-155
  (T-1716; PodUtils/podcrates#52, #54).
* PodUtils/podrunner has a lead session since 2026-11-11 (DR-1 and DR-2
  are yes). It writes to podvm by a COORD issue of podrunner titled
  `(to podvm)`; the watcher of the organization reads it.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`.

## Open COORD issues

* COORD-001 (#194, to podcrates): the takes of `pod-relay` (T-1678,
  blocked: no release), `pod-services` (T-1679, open) and `pod-todo`
  (T-1680, blocked on T-1672).
* COORD-024 (#242, to podcrates): the take of `v0.15.0` (T-1716).
* COORD-025 (#243, to podcrates): the answer to PodUtils/podcrates#56
  (COORD-031): T-1656 moves up, and the incus entries T-1717 and T-1718.
  COORD-023 (#241) is closed.
* From podcrates to podvm: PodUtils/podcrates#54 (COORD-029): podvm's
  part is T-1716, done. PodUtils/podcrates#56 (COORD-031): DR-5 and incus,
  recorded.

## Next steps, in order

1. The work order of `TODO/PROGRESS.md`, from item 1 (T-1708 and
   T-1715, the needs of podrunner first). T-1716, T-1712 and T-1713 are
   done on 2026-11-11.
