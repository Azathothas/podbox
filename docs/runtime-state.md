# Source state

This page is generated from the tracked manifests and source declarations.
It records the build surface. Runtime proof and limits live in
[PROGRESS](../TODO/PROGRESS.md) and [Limits](limits.md).

Regenerate with `py scripts/document-state.py --write` on Windows.
Use `python3` on Linux. The record gate rejects a changed snapshot.

## Workspace

Declared version: `0.1.0-beta.13`.

| Member | Manifest |
| --- | --- |
| `podvm` | [crates/podvm/Cargo.toml](../crates/podvm/Cargo.toml) |
| `podvm-probe` | [crates/podvm-probe/Cargo.toml](../crates/podvm-probe/Cargo.toml) |
| `podvm-image` | [crates/podvm-image/Cargo.toml](../crates/podvm-image/Cargo.toml) |
| `podvm-extract` | [crates/podvm-extract/Cargo.toml](../crates/podvm-extract/Cargo.toml) |
| `podvm-complete` | [crates/podvm-complete/Cargo.toml](../crates/podvm-complete/Cargo.toml) |
| `podvm-enter` | [crates/podvm-enter/Cargo.toml](../crates/podvm-enter/Cargo.toml) |
| `podvm-gate` | [crates/podvm-gate/Cargo.toml](../crates/podvm-gate/Cargo.toml) |
| `podvm-buildstate` | [crates/podvm-buildstate/Cargo.toml](../crates/podvm-buildstate/Cargo.toml) |
| `podvm-release` | [crates/podvm-release/Cargo.toml](../crates/podvm-release/Cargo.toml) |
| `podvm-measure` | [crates/podvm-measure/Cargo.toml](../crates/podvm-measure/Cargo.toml) |
| `podvm-supervise` | [crates/podvm-supervise/Cargo.toml](../crates/podvm-supervise/Cargo.toml) |
| `podvm-windows` | [crates/podvm-windows/Cargo.toml](../crates/podvm-windows/Cargo.toml) |
| `podvm-ssh` | [crates/podvm-ssh/Cargo.toml](../crates/podvm-ssh/Cargo.toml) |

Separate crate: `crates/podvm-interpose`.

## Mechanism declarations

`Rung` in [crates/podvm-probe/src/select.rs](../crates/podvm-probe/src/select.rs): `Namespace`, `Supervise`, `Chroot`, `Userland`, `Interpose`, `Unsupported`.

`Tier` in [crates/podvm/src/tier.rs](../crates/podvm/src/tier.rs): `Ladder`, `Chroot`, `Machine`.

`Mode` in [crates/podvm-enter/src/ladder.rs](../crates/podvm-enter/src/ladder.rs): `Memfd`, `Fuse`, `Tmpfs`, `RunDir`, `Cache`.

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
| `base64` | `0.23.1` |
| `flate2` | `1.1.10` |
| `linux-raw-sys` | `0.12.1` |
| `pod-core` | `0.6.0` |
| `pod-proxy` | `0.6.0` |
| `pod-term` | `0.6.0` |
| `pod-tls` | `0.6.0` |
| `rustls` | `0.23.46` |
| `rustls-pki-types` | `1.15.1` |
| `ruzstd` | `0.9.1` |
| `serde` | `1.0.229` |
| `serde_json` | `1.0.154` |
| `sha2` | `0.11.1` |
| `tar` | `0.4.46` |
