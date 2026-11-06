#!/bin/sh
# Lane job for T-1309: build the interpose objects, then the musl release
# binary embedding them, and export it. Evidence leaves through /out.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools qemu vmtools || exit 2
./scripts/build-interpose.sh || exit 1
ls -la crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so crates/podbox-interpose/target/x86_64-unknown-linux-musl/release/libpodbox_interpose.so
cargo build --release --target x86_64-unknown-linux-musl 2>&1 | grep -i "interpos" || echo "no interpose warnings in build"
export PODBOX_BIN="/work/target/x86_64-unknown-linux-musl/release/podbox"
ls -la "$PODBOX_BIN"
mkdir -p /out
cp "$PODBOX_BIN" /out/podbox
echo "== job done"
