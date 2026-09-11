# podbox

podbox is a Linux container runtime for restricted environments where uid 0 is
present but namespaces, mounts, device creation, and ownership changes may be
denied.

It exposes familiar Docker and Podman-shaped commands, probes the host before
choosing an execution rung, and names every degradation it cannot avoid.

## Status

Milestones M0 through M5 are implemented: probing, image acquisition,
ownership-neutral extraction, constrained entry, lifecycle supervision, and
environment completion. M6, ownership virtualization through a per-libc
interposer, is partial. M7 packaging has not started.

The current tree is useful for development and controlled experiments. It is
not a general container isolation boundary and it is not release-complete. See
[`TODO/PROGRESS.md`](TODO/PROGRESS.md) for the live work order and open limits.

## Quick start

On Linux:

```sh
./scripts/common/bootstrap-env.sh rust bloat cc zig tools
cargo build --release --target x86_64-unknown-linux-musl
./target/x86_64-unknown-linux-musl/release/podbox probe
./target/x86_64-unknown-linux-musl/release/podbox pull alpine:latest
./target/x86_64-unknown-linux-musl/release/podbox run alpine:latest /bin/echo hello
```

For repository work, the fast path starts the environment and build in the
background:

```sh
./scripts/dev.sh
./scripts/dev.sh status
./scripts/dev.sh check
```

The interposer is deliberately outside the Cargo workspace and builds once per
libc:

```sh
./scripts/build-interpose.sh
```

## Guarantees and limits

- Every claimed execution rung is derived from probes, not from uid or a build
  constant.
- Image blobs are content-addressed and verified before they are committed to
  the store.
- Layer extraction refuses path and symlink escapes. It records intended image
  ownership without pretending kernel ownership was restored.
- A weaker rung never silently satisfies a stronger request. Unsupported work
  exits with a named refusal.
- The chroot and interpose rungs share the host kernel and are not security
  boundaries against a hostile payload.
- Registry authentication is not implemented. Foreign-architecture execution
  depends on host `binfmt_misc` and QEMU support.

The complete product contract is the pinned
[`TOOL.md`](references/Azathothas__container-research/tree/TOOL.md) in the
research corpus.

## Project map

| Path | Purpose |
| --- | --- |
| [`AGENTS.md`](AGENTS.md) | Binding router for contributors and automated sessions |
| [`docs/architecture.md`](docs/architecture.md) | Runtime boundaries, data flow, and invariants |
| [`docs/code-map.md`](docs/code-map.md) | Crate and script ownership map |
| [`TODO/`](TODO/) | Checked backlog, decisions, acceptance commands, and live state |
| [`experiments/`](experiments/) | Reproducible measurements and committed result captures |
| [`references/`](references/) | Read-only source corpus at pinned revisions |
| [`SECURITY.md`](SECURITY.md) | Threat model, trust boundaries, and reporting route |
| [`THIRD_PARTY.md`](THIRD_PARTY.md) | Copied and redistributed material with license determinations |

## Results and history

- [`TODO/PROGRESS.md`](TODO/PROGRESS.md) is the current verified result set and
  the only live work order.
- [`experiments/README.md`](experiments/README.md) maps each experiment to its
  committed output under `experiments/results/`.
- [`docs/history/source-progress-ea5b671.md`](docs/history/source-progress-ea5b671.md)
  preserves the complete source-project record before migration.
- [`docs/history/migration-2026-09-11.md`](docs/history/migration-2026-09-11.md)
  records the migration inputs, review, and validation.

## Contributing

Read [`AGENTS.md`](AGENTS.md), then [`TODO/PROGRESS.md`](TODO/PROGRESS.md),
before changing the tree. Work from a focused branch, update the relevant TODO
entry in the same change, and run `./scripts/dev.sh check` before opening a
pull request. The [pull-request checklist](.github/PULL_REQUEST_TEMPLATE.md)
defines the review evidence expected before merge.

## License

podbox is released under the [0BSD license](LICENSE). Third-party material
retains its original terms as recorded in [`THIRD_PARTY.md`](THIRD_PARTY.md).
