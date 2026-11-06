#!/bin/sh
# Lane job: build the release musl binary and hand it back through /out.
# Evidence for TODO/complete.md T-0414 (drive of experiments/155).
set -u
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 1
./scripts/build-interpose.sh || exit 1
cargo build --release --target x86_64-unknown-linux-musl || exit 1
BIN=target/x86_64-unknown-linux-musl/release/podbox
"$BIN" version || exit 1
mkdir -p /out || exit 1
cp "$BIN" /out/podbox || exit 1
"$BIN" version > /out/version.txt || exit 1
echo "== handed /out/podbox"
