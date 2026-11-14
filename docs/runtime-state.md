# Source state

This page is generated from the tracked manifests and source declarations.
It records the build surface. Runtime proof and limits live in
[PROGRESS](../TODO/PROGRESS.md) and [Limits](limits.md).

Regenerate with `python3 scripts/document-state.py --write`.
The record gate rejects a changed snapshot.

## Workspace

Declared version: `0.2.0-beta.17`.

| Member | Manifest |
| --- | --- |
| `podvm` | [crates/podvm/Cargo.toml](../crates/podvm/Cargo.toml) |
| `podvm-probe` | [crates/podvm-probe/Cargo.toml](../crates/podvm-probe/Cargo.toml) |
| `podvm-image` | [crates/podvm-image/Cargo.toml](../crates/podvm-image/Cargo.toml) |
| `podvm-extract` | [crates/podvm-extract/Cargo.toml](../crates/podvm-extract/Cargo.toml) |
| `podvm-complete` | [crates/podvm-complete/Cargo.toml](../crates/podvm-complete/Cargo.toml) |
| `podvm-enter` | [crates/podvm-enter/Cargo.toml](../crates/podvm-enter/Cargo.toml) |
| `podvm-dispatch` | [crates/podvm-dispatch/Cargo.toml](../crates/podvm-dispatch/Cargo.toml) |
| `podvm-gate` | [crates/podvm-gate/Cargo.toml](../crates/podvm-gate/Cargo.toml) |
| `podvm-buildstate` | [crates/podvm-buildstate/Cargo.toml](../crates/podvm-buildstate/Cargo.toml) |
| `podvm-release` | [crates/podvm-release/Cargo.toml](../crates/podvm-release/Cargo.toml) |
| `podvm-measure` | [crates/podvm-measure/Cargo.toml](../crates/podvm-measure/Cargo.toml) |
| `podvm-supervise` | [crates/podvm-supervise/Cargo.toml](../crates/podvm-supervise/Cargo.toml) |
| `podvm-windows` | [crates/podvm-windows/Cargo.toml](../crates/podvm-windows/Cargo.toml) |
| `podvm-ssh` | [crates/podvm-ssh/Cargo.toml](../crates/podvm-ssh/Cargo.toml) |
| `podvm-todo` | [crates/podvm-todo/Cargo.toml](../crates/podvm-todo/Cargo.toml) |
| `podvm-api` | [crates/podvm-api/Cargo.toml](../crates/podvm-api/Cargo.toml) |

Separate crate: `crates/podvm-interpose`.

## Mechanism declarations

`Rung` is `pod_hostprobe::select::Rung` (PodUtils/podcrates), re-exported by [crates/podvm-probe/src/select.rs](../crates/podvm-probe/src/select.rs).

`Tier` in [crates/podvm/src/tier.rs](../crates/podvm/src/tier.rs): `Ladder`, `Machine`, `Incus`, `Emulate`, `Syscall`, `Runsc`.

`Mode` is `pod_enter::ladder::Mode` (PodUtils/podcrates), re-exported by [crates/podvm-enter/src/lib.rs](../crates/podvm-enter/src/lib.rs).

## SSH helper sources

| Executable | Source |
| --- | --- |
| `node` | [crates/podvm-ssh/src/bin/node.rs](../crates/podvm-ssh/src/bin/node.rs) |
| `operator` | [crates/podvm-ssh/src/bin/operator.rs](../crates/podvm-ssh/src/bin/operator.rs) |
| `proxy` | [crates/podvm-ssh/src/bin/proxy.rs](../crates/podvm-ssh/src/bin/proxy.rs) |
| `shell` | [crates/podvm-ssh/src/bin/shell.rs](../crates/podvm-ssh/src/bin/shell.rs) |

## Declared direct registry dependencies

| Dependency | Locked versions |
| --- | --- |
| `flate2` | `1.1.11` |
| `pod-broker` | `0.43.0` |
| `pod-confine` | `0.43.0` |
| `pod-core` | `0.43.0` |
| `pod-enter` | `0.43.0` |
| `pod-extract` | `0.43.0` |
| `pod-guest` | `0.43.0` |
| `pod-hostprobe` | `0.43.0` |
| `pod-incus` | `0.43.0` |
| `pod-link` | `0.43.0` |
| `pod-netlab` | `0.43.0` |
| `pod-oci` | `0.43.0` |
| `pod-proxy` | `0.43.0` |
| `pod-relay` | `0.43.0` |
| `pod-rootfs` | `0.43.0` |
| `pod-services` | `0.43.0` |
| `pod-supervise` | `0.43.0` |
| `pod-term` | `0.43.0` |
| `pod-tls` | `0.43.0` |
| `pod-todo` | `0.43.0` |
| `pod-ws` | `0.43.0` |
| `rustls` | `0.23.46` |
| `rustls-pki-types` | `1.15.1` |
| `serde` | `1.0.229` |
| `serde_json` | `1.0.154` |
| `sha2` | `0.10.9`, `0.11.1` |
| `tar` | `0.4.46` |
| `tokio` | `1.54.1` |
| `toml` | `1.2.1+spec-1.1.0` |
| `xz4rust` | `0.2.3` |
