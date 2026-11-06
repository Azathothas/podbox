# Handoff

The state at the end of the first cloud session of 2026-11-06, after the
move from podbox to podvm and from one Windows checkout to cloud sessions. Read this file
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

1. The first cloud session ran on 2026-11-06: `sh scripts/cloud-setup.sh`
   exits 0 (zig 0.16.0, Rust 1.99.1, openssh-server). The host is uid 0
   with no `/dev/kvm`; it permits chroot and `mknod`. Saving the host
   measurement is T-1670.
2. T-1669 is `partial`: the pin `1.99` and edition 2024 are on `main`
   (`75ed306`, `5085c14`); `cargo test --workspace` still fails on the
   two device tests of T-1686 where `mknod` is permitted.
3. T-1675 is done: podvm took `pod-term` at `tag = "v0.3.0"` (`75ed306`,
   COORD-004, #197). Change the tag to `v0.4.0` when podcrates publishes
   it. The next hand-offs of podcrates are the trials of `pod-tls` with
   no tokio and `pod-proxy` with the feature `blocking` (T-1676, T-1677).
4. T-1604 stays `partial`: `4c2cb91` makes `run` check the presence of
   the blobs, not their hash, and gate run 42177673037 passed the
   workspace tests; the plant of its Prove remains. Gate run 42179344015
   at `7f87f24` is green in each job.
5. T-1687 is `partial` (COORD-006, #200, to podcrates): the Alpine
   netboot files come from the 3.22.6 archive; a run of
   `podvm-workload` remains.
6. Watch PodUtils/podcrates#1 and COORD-001 (#194), and the import of
   the seven hand-over crates (wave M6): from the import, their files
   here take fixes only.
7. COORD-005 (#198): dispatch podcrates' `ci.yml` and `release.yml` from
   the owner's account when podcrates asks.
8. T-1674 is done: each `.rs` file outside `vendor/` has 500 lines or
   fewer, and `check_rust_file_lines` of the gate holds the limit.
9. The work order of `TODO/PROGRESS.md`: the errand-host cluster
   (T-1643 to T-1653) first, then the scope entries.

## A workflow that a session cannot start

`AGENTS.md`, section 9, item 5, gives the procedure: an issue titled
exactly `[gh action] requested`.

## Questions for the owner

`docs/decisions.md`, "Questions for the owner": the synchronous face of
`pod-proxy` and `pod-tls`, the access of the podcrates session (T-040),
and the source-file size limit.
