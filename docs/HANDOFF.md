# Handoff

The state on 2026-11-10. Read this
file first, then `AGENTS.md`, `TODO/PROGRESS.md` and `docs/decisions.md`.
Remove this file when each step below is done.

## Where things are

* The repository is PodUtils/podvm. GitHub redirects Azathothas/podbox.
  Nobody may create a new Azathothas/podbox: that ends the redirects.
* The newest release is `v0.2.0-beta.2` (2026-11-10), the first with the
  name podvm. The tags `v0.1.0-beta.13` and `v0.2.0-beta.1` have no
  release: their builds failed. A release starts from `release.yml`;
  dispatch `nightly.yml` with `trial=true` on main first. The owner allows
  a beta when it fits (2026-11-09); no crates.io release.
* podvm takes the crates of PodUtils/podcrates by git tag. podvm uses
  each of the 11 crates at v0.12.0 (5e5629a, COORD-020 #226).
* KVM is free (the owner, 2026-11-09). The cloud host has no `/dev/kvm`.
* A session dispatches the workflows of this repository itself
  (`AGENTS.md`, section 9, item 5).
* A cloud session runs `sh scripts/cloud-setup.sh` and puts
  `$HOME/.local/podvm/zig-x86_64-linux-0.16.0` on `PATH` before a cargo
  build of a crate with C code.

## Next steps, in order

1. The work order of `TODO/PROGRESS.md`: T-1652, T-1651, T-1655 (the last
   clause of T-1653), T-1703.
2. T-1694 to T-1696 (the asks of cfb) wait for the work of podvm itself
   (the owner, 2026-11-09, "Priority").

## Questions for the owner

None open. `docs/decisions.md` has the calls C12 and C13 of 2026-11-09.
