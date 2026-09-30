# pg-toolkit

pg-toolkit builds a Linux program one time, against glibc, and makes the
result run on glibc and musl distributions. The output of the main mode is an
ordinary static ELF executable: no launcher, no runtime beside it, and no
packaging format.

```sh
./pg-toolkit binary --engine podman env create   # the pinned build environment
./pg-toolkit binary rootfs fetch                 # the test bed: eleven distributions
./pg-toolkit binary build -- make                # build your project, without changes to it
./pg-toolkit binary verify ./yourprogram         # run the result on the eleven distributions
```

---

## The problem

A static glibc executable is not self-contained, although `file` and `ldd` say
that it is. glibc loads name-service modules and character-conversion modules
with `dlopen` at run time. The host configuration names these modules, and
each module needs `libc.so.6`. So a second libc enters a process that every
inspection tool calls static.

Measured on eleven pinned distributions, a plain `gcc -static` executable:

| test | result |
| --- | --- |
| name resolution | loads host name-service modules on 5 of 11, and the process stops on 2 |
| character conversion | stops on 3, and loses 11 of 12 encodings on the others |
| a UTF-8 locale | gets `ANSI_X3.4-1968` on all 4 musl distributions |

The same source, built by the binary mode, runs on all eleven and opens no host
shared object. [`docs/binary/README.md`](docs/binary/README.md) has the detail.

## What the binary mode does

Two mechanisms are on by default. A mechanism that writes to the filesystem,
or that changes what a program does, is on only when you ask for it.

| mechanism | effect |
| --- | --- |
| name resolution | pins the databases to services that glibc has inside itself. Nothing is loaded |
| character conversion | links the encoding tables from static GNU libiconv |
| `--embed-locale` | carries C.UTF-8, and writes it only where the host has no answer |
| `--wrap-dlopen` | answers `dlopen` for the plugins the build made, from a table |
| `--host-dlopen` | compiles an ELF loader in, which binds a host plugin to the libc already linked |
| `--embed-terminfo`, `--embed-cacert`, `--embed-tzdata`, `--embed-netdb` | uses the host data first, and carries a fallback |

No mechanism changes your source code. Compiler wrappers on `PATH` deliver
them, so autotools, CMake, meson and make use them without changes.

## Status

| mode | output | state |
| --- | --- | --- |
| `binary` | one static ELF | works. The ten proof-of-concept projects build and run |
| `appimage` | an AppImage from an AppDir you assembled | works. Discovery of the AppDir is open (T-146) |
| `bundle` | the same artefact from a nixpkgs package name | works, with core work open |
| `container` | a container or a relocatable distribution | draft. [`docs/container.md`](docs/container.md) |
| `distro` | the case that nothing else reaches | draft. [`docs/distro.md`](docs/distro.md) |

`pg-toolkit plan SUBJECT` names the mode that a subject needs.
[`docs/toolkit.md`](docs/toolkit.md) explains the choice.

## Limits

- No LDAP, SSSD, NIS, mDNS or systemd-resolved. These are the host modules
  that the binary mode keeps out.
- x86_64 only. aarch64 is not tested.
- Compatibility is claimed for the eleven named distributions only.

[`docs/limitations.md`](docs/limitations.md) has each open problem with a
reproduction.

---

## Get started

You need a Linux host, or a Windows host with WSL 2. macOS is not supported;
use a Linux VM. The build environment is a pinned Debian image that the tool
fetches, so the compiler of your host does not build your program.

### Linux

```sh
git clone https://github.com/Azathothas/pg-toolkit
cd pg-toolkit
make setup
make check
```

`make setup` installs the packages with sudo, then Rust with the musl target
and Go into `/opt`. `make check` builds every artefact, then runs every test
and the gate. Root is not necessary after the setup: a non-root caller enters a
user namespace and mounts in it. `pg-toolkit binary doctor` shows the route
that this machine has.

### Windows

Use Git Bash in the checkout. Install Git, Go and GNU make first
([`docs/windows.md`](docs/windows.md), section "Requirements").

```sh
go build -o pg-devenv ./cmd/pg-devenv
./pg-devenv tool install
make setup
make check
```

`./pg-devenv tool install` installs wsl-toolkit. `make setup` makes a WSL
distribution for this project (the base) and installs the toolchain in it.
`make check` runs the checks there.

### After the setup

| task | command |
| --- | --- |
| run any command in the environment | `./pg-devenv 'CMD'` |
| build everything | `make` |
| build, test and check | `make check` |
| see what is outdated | `./build --check` |
| run the gate | `./check` |
| see what the host can do | `./pg-devenv doctor` |

The gate exits 1 today, with one known problem in the check `bundle`.
[`TODO/PROGRESS.md`](TODO/PROGRESS.md) has the expected state. A different
number is a finding.

---

## Documentation

| page | subject |
| --- | --- |
| [`HUMAN.md`](HUMAN.md) | setup, verification and troubleshooting, for a person |
| [`AGENTS.md`](AGENTS.md) | the start page for an automated session |
| [`docs/README.md`](docs/README.md) | the map: which page answers which question |
| [`docs/devenv.md`](docs/devenv.md) | `pg-devenv`, the one way to run a command |
| [`docs/build.md`](docs/build.md) | `make` and `./build` |
| [`docs/windows.md`](docs/windows.md) | the Windows host and the wsl-toolkit base |
| [`docs/binary/README.md`](docs/binary/README.md) | the binary mode |
| [`docs/appimage/README.md`](docs/appimage/README.md) | the appimage and bundle modes |
| [`docs/comparison.md`](docs/comparison.md) | this project against the alternatives, measured |
| [`SECURITY.md`](SECURITY.md) | the threat model |
| [`CHANGELOG.md`](CHANGELOG.md) | what changed, when, and where the evidence is |

---

## Licence

0BSD. See [`LICENSE`](LICENSE).

GNU libiconv is LGPL. The binary mode links it statically into a binary that
calls `iconv`, so the LGPL relinking obligation applies to that binary. This
repository does not redistribute libiconv: the build environment fetches and
builds it. [`docs/binary/iconv.md`](docs/binary/iconv.md) names the build flag
that turns the mechanism off, and what that costs.

`vendor/` holds trees that this project patches, each with its licence.
`references/` holds trees for study, which are never built.
[`patches/UPSTREAM.md`](patches/UPSTREAM.md) records every change to a
vendored tree.
