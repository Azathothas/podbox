# Security

## Threat model

podbox serves one operator or automation harness running containers on a
restricted Linux host. It assumes the operator controls the host account and
the podbox store, while image names, registries, manifests, layer archives, and
payloads may be untrusted.

In scope are transport verification, digest integrity, storage races, archive
path and symlink traversal, unsafe ownership restoration, and false claims
about which isolation mechanism the host permitted.

Explicitly out of scope are isolation from a malicious payload on the chroot or
interpose rungs, a public multi-tenant control plane, and protection after the
operator account or host kernel is compromised. Those rungs share the host
kernel and must not be presented as virtual machines or namespace containers.

If podbox becomes a network service or serves mutually untrusted tenants, this
threat model must be re-derived before deployment.

## Secrets and authority

The current implementation has no registry-authentication path and stores no
long-lived application credential. Registry authentication remains tracked as
T-0209 in [`TODO/image.md`](TODO/image.md). Do not place credentials in the
repository, command examples, logs, image records, or experiment results.

The process has the authority of the invoking Linux account. The store is
selected by `PODBOX_STORE`, then `XDG_DATA_HOME`, then the account data
directory. Anyone who can modify that store can modify podbox's local images,
root filesystems, lifecycle records, and cached probe result.

Host CA injection is a trust change, not a credential. When enabled, podbox may
copy the CA bundle named by `SSL_CERT_FILE`, `CURL_CA_BUNDLE`, or
`REQUESTS_CA_BUNDLE` into a completed rootfs. `--no-host-cas` disables it.

## Trust boundaries

| Boundary | Untrusted input | Validation |
| --- | --- | --- |
| CLI | verbs, flags, image references, paths | `crates/podbox-cli/src/` and `crates/podbox-image/src/reference.rs` |
| Registry | TLS peer, manifests, indexes, blob bodies | `crates/podbox-image/src/tls.rs`, `registry.rs`, `digest.rs`, and `pull.rs` |
| Store | concurrent records and content-addressed blobs | `crates/podbox-image/src/store.rs` and `contain.rs` |
| Layer archive | entry names, links, whiteouts, metadata | `crates/podbox-extract/src/safety.rs`, `layer.rs`, and `apply.rs` |
| Host capability | syscall results and writable destinations | `crates/podbox-probe/src/probes.rs`, `select.rs`, and `writable.rs` |
| Payload launch | argv, environment, rootfs, foreign interpreter | `crates/podbox-enter/src/` and `crates/podbox-supervise/src/` |

## Invariants

- HTTPS verification is on unless the caller names an explicit insecure
  registry policy. Plain HTTP is never an automatic fallback.
- A blob is accepted only when its computed digest matches the manifest.
- Extraction remains beneath its opened root directory, including when an
  earlier layer entry created a symlink.
- Existing rootfs content survives when a privileged replacement operation is
  denied.
- The selected and entered rungs are reported separately if they differ.
- Unsupported or unmeasurable security properties remain visible and do not
  become success.

## Reporting a vulnerability

Use a private [GitHub security advisory](https://github.com/Azathothas/podbox/security/advisories/new).
Include the affected command, host conditions, expected and observed result,
and the smallest safe reproducer. Do not include credentials or private image
contents. Reports are handled on a best-effort basis; no response-time promise
is made.

## Known limits

- The chroot rung changes path resolution but does not isolate the kernel,
  network, process table, or every host filesystem surface.
- The interposer can affect dynamically linked calls only. Static binaries and
  direct syscalls bypass it.
- Host capability probes describe the instant and context in which they ran.
  Cached results are keyed, but a later external policy change can invalidate
  any runtime observation.
- Two tracked blockers remain: notification supervision cannot be made
  race-safe on the studied target, and one possible vendored dependency lacks
  sufficient license evidence.
