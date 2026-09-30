# Security

## Trust model

The operator controls the host account and podbox store.
Registries, image metadata, archives, payloads, and remote peers are untrusted inputs.
The runtime validates transport, digest, archive paths, and requested mechanisms.

The chroot, userland, and interpose paths share host resources.
They do not isolate an untrusted payload from the invoking account.
The namespace path supplies the mount behavior stated in
[the architecture](docs/architecture.md), without user, PID, or network isolation.
The machine tier boots a guest through the selected emulator.

## Credentials

Registry login uses the Docker configuration format or a credential helper.
The caller supplies a password through stdin.
Logout removes the selected record.
The [credential module](crates/podbox-image/src/credentials.rs) owns this path.

SSH server configuration, relay tokens, and private keys confer separate authority.
Keep them outside the tree. Do not publish them in result captures.
TLS verification remains enabled on the relay path.

## Boundaries

| Boundary | Source |
| --- | --- |
| CLI arguments and dispatch | `crates/podbox-cli/src/` |
| Registry transport and bodies | `crates/podbox-image/src/registry.rs`, `tls.rs`, `pull.rs` |
| Credential storage | `crates/podbox-image/src/credentials.rs` |
| Store names and mutations | `crates/podbox-image/src/store.rs`, `contain.rs` |
| Archive names and links | `crates/podbox-extract/src/safety.rs`, `layer.rs` |
| Host operation probes | `crates/podbox-probe/src/` |
| Entry and process lifecycle | `crates/podbox-enter/src/`, `crates/podbox-supervise/src/` |
| Relay framing and sessions | `crates/podbox-ssh/src/` |
| Guest disk and mailbox | `crates/podbox-windows/src/` |

## Invariants

- The registry verifies HTTPS unless the caller selects an explicit registry policy.
- The runtime does not select plain HTTP as an automatic fallback.
- Blob digests must match before the store accepts their bytes.
- Layer paths and symlink resolution remain beneath the extraction root.
- Intended ownership metadata does not change kernel permission checks.
- Runtime output states the mechanism entered.
- A missing or unmeasured mechanism does not become a successful isolation claim.

The store path follows `PODBOX_STORE`, then `XDG_DATA_HOME`, then the account data directory.
A process that can change the store can change local images and runtime records.
Host CA injection changes the payload's trust data.
`--no-host-cas` disables that completion step.

## Current limits

[Limits](docs/limits.md) owns the current constraint list.
Interposition covers library calls. Static payloads and direct syscalls can
operate outside that coverage.
A probe describes the host at measurement time.
An external policy change can invalidate a cached observation.

## Report a vulnerability

Use a private [security advisory](https://github.com/Azathothas/podbox/security/advisories/new).
Include the command, host conditions, expected result, actual result, and a small reproducer.
Exclude credentials and private image content. No response-time promise applies.
