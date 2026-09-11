# Roadmap

This document lists completed features and possible future work.

## Completed

| feature | current implementation |
| --- | --- |
| Linux pidfd process creation | The library uses `clone3` with `CLONE_PIDFD` and `CLONE_VFORK`. It uses `fork` when the kernel does not support this operation. |
| Pidfd child access | `Child::pidfd()` provides the descriptor. Wait and kill operations use the descriptor when possible. |
| Temporary-file support | Linux uses `O_TMPFILE` when possible. Linux and FreeBSD can use a named file. The parent removes named files. |
| Fallback directories | The library checks `XDG_RUNTIME_DIR`, `TMPDIR` or `/tmp`, `/dev/shm`, `/var/tmp`, and `$HOME/.cache`. |
| File seals | The API selects individual seals and reads the current set. The default set prevents shrink, grow, and write operations. |
| Seal portability | The implementation uses the correct `fcntl` command values on Linux and FreeBSD. |
| Process groups | The API provides `setsid()` and `process_group()`. |
| Multi-architecture checks | Compile checks cover 32-bit, 64-bit, little-endian, and big-endian Linux targets. CI runs seven Linux architectures with QEMU. |
| Linux ABI fixes | `clone_args` uses 64-bit fields. `siginfo.si_status` uses the target-specific `libc` accessor. |
| FreeBSD support | FreeBSD uses `fexecve`. CI builds and runs portable tests in a FreeBSD 14.2 virtual machine. |
| Pipe protocol tests | Structure-based tests check valid, invalid, partial, and modified messages. |
| Huge-page images | `hugetlb(true)` requests a Linux hugetlb memfd and uses a normal memfd after a failure. Linux 4.16 and later apply the requested seals to hugetlb memfds. |
| Command-line program | The `cli` feature provides `memfd-run`. |
| C interface | The `memfd-ng-ffi` crate provides spawn, process ID, kill, wait, and free operations. |

## Possible future work

- Add an explicit Linux huge-page size option.
- Add a typed Linux `vm.memfd_noexec` policy option.
- Add a direct test for the `/var/tmp` fallback directory.
- Add i686 runtime execution to the Linux cross-target workflow.

## Not planned

| request | reason |
| --- | --- |
| `no_std` support | The library requires process, error, and input/output types from the standard library. |
| WASI support | WASI does not provide the required process execution operations. |
| Built-in asynchronous runtime | A separate adapter can add runtime-specific behavior. |
| Raw environment pointer API | The existing environment builder covers the supported use cases. |
