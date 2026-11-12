# Security

## Trust model

The operator controls the host account and podvm store.
Registries, image metadata, archives, payloads, and remote peers are untrusted inputs.
The runtime validates transport, digest, archive paths, and requested mechanisms.

The chroot, userland, and interpose paths share host resources.
They do not isolate an untrusted payload from the invoking account.
The namespace path supplies the mount behavior stated in
[the architecture](docs/architecture.md), without user, PID, or network isolation.
The Windows, DOS, and `machine ssh` guests run in QEMU on KVM or TCG.
Under TCG the emulator process is the boundary, not hardware isolation.

## Credentials

Registry login uses the Docker configuration format or a credential helper.
The caller supplies a password through stdin.
Logout removes the selected record.
The [credential module](crates/podvm-image/src/credentials.rs) gives podvm's paths to
`pod_oci::credentials` (PodUtils/podcrates), which reads and writes the records.

SSH server configuration, relay tokens, and private keys confer separate authority.
Keep them outside the tree. Do not publish them in result captures.
TLS verification remains enabled on the relay path.

## Boundaries

| Boundary | Source |
| --- | --- |
| CLI arguments and dispatch | `crates/podvm/src/` |
| Registry transport and bodies | `pod-oci` (PodUtils/podcrates); `crates/podvm-image/src/registry.rs`, `tls.rs`, `pull.rs` |
| Credential storage | `pod-oci` (PodUtils/podcrates); `crates/podvm-image/src/credentials.rs` |
| Store names and mutations | `crates/podvm-image/src/store.rs`, `contain.rs` |
| Archive names and links | `src/safety.rs`, `src/layer.rs` of `pod-extract` (PodUtils/podcrates) |
| Host operation probes | `pod-hostprobe` (PodUtils/podcrates); `crates/podvm-probe/src/` |
| Entry and process lifecycle | `pod-enter`, `pod-supervise` (PodUtils/podcrates); `crates/podvm-enter/src/`, `crates/podvm-supervise/src/` |
| Root completion writes | `crates/podvm-complete/src/`; `src/write.rs`, `src/write/follow.rs` of `pod-rootfs` (PodUtils/podcrates) |
| Relay framing and sessions | `crates/podvm-ssh/src/`; `pod-relay`, `pod-ws`, `pod-tls`, `pod-proxy`, `pod-term` (PodUtils/podcrates) |
| Guest disk and mailbox | `pod-guest` (PodUtils/podcrates); `crates/podvm-windows/src/` |

## Invariants

- The registry verifies HTTPS unless the caller selects an explicit registry policy.
- The runtime does not select plain HTTP as an automatic fallback.
- Blob digests must match before the store accepts their bytes.
- Layer paths and symlink resolution remain beneath the extraction root.
- Completion reads and writes remain beneath the root, and a fifo or a device of the image does not stop them.
- Intended ownership metadata does not change kernel permission checks.
- Runtime output states the mechanism entered.
- A missing or unmeasured mechanism does not become a successful isolation claim.

The store path follows `PODVM_STORE`, then `XDG_DATA_HOME`, then the account data directory.
A process that can change the store can change local images and runtime records.
Host CA injection changes the payload's trust data.
`--no-host-cas` disables that completion step.

## Current limits

[Limits](docs/limits.md) owns the current constraint list.
Interposition covers library calls. Static payloads and direct syscalls can
operate outside that coverage.
A probe describes the host at measurement time.
An external policy change can invalidate a cached observation.

## Release identity

`v0.1.0-beta.12` is the last podbox release, and `v0.2.0-beta.2` is the first podvm release.
The tags `v0.1.0-beta.13` and `v0.2.0-beta.1` have no release, because their builds failed.
Tags up to `v0.1.0-beta.13` keep the podbox identity: their releases carry `podbox-*` assets and are signed by the nightly workflow of `Azathothas/podbox`.
`podvm-verify` checks those tags against that identity and every later tag against this repository's.
`Azathothas/podbox` redirects here; nobody may create a new repository of that name, because it would take over the redirect.

## Report a vulnerability

Use a private [security advisory](https://github.com/PodUtils/podvm/security/advisories/new).
Include the command, host conditions, expected result, actual result, and a small reproducer.
Exclude credentials and private image content. No response-time promise applies.
