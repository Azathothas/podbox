#!/bin/sh
# Build the shared library. Link and run a C test program.
set -e
cd "$(dirname "$0")/.."

command -v cc >/dev/null || { echo "cc not found; skipping C integration test"; exit 0; }

echo "== building cdylib =="
cargo build -p memfd-ng-ffi

echo "== compiling C test program =="
LIBDIR="target/debug"
cc -Wall -Wextra -o "$LIBDIR/ffi-smoke" \
    ffi/smoke/main.c \
    -I ffi/include \
    -L "$LIBDIR" -l memfd_ng_ffi

echo "== running (LD_LIBRARY_PATH=$LIBDIR) =="
LD_LIBRARY_PATH="$LIBDIR" "$LIBDIR/ffi-smoke"
