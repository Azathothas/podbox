# Build

`make` builds [`../cmd/build`](../cmd/build) and then runs it. `./build`
knows every artefact of this tree, finds which are outdated, builds them,
strips them and installs them.

| command | effect |
| --- | --- |
| `make` | build what is outdated. On Windows: the Windows tools only (see below) |
| `make NAME` | build one artefact. The names come from `./build --list` |
| `make check` | build, then run every selftest, every crate test and the gate |
| `make gate` | rebuild the gate binary, then run it |
| `./build --check` | report what is outdated. Change nothing |
| `./build --list` | list every artefact and what builds it |
| `./build --test` | the same as `make check` on Linux |
| `./build --clean` | remove what a build wrote |
| `./build pg-toolkit prun` | build the named artefacts only |

`make check` is the name of the test run. `./build check` builds the gate
binary, which is also called `check`.

---

## The artefacts

| name | language | output | what it is |
| --- | --- | --- | --- |
| `pg-devenv` | Go | `pg-devenv` | runs a command in the development environment. [`devenv.md`](devenv.md) |
| `check` | Go | `check` | the gate: the checks over the tracked tree and the history |
| `pg-toolkit` | Go | `pg-toolkit` | the program of the project: every mode. [`toolkit.md`](toolkit.md) |
| `pgb` | Go | `pgb` | the same program as `pg-toolkit`, under the name the parked experiments use (T-147) |
| `pstrip` | Rust | `bin/pstrip` | the ELF stripper. [`appimage/stripper.md`](appimage/stripper.md) |
| `prun` | Rust | `bin/prun` | the launcher a bundle carries. [`appimage/launcher.md`](appimage/launcher.md) |
| `pboot` | Rust | `bin/pboot` | the runtime an artefact starts with |

The Go binaries are at the root of the checkout, and every page and script
calls them there (`./check`, `./pg-toolkit`). The crates go to `bin/`.

The three crates link static glibc through the binary mode, against the pinned
build environment (T-145). That needs the environment and root. Without them,
`./build` uses the host cargo and static musl, and the run ends with a list of
the artefacts that used this fallback.

These directories are carried sources, not artefacts:

| directory | what it is |
| --- | --- |
| `tool/runtime/binary` | C that the binary mode compiles into a user's binary. `./check c-runtime` reads it |
| `tool/runtime/appimage` | C that is loaded into a bundled payload |
| `tool/runtime/verify` | C for the tracer of `pg-toolkit binary verify`. It never enters a user's binary |

### On Windows

`pg-toolkit` and `pgb` do not build on Windows: they call `mknod` and read
`syscall.Stat_t`. The crates are Linux programs. So on Windows, `make` builds
`pg-devenv` and `check` only. The base builds the rest:

```sh
./pg-devenv --mirror 'make'
./pg-devenv --mirror 'make check'
```

`make check` on Windows runs the second command.

---

## How staleness is found

```sh
./build --check
```

The input set of each artefact is derived, not written in a list:

- a Go binary: `go list -deps -json` gives the source files and the
  `go:embed` files of every package of this module that the binary uses
- a crate: its directory, without `target/`
- every artefact: the version string of the toolchain, so a compiler change
  makes everything outdated

Measured on this tree:

| edit | makes outdated |
| --- | --- |
| `tool/runtime/binary/elfload.c` | `pg-toolkit` only, which embeds it |
| `tool/prun/src/names.rs` | `prun`, and `pg-toolkit`, which embeds the launcher sources |
| `internal/elfx` | `pg-toolkit` only, and not `check` or `pg-devenv` |

`bin/manifest.json` records, for each artefact, the digest of its inputs and
the digest of the binary written. A record that does not match the file on
disk is outdated, whatever the inputs say.

---

## Stripping

Each output is linked with `-trimpath` and without a symbol table (`-s -w` for
Go, `strip = true` for the crates). Then [`../tool/pstrip`](../tool/pstrip)
removes the section header table and every byte after the last `PT_LOAD`,
which `strip` cannot do.

The stripper runs as a second pass over every output, because the stripper is
itself an artefact. A file that is not an ELF (for example a Windows build) is
skipped; the check reads the first four bytes of the file, not `GOOS`.

Measured on a Go binary: 7,734 bytes less on `check`, with identical output.
That is 0.1% to 0.5% on a binary that the linker stripped already.

---

## Failures

`./build` tries every artefact and reports the failures together at the end.
A failure does not stop the others. Each artefact carries a note that turns a
compiler message into an action (for example, the `mknod` note above).

---

## The test run

```sh
make check
```

Three suites, in this order:

1. `pg-toolkit selftest`
2. each Rust crate, through [`../scripts/common/rust-test.sh`](../scripts/common/rust-test.sh)
3. the gate, `./check`

A suite that could not run is reported by name and does not count as a pass.

Measured on 2026-09-29 in the Windows base (Debian 13, in the mirror): 924
selftest cases pass; the crates pass 25, 65 and 27 tests; the gate has one
problem, in `bundle` (T-104). `make` then returns 2, because a recipe failed.
[`../TODO/PROGRESS.md`](../TODO/PROGRESS.md) has the known gate state.

---

## One name for each artefact

The outputs have no extension on any host, Windows too. A Go binary without an
extension runs from Git Bash. Two files for one artefact (`check` and
`check.exe`) let the shell run the older one, so a build removes the `.exe`
twin when it finds one.
