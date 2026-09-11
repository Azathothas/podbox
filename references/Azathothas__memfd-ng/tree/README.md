# memfd-ng

[![crates.io](https://img.shields.io/crates/v/memfd-ng.svg)](https://crates.io/crates/memfd-ng)
[![docs.rs](https://docs.rs/memfd-ng/badge.svg)](https://docs.rs/memfd-ng)
[![license: 0BSD](https://img.shields.io/crates/l/memfd-ng.svg)](https://github.com/Azathothas/memfd-ng/blob/main/LICENSE)

`memfd-ng` executes ELF image bytes from memory on Linux and FreeBSD. The API
follows the main behavior of `std::process::Command`.

```rust
use memfd_ng::{MemFdExecutable, Stdio};

let code = std::fs::read("/bin/sh").unwrap();
let output = MemFdExecutable::new("sh", &code)
    .arg("-c")
    .arg("echo in-memory; exit 7")
    .stdout(Stdio::piped())
    .output()
    .unwrap();

assert_eq!(output.stdout, b"in-memory\n");
assert_eq!(output.status.code(), Some(7));
```

## Install

Add the crate with Cargo:

```sh
cargo add memfd-ng
```

Or add the dependency to `Cargo.toml`:

```toml
[dependencies]
memfd-ng = "0.1.1"
```

| crate | crates.io | documentation |
| --- | --- | --- |
| `memfd-ng` | [crates.io/crates/memfd-ng](https://crates.io/crates/memfd-ng) | [docs.rs/memfd-ng](https://docs.rs/memfd-ng) |
| `memfd-ng-ffi` | [crates.io/crates/memfd-ng-ffi](https://crates.io/crates/memfd-ng-ffi) | [docs.rs/memfd-ng-ffi](https://docs.rs/memfd-ng-ffi) |

## Execution sequence

The library performs these operations:

1. It creates an anonymous file with `memfd_create`.
2. It writes the image to the file.
3. It applies the configured file seals.
4. It creates a child process.
5. It executes the file descriptor.

Linux uses `execveat` with `AT_EMPTY_PATH` first. Linux can then use
`/proc/self/fd/N` when procfs is available. FreeBSD uses `fexecve`.

The library uses a temporary executable file if descriptor execution is not
available. It checks these directories in order:

1. `XDG_RUNTIME_DIR`
2. `TMPDIR`, or `/tmp` when `TMPDIR` is not set
3. `/dev/shm`
4. `/var/tmp`
5. `$HOME/.cache`

Linux checks the mount flags before it uses a directory. Linux first tries
`O_TMPFILE` on supported file systems. FreeBSD uses the named-file method.
The parent process removes each named file after the execution result is
known.

The child does not allocate memory between `fork` and `exec`. The parent
creates the argument and environment arrays before `fork`. The library does
not write diagnostic text to standard error.

## Main behavior

- The library returns the operating system error from a failed `exec` call.
- The library rejects NUL bytes in arguments and environment values.
- Each image descriptor uses `MFD_CLOEXEC`.
- File sealing is enabled by default.
- `prepare()` writes and seals an image once for repeated execution.
- `current_seals()` returns the seals reported by the operating system.
- `setsid()` and `process_group()` configure the child before execution.
- `Child::pidfd()` returns a Linux pidfd when the kernel provides one.
- `kill`, `wait`, and `try_wait` use the pidfd on supported Linux kernels.
- The library uses `fork`, `kill`, and `waitpid` when pidfds are not available.

The default seal set is `F_SEAL_SHRINK | F_SEAL_GROW | F_SEAL_WRITE`. Use
`seals()` to select a different set. Use `sealed(false)` to disable sealing.

## Optional components

### Huge pages

Call `hugetlb(true)` to request a Linux hugetlb memfd. The library uses an
ordinary memfd if the request fails. Call `is_hugetlb()` after `prepare()` to
read the result. Linux 4.16 and later support file seals on hugetlb memfds.
Older kernels use an ordinary memfd when sealing is enabled.

### Command-line program

Enable the `cli` feature to build `memfd-run`.

```sh
cargo build --release --features cli
memfd-run [--name NAME] [--argv0 ARGV0] FILE [ARGS...]
```

The command returns the child exit code. It returns `128 + signal` when a
signal terminates the child. It returns 126 when it cannot start the child.

### C interface

The `memfd-ng-ffi` workspace crate provides a C interface. The interface
includes spawn, process ID, kill, wait, and free operations. See
[`ffi/include/memfd-ng.h`](ffi/include/memfd-ng.h).

## Measured results

These measurements used x86_64 WSL2, Linux 7.2, and Rust 1.98. Each timing
uses 300 executions of a static program that exits with code 0.

| measurement | memfd-ng | VHSgunzo/memfd-exec 0.2.6 |
| --- | ---: | ---: |
| stripped minimal program | 329 KiB | 366 KiB |
| new image for each execution | about 446 microseconds | about 578 microseconds |
| prepared image | about 289 microseconds | not available |

Results depend on the machine and operating system. Run `cargo bench` to
measure the current system.

## Platform checks

The local test suite covers x86_64 Linux with glibc. The static musl job runs
the same feature tests in CI. The cross-target checks cover FreeBSD,
aarch64, ARMv7, i686, PowerPC, PowerPC64, PowerPC64LE, s390x, and RISC-V 64.

The `freebsd` workflow builds and runs the portable tests in a FreeBSD 14.2
virtual machine. The `cross` workflow runs the Linux tests for seven CPU
architectures with QEMU user-mode emulation.

Linux-only tests cover pidfds, hugetlb, Linux mount flags, and Linux fallback
methods. Portable tests cover process behavior, the command-line program, and
the C interface on FreeBSD.

Set `NO_MEMFDEXEC=1` to skip memfd execution and use the temporary-file
sequence.

## Test commands

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --no-fail-fast
cargo test --no-fail-fast --features test-hooks
cargo test --no-fail-fast --features cli
cargo test -p memfd-ng-ffi --no-fail-fast
./scripts/ffi-smoke.sh
cargo publish --dry-run -p memfd-ng
```

The integration tests compile static and dynamic C fixtures. Set
`MEMFD_NG_TEST_CC` to select the C compiler for a cross-target environment.

The `test-hooks` feature is for integration tests. Do not enable it in normal
applications.

## Minimum Rust version

The minimum supported Rust version is 1.65.

## Publication order

Publish `memfd-ng` before `memfd-ng-ffi`. The FFI package depends on version
0.1.1 of `memfd-ng`, so Cargo can run its package or publish dry run only after
that version is available from crates.io.

## License

This project uses the 0BSD license. See [LICENSE](LICENSE).
