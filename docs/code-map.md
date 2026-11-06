# Code map

## Crates

| Directory | Owner |
| --- | --- |
| `crates/podvm` | Dispatch, arguments, parity, manual, rootfs orchestration, machine and remote commands |
| `crates/podvm-probe` | Disposable probes, host findings, rung selection, shared exit codes |
| `crates/podvm-image` | Registry, credentials, content verification, platforms, store |
| `crates/podvm-extract` | Ordered layers, whiteouts, path containment, metadata |
| `crates/podvm-complete` | Device and account data, package configuration, resolver and CA data |
| `crates/podvm-enter` | Rootfs entry, ELF classification, launch modes, userland entry |
| `crates/podvm-supervise` | Launcher state, pidfds, logs, signals, lifecycle |
| `crates/podvm-windows` | Guest mailbox, agent, disk fetch, QMP, boot plan |
| `crates/podvm-ssh` | SSH transport, relay frames, multiplexing, server and interactive session |
| `crates/podvm-interpose` | Separate glibc and musl objects |
| `crates/podvm-gate` | Task, citation and source-state gate; count writer, plant, and `podvm-dev` host commands |
| `crates/podvm-buildstate` | Build freshness, `podvm-release-licenses` licence inventory |
| `crates/podvm-release` | `podvm-verify`, `podvm-size`, `podvm-prove-t0211`, `document-state`, `release-notes`, `podvm-reconcile`, `podvm-publish` |
| `crates/podvm-measure` | `podvm-parity` guest driver and `podvm-workload` spread measurement |

## Tooling

| File | Purpose |
| --- | --- |
| podvm-dev session, status, check | Host discovery, Linux build and complete check |
| `scripts/build-state.py` | An exec shim; the `podvm-buildstate` binary owns the logic |
| `scripts/document-state.py` | An exec shim; the `document-state` binary owns the logic |
| `scripts/windows/run-in-base.sh` | Windows job and artifact transport |
| `scripts/windows/kvm-guest.py` | Windows driver for the KVM proof |
| `crates/podvm-gate/src/main.rs` | Independent task, citation, and document-state gate |
| `crates/podvm-gate/src/count.rs` | Task status and count writer |
| `crates/podvm-gate/src/plant.rs` | Gate failure tests |
| `scripts/common/` | Maintained shell and PowerShell checks |
| `.github/workflows/` | Hosted checks and release workflow |

[Tool discovery](agent-tooling.md) gives CodeGraph use.
[Architecture](architecture.md) gives composition.
[The history index](history/README.md) gives superseded evidence.
