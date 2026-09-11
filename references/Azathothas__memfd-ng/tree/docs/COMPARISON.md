# Comparison with related crates

This document compares `memfd-ng` with three related projects. The source
review uses these revisions:

| project | revision | version |
| --- | --- | --- |
| novafacing/memfd-exec | `0a15efeaf17b7f748b81e4cf01643a81f139f99f` | no tag |
| VHSgunzo/memfd-exec | `2decf7d1cec3d183e55000526abd2e5bb6df40c5` | 0.2.6 |
| lucab/memfd-rs | `34f0581d1b16214ffcd00680914cd5b48660e3d7` | 0.6.4 |

The local reference copies record these revisions in `.refs/PROVENANCE.md`.

## Scope and dependencies

| item | memfd-ng | novafacing/memfd-exec | VHSgunzo/memfd-exec | lucab/memfd-rs |
| --- | --- | --- | --- | --- |
| Main purpose | Execute image bytes | Execute image bytes | Execute image bytes | Create and seal memfds |
| License | 0BSD | MIT | MIT | MIT OR Apache-2.0 |
| Direct dependencies | `libc` | `libc`, `nix` | `libc`, `nix` | `rustix` |
| Minimum Rust version | 1.65 | not stated | at least 1.69 through `nix` 0.31 | 1.85 |

The VHSgunzo project has three development dependencies. They are
`tempfile`, `serial_test`, and `reqwest`. It does not list `bitflags` as a
direct dependency.

## Execution and fallback behavior

| behavior | memfd-ng | novafacing | VHSgunzo | memfd-rs |
| --- | --- | --- | --- | --- |
| Linux descriptor execution | `execveat` with `AT_EMPTY_PATH` | `fexecve` | `fexecve` | No execution API |
| FreeBSD descriptor execution | `fexecve` | Not supported | Not supported | No execution API |
| `/proc/self/fd` fallback | Yes | No | No | Not applicable |
| Temporary-file fallback | Five directory candidates | No | Three directory candidates | Not applicable |
| Linux `ST_NOEXEC` check | Yes | No | No | Not applicable |
| Parent-controlled file cleanup | Yes | Not applicable | No | Not applicable |
| Writes fallback diagnostics | No | Not applicable | Yes | Not applicable |

The temporary-file sequence checks `XDG_RUNTIME_DIR`, `TMPDIR` or `/tmp`,
`/dev/shm`, `/var/tmp`, and `$HOME/.cache`. Linux first tries `O_TMPFILE`.
FreeBSD uses the named-file method.

## Correctness

| behavior | memfd-ng | novafacing | VHSgunzo | standard library |
| --- | --- | --- | --- | --- |
| Exit code 0 reports success | Yes | No at the reviewed revision | No at the reviewed revision | Yes |
| Failed `exec` returns its operating system error | Yes | No | No | Yes |
| Image descriptor uses close-on-exec | Yes | Yes | No at the reviewed revision | Not applicable |
| Partial writes are handled | Yes | No | No | Not applicable |
| NUL input is rejected | Yes | No | No | Yes |
| User input is excluded from fallback paths | Yes | Not applicable | No | Not applicable |
| Default image seals | Shrink, grow, and write | None | None | Not applicable |

The original seal constants in `memfd-ng` were incorrect. The previous values
used bit 0 for shrink and bit 1 for grow. Bit 0 is `F_SEAL_SEAL`. The corrected
values are `0x2` for shrink, `0x4` for grow, and `0x8` for write. Tests read the
active set with `F_GET_SEALS`.

Linux and FreeBSD use different `fcntl` command numbers for seal operations.
The implementation selects the command numbers for each operating system.

## API and operating system features

| feature | memfd-ng | novafacing | VHSgunzo | memfd-rs |
| --- | --- | --- | --- | --- |
| Command-style builder | Yes | Yes | Yes | No |
| Prepared image reuse | Yes | No | No | Not applicable |
| Read active seals | Yes | No | No | Yes |
| Linux `MFD_EXEC` handling | Yes | No | No | Yes |
| Linux hugetlb request | Yes | No | No | Yes |
| Linux pidfd child handle | Yes | No | No | Not applicable |
| Process group controls | Yes | No | No | Not applicable |
| Command-line program | Yes | No | No | No |
| C interface | Yes | No | No | No |

`memfd-ng` uses a 64-bit `clone_args.pidfd` field on all Linux targets. This
matches the kernel ABI on 32-bit and 64-bit systems. It uses the target-specific
`libc` accessor for `siginfo.si_status`. Compile checks cover little-endian and
big-endian targets.

## Measurements

These measurements used x86_64 WSL2, Linux 7.2, and Rust 1.98. Each timing
uses 300 executions of a static program that exits with code 0.

| measurement | memfd-ng | VHSgunzo/memfd-exec 0.2.6 |
| --- | ---: | ---: |
| stripped minimal program | 337,352 bytes | 375,088 bytes |
| new image for each execution | about 446 microseconds | about 578 microseconds |
| prepared image | about 289 microseconds | not available |

These values apply only to the measured system. Use the included benchmark to
measure another system.

## Verification

The Linux suite includes process comparisons with `std::process::Command`.
It also includes tests for pidfds, process groups, seals, hugetlb, fallback
methods, the command-line program, and the C interface.

The FreeBSD cross-target build checks all workspace targets and features. The
FreeBSD workflow runs the portable tests in a FreeBSD 14.2 virtual machine.
The Linux cross-target workflow covers seven CPU architectures with QEMU.

`memfd-ng` provides more execution features than the reviewed alternatives.
`memfd-rs` remains a focused memfd creation library and does not provide a
process execution API.
