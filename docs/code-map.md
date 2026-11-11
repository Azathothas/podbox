# Code map

## Crates of this tree

podvm takes 11 shared crates from PodUtils/podcrates at the tag `v0.16.0`
(podcrates commit `60a675c`; T-1708 took it).
A local crate gives podvm's names, paths and settings to its `pod-*` crate
and keeps the parts that only podvm needs.
The `Cargo.toml` of each crate names its `pod-*` dependencies.

| Directory | Owns | Takes from podcrates |
| --- | --- | --- |
| `crates/podvm` | The binary: dispatch, arguments, parity table, manual, `run` and the lifecycle verbs, `images`, `doctor`, `system`, the tier flag, `remote`, `machine ssh`, `windows` | through the crates below |
| `crates/podvm-probe` | podvm's settings for the probes, the report, the exit codes, the JSON writer, the non-goals | `pod-hostprobe` (probes, rung selection, machine legs) |
| `crates/podvm-image` | The store, its layout and records, credentials paths, platform choice, space and exec checks, `pull` | `pod-oci` (registry client), `pod-proxy`, `pod-tls`, `pod-supervise` (store lock), `pod-hostprobe`, `pod-core` |
| `crates/podvm-extract` | The layers of a manifest as blob paths, the rootfs place in the store | `pod-extract` (layers, whiteouts, containment, sidecar) |
| `crates/podvm-complete` | podvm's flags and file names for completion, the host CA bundle, the mirror probe, the report | `pod-rootfs` (devices, accounts, resolver, package configuration, CA data) |
| `crates/podvm-enter` | podvm's variable names, cache marker and memfd label; docker's exit codes | `pod-enter` (chroot, namespace and userland entry, launch ladder, FUSE server, ELF reader) |
| `crates/podvm-supervise` | The store and names of the container table, the fields of a podvm record, the launcher entry of a payload, the ownership memo | `pod-supervise` (launcher, pidfd, control socket, logs, lifecycle verbs) |
| `crates/podvm-windows` | podvm's names and places for the Windows and DOS guest driver, the base caches, docker's exit range | `pod-guest` (boot plan, overlay, FAT16 mailbox, agent, QMP, bounded stop) |
| `crates/podvm-ssh` | SSH transport, relay frames, multiplexing, server, interactive session; the `node`, `operator`, `proxy` and `shell` helpers | `pod-tls`, `pod-proxy`, `pod-term` (line discipline) |
| `crates/podvm-interpose` | The `LD_PRELOAD` interposer, one object for glibc and one for musl; outside the workspace | none |
| `crates/podvm-gate` | `podvm-gate`, `podvm-count`, `podvm-plant`, `podvm-dev`, `podvm-smoke`, `podvm-interpose-build`, `podvm-libc-interpose` | none |
| `crates/podvm-buildstate` | `podvm-buildstate` (build freshness), `podvm-release-licenses` (licence inventory) | none |
| `crates/podvm-release` | `podvm-verify`, `podvm-size`, `podvm-prove-t0211`, `document-state`, `release-notes`, `podvm-reconcile`, `podvm-publish` | none |
| `crates/podvm-measure` | `podvm-parity` (guest driver), `podvm-workload` (spread measurement) | none |

`pod-core` (failure type, dial traits) comes in under most of the crates above.
COORD-001 (#194) and PodUtils/podcrates#1 list the hand-over of each crate.
A file that podcrates imported and podvm has not taken is frozen
(`AGENTS.md`, section 9, item 4).

## Tooling

| File | Purpose |
| --- | --- |
| `scripts/cloud-setup.sh` | zig, the Rust toolchain and the host report for a cloud session |
| `podvm-dev` `session`, `build`, `status`, `check` | Host report, foreground build, build state, complete Linux check |
| `scripts/build-state.py` | An exec shim; the `podvm-buildstate` binary owns the logic |
| `scripts/document-state.py` | An exec shim; the `document-state` binary owns the logic |
| `crates/podvm-gate/src/main.rs` | Independent task, citation, and document-state gate |
| `crates/podvm-gate/src/count.rs` | Task status and count writer |
| `crates/podvm-gate/src/plant.rs` | Gate failure tests |
| `scripts/common/` | Maintained shell and PowerShell checks |
| `scripts/windows/run-in-base.sh` | Job transport of the parked Windows lane |
| `scripts/windows/kvm-guest.py` | Windows driver for the KVM proof of the parked lane |
| `.github/workflows/` | Hosted checks, the nightly build and the release workflow |

[Tool discovery](agent-tooling.md) gives source search.
[Architecture](architecture.md) gives composition.
[The history index](history/README.md) gives superseded evidence.
