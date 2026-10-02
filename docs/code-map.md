# Code map

## Crates

| Directory | Owner |
| --- | --- |
| `crates/podbox-cli` | Dispatch, arguments, parity, manual, rootfs orchestration, machine and remote commands |
| `crates/podbox-probe` | Disposable probes, host findings, rung selection, shared exit codes |
| `crates/podbox-image` | Registry, credentials, content verification, platforms, store |
| `crates/podbox-extract` | Ordered layers, whiteouts, path containment, metadata |
| `crates/podbox-complete` | Device and account data, package configuration, resolver and CA data |
| `crates/podbox-enter` | Rootfs entry, ELF classification, launch modes, userland entry |
| `crates/podbox-supervise` | Launcher state, pidfds, logs, signals, lifecycle |
| `crates/podbox-windows` | Guest mailbox, agent, disk fetch, QMP, boot plan |
| `crates/podbox-ssh` | SSH transport, relay frames, multiplexing, server and interactive session |
| `crates/podbox-interpose` | Separate glibc and musl objects |
| `crates/podbox-gate` | Task, citation and source-state gate; count writer, plant, and `podbox-dev` host commands |
| `crates/podbox-buildstate` | Build freshness, `podbox-release-licenses` licence inventory |
| `crates/podbox-release` | `podbox-verify`, `podbox-size`, `podbox-prove-t0211`, `document-state`, `release-notes`, `podbox-reconcile`, `podbox-publish` |
| `crates/podbox-podvm` | `podbox-podvm` guest driver and `podbox-podvm-workload` spread measurement |

## Tooling

| File | Purpose |
| --- | --- |
| podbox-dev session, status, check | Host discovery, Linux build and complete check |
| `scripts/build-state.py` | An exec shim; the `podbox-buildstate` binary owns the logic |
| `scripts/document-state.py` | An exec shim; the `document-state` binary owns the logic |
| `scripts/windows/run-in-base.sh` | Windows job and artifact transport |
| `scripts/windows/kvm-guest.py` | Windows driver for the KVM proof |
| `crates/podbox-gate/src/main.rs` | Independent task, citation, and document-state gate |
| `crates/podbox-gate/src/count.rs` | Task status and count writer |
| `crates/podbox-gate/src/plant.rs` | Gate failure tests |
| `scripts/common/` | Maintained shell and PowerShell checks |
| `.github/workflows/` | Hosted checks and release workflow |

[Tool discovery](agent-tooling.md) gives CodeGraph use.
[Architecture](architecture.md) gives composition.
[The history index](history/README.md) gives superseded evidence.
