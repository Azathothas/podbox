# Repository migration, 2026-09-11

## Inputs

| Input | Identity |
| --- | --- |
| Source commit | `Llaberry/podbox` at `ea5b671b0490a889d514207836cbcdb1fa0d032a` |
| Template commit | `Azathothas/TEMPLATE` at `ea26c7d91a087a7132f56034fb77f8d40d7766cb` |
| Target | `Azathothas/podbox`, branch `main` |
| License | 0BSD |
| Linux method | WSL ToolKit 2.0.2, isolated instance `podbox-migrate` |

No external URLs were supplied for independent review, so the external-review
phase was not applicable.

The ToolKit instance was rootless and had no delegated cgroup controller. That
is sufficient for this repository's build, probe, and denial-path tests; it is
not evidence for cgroup-backed resource enforcement.

## Evidence preservation

Before migration, `experiments/results/` contained 69 committed files with Git
tree identity `4ce9cc4b4753627a9affe172bdde353e7d0f41ac`.
`references/` contained 6,615 committed files, including 169 GitHub API JSON
snapshots and 30 provenance files, with Git tree identity `0749c0b7e2853cb9cb1aed88e84526f90e95e4b8`.

Neither evidence tree was rewritten during migration. The complete source-era
progress record and three handoff documents were moved to `docs/history/`;
their relative TODO links were retargeted without changing their statements or
measurements.

## Template adoption

The root router, documentation roles, history layout, security policy,
changelog, human notes, common checks, doctor scripts, line-ending policy,
commit hook, dependency updater, and read-only remote-item review follow the
current template. Project-specific TODO, experiment, reference, bootstrap, and
build machinery remains authoritative where it is stricter or domain-specific.

The copied methodology from the earlier template revision remains available
because the checked TODO model and experiment discipline depend on it. Two
unused documents that presented competing work models were retired; podbox uses
the TODO model only.

## Behavior repair

The source baseline passed formatting, clippy, static release build, and
project-specific gates. Its first complete workspace test in the isolated
rootless container found a non-idempotent device-completion path. The repair is
described in [`reviews/migration-2026-09-11.md`](reviews/migration-2026-09-11.md).

## Validation

The release candidate was tested in
`docker.io/library/rust:1.98.1-bookworm`. ToolKit job `f7069da26f82ee55`
passed workspace and excluded-interposer formatting, clippy, and tests; the
x86_64-musl release build; both interposer builds; the project TODO gate; and
every maintained shell check before stopping because the Rust image does not
carry PowerShell. The cross-platform twin gate then passed on the host with
POSIX shell and PowerShell 7.6.5.

ToolKit job `d5145093bb1601e1` reproduced the hosted artifact gate. The release
binary was 2,655,088 bytes, leaving 5,344,912 bytes below its 8,000,000-byte
ceiling, and had no `PT_INTERP`. The musl interposer named `libc.so`; the glibc
interposer named `libc.so.6`. Job `2c5f80c0689e1480` verified the canonical
0BSD license hash and parsed all six GitHub YAML files.

Corpus identities are recorded above. Attribution, mutation, and hosted
workflow evidence are attached to the parentless commit after publication;
they cannot truthfully be evidence contained by a commit they validate.

## Publication

The migrated history is intentionally one parentless commit on `main`. The
source revision and template revision above provide the immutable lineage; the
target Git history contains only the reviewed migrated tree.
