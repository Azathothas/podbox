# Handoff

The state at the end of the session of 2026-11-06: the move from podbox
to podvm, and from one Windows checkout to cloud sessions. Read this file
first, then `AGENTS.md`, `TODO/PROGRESS.md` and `docs/decisions.md`.
Remove this file when each step below is done.

## Where things are

* The repository is PodUtils/podvm. GitHub redirects Azathothas/podbox.
  Nobody may create a new Azathothas/podbox: that ends the redirects.
* The project, the crates, the binaries, the variables and the flags are
  named podvm (`CHANGELOG.md`, `## Unreleased`, `### Breaking`). Dated
  records in `docs/history/` and `experiments/results/` keep podbox.
* The captured reference trees are gone from the tree. Each one is an
  issue with the label `reference`; `references/README.md` maps them.
* The untracked material of the old checkout is on the branch
  `archive/podbox-worktree` (`docs/history/worktree-archive.md`).
* GitHub hid 41 issue numbers with their author's account
  (`docs/history/github-hidden-issues-2026-11-06.md`). Five open ones are
  reposted with the label `recovered` (#127 to #131).
* The scope of podvm widens to what sand-net does, without root. The
  topics are the issues with the label `scope`, index #193.
* `TODO/github.txt` and `TODO/issues.md` mirror the GitHub numbers from
  #127 on; `.github/workflows/record-issues.yml` checks them each day.
* The newest release is `v0.1.0-beta.12` of podbox. No podvm release yet.

## Next steps, in order

1. The first cloud session: run `sh scripts/cloud-setup.sh`, measure the
   host, and save the result (the entry for the first cloud session in
   `TODO/PROGRESS.md`).
2. Move the toolchain to the pinned stable Rust and edition 2024 before
   the first `pod-*` crate (`docs/decisions.md`, call C6).
3. Watch PodUtils/podcrates#1 (the names that podvm asked for) and
   COORD-001 (#194). Take each `pod-*` crate on the day of its release:
   `adoption/podvm/CRATE.patch` and `.run` at the tag (`AGENTS.md`,
   section 9). Until podcrates imports a podvm file, change it for fixes
   only.
4. podcrates asked for podvm's inventory of the hosts of other parties
   (PodUtils/podcrates#3, T-054 there); COORD-003 answers it. Watch for the
   import of the seven hand-over crates (wave M6 of podcrates): from the
   import, their files here take fixes only.
5. T-1604: `detached_start_returns_before_the_payload_ends` fails in CI
   (`run -d` takes 5 s to detach). It was red on `main` before the
   migration; it keeps the job "workspace and interposer tests" red.
6. The work order of `TODO/PROGRESS.md`: the errand-host cluster
   (T-1643 to T-1653) first, then the scope entries.

## Questions for the owner

`docs/decisions.md`, "Questions for the owner": the synchronous face of
`pod-proxy` and `pod-tls`, the access of the podcrates session (T-040),
and the source-file size limit.
