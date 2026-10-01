# Source state

This page is generated from the tracked manifests and source declarations.
It records the build surface. Runtime proof and limits live in
[PROGRESS](../TODO/PROGRESS.md) and [Limits](limits.md).

Regenerate with `py scripts/document-state.py --write` on Windows.
Use `python3` on Linux. The record gate rejects a changed snapshot.

## Workspace

Declared version: `0.1.0-beta.12`.

| Member | Manifest |
| --- | --- |
| `podbox-cli` | [crates/podbox-cli/Cargo.toml](../crates/podbox-cli/Cargo.toml) |
| `podbox-probe` | [crates/podbox-probe/Cargo.toml](../crates/podbox-probe/Cargo.toml) |
| `podbox-image` | [crates/podbox-image/Cargo.toml](../crates/podbox-image/Cargo.toml) |
| `podbox-extract` | [crates/podbox-extract/Cargo.toml](../crates/podbox-extract/Cargo.toml) |
| `podbox-complete` | [crates/podbox-complete/Cargo.toml](../crates/podbox-complete/Cargo.toml) |
| `podbox-enter` | [crates/podbox-enter/Cargo.toml](../crates/podbox-enter/Cargo.toml) |
| `podbox-gate` | [crates/podbox-gate/Cargo.toml](../crates/podbox-gate/Cargo.toml) |
| `podbox-supervise` | [crates/podbox-supervise/Cargo.toml](../crates/podbox-supervise/Cargo.toml) |
| `podbox-windows` | [crates/podbox-windows/Cargo.toml](../crates/podbox-windows/Cargo.toml) |
| `podbox-ssh` | [crates/podbox-ssh/Cargo.toml](../crates/podbox-ssh/Cargo.toml) |

Separate crate: `crates/podbox-interpose`.

## Mechanism declarations

`Rung` in [crates/podbox-probe/src/select.rs](../crates/podbox-probe/src/select.rs): `Namespace`, `Supervise`, `Chroot`, `Userland`, `Interpose`, `Unsupported`.

`Tier` in [crates/podbox-cli/src/tier.rs](../crates/podbox-cli/src/tier.rs): `Ladder`, `Chroot`, `Machine`.

`Mode` in [crates/podbox-enter/src/ladder.rs](../crates/podbox-enter/src/ladder.rs): `Memfd`, `Fuse`, `Tmpfs`, `RunDir`, `Cache`.

## SSH helper sources

| Executable | Source |
| --- | --- |
| `node` | [crates/podbox-ssh/src/bin/node.rs](../crates/podbox-ssh/src/bin/node.rs) |
| `operator` | [crates/podbox-ssh/src/bin/operator.rs](../crates/podbox-ssh/src/bin/operator.rs) |
| `proxy` | [crates/podbox-ssh/src/bin/proxy.rs](../crates/podbox-ssh/src/bin/proxy.rs) |
| `shell` | [crates/podbox-ssh/src/bin/shell.rs](../crates/podbox-ssh/src/bin/shell.rs) |

## Declared direct registry dependencies

| Dependency | Locked versions |
| --- | --- |
| `base64` | `0.22.1`, `0.23.1` |
| `flate2` | `1.1.10` |
| `linux-raw-sys` | `0.12.1` |
| `rustls` | `0.23.45` |
| `rustls-pemfile` | `2.2.0` |
| `rustls-pki-types` | `1.15.1` |
| `ruzstd` | `0.9.0` |
| `serde` | `1.0.229` |
| `serde_json` | `1.0.151` |
| `sha2` | `0.11.0` |
| `tar` | `0.4.46` |
| `ureq` | `2.12.1` |
| `webpki-roots` | `0.26.11`, `1.0.9` |
