# podvm

podvm runs Linux payloads on hosts that can deny namespaces, mounts,
device creation, and ownership changes. It accepts Docker and Podman commands
and reports the behavior it can provide.

## Current behavior

The runtime has image acquisition, layer extraction, rootfs completion,
payload entry, lifecycle control, interposition, and a machine tier.
The CLI also provides remote and machine SSH commands.

Read [the architecture](docs/architecture.md) for the execution paths.
Read [the limits](docs/limits.md) before you select a path.
[The generated source state](docs/runtime-state.md) names the current build inputs.
[The work record](TODO/PROGRESS.md) gives current verification and remaining work.

## Build on Linux

```sh
./scripts/common/bootstrap-env.sh rust cc zig tools openssh
./target/release/podvm-dev build
./target/x86_64-unknown-linux-musl/release/podvm probe
./target/x86_64-unknown-linux-musl/release/podvm run alpine:latest /bin/echo hello
```

The build script builds both interposer objects before the CLI embeds them.
The build status verifies input and output bytes.

For repository work, start with `./target/release/podvm-dev session`.
Use `./target/release/podvm-dev check` for the complete Linux check.

## Build from Windows

```powershell
cargo build --release -p podvm-gate
./target/release/podvm-dev session
./target/release/podvm-gate
$env:PODVM_ARTIFACTS = '.dev/artifacts'
sh scripts/windows/run-in-base.sh
```

The Linux job copies this checkout into the `podvm` base.
It returns requested artifacts through `/out`.
The [Windows procedure](docs/containers.md) gives setup and cleanup steps.
The produced executable runs on Linux.

## Run a payload

```sh
podvm probe
podvm doctor
podvm run alpine:latest /bin/echo hello
podvm man
```

The probe measures this host. The banner reports the selected mechanism.
`inspect` and `system info` report runtime state.

A chroot changes path resolution. The namespace path creates a mount namespace
and private temporary filesystem. It does not create user, PID, or network isolation.
Where chroot is denied, the userland path can use the image loader or memfd.
Read the named refusal when a required mechanism is absent.

## Windows and DOS guests

The machine tier runs a disposable disk guest through an emulator.
It also has a FreeDOS path. It does not redistribute a Windows image.

```sh
podvm windows doctor
podvm windows setup --image win.vhdx
podvm windows run --image win.podvm.qcow2 -- ver
podvm run --rm --podvm-tier=machine --platform windows/amd64 win.podvm.qcow2 cmd /c ver
```

The operator supplies the image under its applicable terms.
A run uses a fresh overlay and a mailbox for command output and exit status.
The [guest task](TODO/milestones.md) records proof per accelerator and guest flavor.

## SSH

```sh
podvm remote ssh --help
podvm machine ssh --help
```

The remote commands use the `node`, `operator`, and `proxy` helper executables.
Place compatible helpers beside podvm or on PATH.
A source workspace build produces them. Read the release limits before you
use a downloaded standalone binary for remote SSH.

The release workflow also packages all four helpers in an archive for each
architecture. Keep the CLI and helpers from the same release in one directory.
Use the archive's licence directory with the distributed executables.
The archive does not include an external SSH server.

Select a verified tag and architecture from
[the release assets](https://github.com/PodUtils/podvm/releases).
From this checkout, verify the selected binary and helper archive:

```sh
sh scripts/verify-release.sh TAG ARCH binary
sh scripts/verify-release.sh TAG ARCH ssh
```

Binary `podvm-verify` owns that verification, and the two lines above
are an exec shim.
The verifier requires `gh` and `cosign`.
It checks the tag's workflow identity.
The project was named podbox up to `v0.1.0-beta.13`, the last podbox release.
Releases up to `v0.1.0-beta.13` carry `podbox-*` assets and are signed by the nightly workflow of `Azathothas/podbox`, and the verifier checks those tags against that identity.
Each later tag is a podvm release with `podvm-*` assets; `podvm-verify` decides the identity from the tag.
`Azathothas/podbox` redirects to this repository, so nobody may create a new repository of that name: it would break the redirect and the old identity with it.
The server endpoint also needs the SSH server named by its configuration.

## Documents and evidence

| Page | Purpose |
| --- | --- |
| [AGENTS.md](AGENTS.md) | Session entry point and task routing |
| [HUMAN.md](HUMAN.md) | Operator procedure |
| [Architecture](docs/architecture.md) | Execution paths and data boundaries |
| [Code map](docs/code-map.md) | Source ownership |
| [Limits](docs/limits.md) | Current constraints and proof gaps |
| [TODO index](TODO/INDEX.md) | Tasks and derived counts |
| [Experiments](experiments/README.md) | Scripts and result captures |
| [Security](SECURITY.md) | Trust boundaries |
| [Third-party notice](THIRD_PARTY.md) | Licenses and carried material |

## Contributions

Read the router and live work record. Work on `main`.
Update the affected task in the same change.
Run the full gate before commit and authorized publication.
The [review checklist](.github/PULL_REQUEST_TEMPLATE.md) defines review evidence.

## License

podvm uses [0BSD](LICENSE). Carried material retains its own terms.
