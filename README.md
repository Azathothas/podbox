# podvm

podvm runs Linux payloads on hosts that can deny namespaces, mounts, chroot,
device creation, ownership changes, or ptrace. It takes Docker and Podman
commands, selects a mechanism from the host probes, and names that
mechanism in its output. Each released binary is static musl.

## Current behavior

The runtime has image acquisition, layer extraction, rootfs completion,
payload entry, lifecycle control, interposition, and a machine tier.
The CLI also provides remote and machine SSH commands.
Most mechanisms are shared crates of
[PodUtils/podcrates](https://github.com/PodUtils/podcrates); [the code map](docs/code-map.md) names them.
The machine tier boots a Linux guest from an OCI image (`run
--podvm-tier=machine`, TODO/machine.md T-1656), and Windows, DOS, and
`machine ssh` guests.
A Linux guest can have no NIC (`--podvm-net none`), one channel to a host
socket (`--podvm-channel`), its run as a bundle on stdin with no host
file of the run (`--podvm-sealed`, T-1751), and a container engine
(`--podvm-engine`, T-1780); the incus tier also takes the first two
(T-1779). [The limits](docs/limits.md#guests) state each flag.
`--podvm-tier` also selects `incus` (an incusd over its REST API,
T-1718; `tier = incus` in `$store/config` selects it with no flag,
T-1722), `runsc` (gVisor, rootless, T-1729), `emulate` (a static
qemu-user, T-1651) and `syscall` (each system call through podvm's
handler, T-1657, T-1731). `podvm lab` plans a network lab in one guest
(T-1662), and the crate `podvm-api` runs podvm from Rust (T-1745).

Read [the architecture](docs/architecture.md) for the execution paths.
Read [the limits](docs/limits.md) before you select a path.
[The generated source state](docs/runtime-state.md) names the current build inputs.
[The work record](TODO/PROGRESS.md) gives current verification and remaining work.

## Install

Download the binary of your architecture from
[the releases](https://github.com/PodUtils/podvm/releases), check it
against its `.sha256`, and put it on `PATH`:

```sh
curl -fLO https://github.com/PodUtils/podvm/releases/download/TAG/podvm-x86_64
curl -fLO https://github.com/PodUtils/podvm/releases/download/TAG/podvm-x86_64.sha256
sha256sum -c podvm-x86_64.sha256
install -m 0755 podvm-x86_64 ~/.local/bin/podvm
podvm version --verbose
```

Each binary is static: it needs no library of the host. The release has
binaries for x86_64, aarch64, armv7, i686, loongarch64, powerpc64le and
riscv64gc; the interposer of the userland rung is in the x86_64 and
aarch64 binaries. `podvm run --help` and `podvm man` list each flag, with
the parity of each docker flag.

## Report a problem

Open [an issue](https://github.com/PodUtils/podvm/issues/new/choose) with
the bug form. Give the command, the image with its tag, what you expected,
and the output of:

```sh
podvm version --verbose
podvm system info --format '{{.Version}} {{.Rung}} {{.EnteredRung}} {{.Platform}} {{.Kernel}}'
podvm probe
```

Run the failing command again with `--verbose` (the banner names the
mechanism that podvm entered) and, for `run`, with `--report report.json`,
which records the rung, the exit code and the times of the run.

## Build on Linux

```sh
sh scripts/cloud-setup.sh
export PATH="$HOME/.local/podvm/zig-x86_64-linux-0.16.0:$PATH"
cargo build --release -p podvm-gate
./target/x86_64-unknown-linux-musl/release/podvm-dev build
./target/x86_64-unknown-linux-musl/release/podvm probe
./target/x86_64-unknown-linux-musl/release/podvm run alpine:latest /bin/echo hello
```

`scripts/cloud-setup.sh` installs zig, the Rust toolchain and the musl
target, and reports the host. The workspace builds static musl binaries
under `target/x86_64-unknown-linux-musl/release/`.
`podvm-dev build` builds both interposer objects before the CLI embeds them.
The build status verifies input and output bytes.

For repository work, read [AGENTS.md](AGENTS.md) first.
`podvm-dev session` reports the host, and `podvm-dev check` runs the
complete Linux check ([the gate](docs/methodology/gate.md)).

The Windows lane on wsl-toolkit is parked. [Container procedures](docs/containers.md)
keep its steps.

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
Where chroot is denied, the userland path enters a dynamic payload through the image loader.
A static x86_64 payload there runs on the native-static path: podvm resolves each
path of its system calls in the image (Syscall User Dispatch, TODO/enter.md T-1652).
Under `--unsafe-host-paths` it runs from a memfd instead.
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
A source workspace build produces them. Read [the SSH limits](docs/limits.md#ssh)
before you use a downloaded standalone binary for remote SSH.

The release workflow also packages all four helpers in an archive for each
architecture. Keep the CLI and helpers from the same release in one directory.
Use the archive's licence directory with the distributed executables.
The archive does not include an external SSH server.
The server endpoint also needs the SSH server named by its configuration.

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
`v0.1.0-beta.12` is the last podbox release, and `v0.2.0-beta.2` is the first podvm release.
The tags `v0.1.0-beta.13` and `v0.2.0-beta.1` have no release, because their builds failed.
Tags up to `v0.1.0-beta.13` keep the podbox identity: their releases carry `podbox-*` assets and are signed by the nightly workflow of `Azathothas/podbox`, and the verifier checks those tags against that identity.
Each later tag carries the podvm identity, and its release has `podvm-*` assets; `podvm-verify` decides the identity from the tag.
`Azathothas/podbox` redirects to this repository, so nobody may create a new repository of that name: it would break the redirect and the old identity with it.

## Documents and evidence

| Page | Purpose |
| --- | --- |
| [AGENTS.md](AGENTS.md) | Session entry point and task routing |
| [HUMAN.md](HUMAN.md) | Operator procedure |
| [Architecture](docs/architecture.md) | Execution paths and data boundaries |
| [Code map](docs/code-map.md) | Source ownership |
| [Limits](docs/limits.md) | Current constraints and proof gaps |
| [Findings](docs/findings.md) | Defects of tools and host traits, with the host of each row |
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
