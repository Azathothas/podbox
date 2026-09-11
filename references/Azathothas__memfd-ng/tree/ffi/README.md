# memfd-ng-ffi

[![crates.io](https://img.shields.io/crates/v/memfd-ng-ffi.svg)](https://crates.io/crates/memfd-ng-ffi)
[![docs.rs](https://docs.rs/memfd-ng-ffi/badge.svg)](https://docs.rs/memfd-ng-ffi)
[![license: 0BSD](https://img.shields.io/crates/l/memfd-ng-ffi.svg)](https://github.com/Azathothas/memfd-ng/blob/main/LICENSE)

`memfd-ng-ffi` provides a C interface for `memfd-ng`.

The library executes ELF image bytes from memory on Linux and FreeBSD. See
[`include/memfd-ng.h`](include/memfd-ng.h) for the complete API.

The API provides these operations:

- `memfd_ng_spawn` starts a child process.
- `memfd_ng_pid` returns the child process ID.
- `memfd_ng_kill` sends `SIGKILL` to the child.
- `memfd_ng_wait` waits for the child and returns the raw wait status.
- `memfd_ng_free` releases the child handle.

Functions return negative operating system error numbers when an operation
fails. The caller owns each handle returned by `memfd_ng_spawn`. The caller
must release the handle with `memfd_ng_free`.

The repository includes a C integration test. It builds a C program and links
it to the generated shared library.

## Minimum Rust version

Rust 1.65.

## License

0BSD. See the repository `LICENSE` file.
