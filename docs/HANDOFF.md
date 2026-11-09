# Handoff

The state on 2026-11-09, after the restart of the podvm session. Read this
file first, then `AGENTS.md`, `TODO/PROGRESS.md` and `docs/decisions.md`.
Remove this file when each step below is done.

## Where things are

* The repository is PodUtils/podvm. GitHub redirects Azathothas/podbox.
  Nobody may create a new Azathothas/podbox: that ends the redirects.
* The newest release is `v0.1.0-beta.13`, the last podbox release. The
  owner allows a beta when it fits (2026-11-09); no crates.io release.
* podvm takes the crates of PodUtils/podcrates by git tag. podcrates
  v0.7.0 is out (PodUtils/podcrates#37). podvm took `pod-hostprobe` at
  `8899c2e`.
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`.
* A session dispatches the workflows of this repository itself
  (`AGENTS.md`, section 9, item 5).
* A cloud session runs `sh scripts/cloud-setup.sh` and puts
  `$HOME/.local/podvm/zig-x86_64-linux-0.16.0` on `PATH` before a cargo
  build of a crate with C code.

## Next steps, in order

1. T-1687: one `podvm-workload` run whose fetch step reports the pinned
   sum. On 2026-11-09 `public.ecr.aws` answered the pull with HTTP 429
   twice; `mirror.gcr.io` served Docker Hub images. Close issue 200 with
   the run.
2. Done on 2026-11-09: the take of podcrates v0.7.0 (35df7f7, COORD-012
   #218), the asks of PodUtils/podcrates#39 (67feade), and T-1688 to
   T-1693 (issues 212 to 217, closed).
3. The work order of `TODO/PROGRESS.md`: T-1648, then the errand-host
   cluster T-1643 to T-1653.
4. Each M6 import of podcrates (pod-guest, pod-enter, pod-supervise,
   pod-oci, pod-extract, pod-rootfs): apply its patch, run its `.run`,
   commit, COORD issue. `pod-supervise` is taken (989f35b). `podvm-enter`
   and `podvm-windows` are frozen for `pod-enter` and `pod-guest`
   (podcrates v0.9.0): each fix of them is a COORD issue `(to podcrates)`.

## Questions for the owner

None open. `docs/decisions.md` has the calls C12 and C13 of 2026-11-09.
