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

## Tooling

| File | Purpose |
| --- | --- |
| `scripts/session-start.sh` | Host discovery and procedure selection |
| `scripts/dev.sh` | Linux build and complete check |
| `scripts/build-state.py` | Input and output byte verification |
| `scripts/document-state.py` | Generated source-state page |
| `scripts/windows/run-in-base.sh` | Windows job and artifact transport |
| `scripts/windows/kvm-guest.py` | Windows driver for the KVM proof |
| `scripts/check-todo.py` | Independent task, citation, and document-state gate |
| `scripts/todo-count.py` | Task status and count writer |
| `scripts/plant.sh` | Gate failure tests |
| `scripts/common/` | Maintained shell and PowerShell checks |
| `.github/workflows/` | Hosted checks and release workflow |

[Tool discovery](agent-tooling.md) gives CodeGraph use.
[Architecture](architecture.md) gives composition.
[The history index](history/README.md) gives superseded evidence.
