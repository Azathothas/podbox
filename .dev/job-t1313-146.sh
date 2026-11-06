#!/bin/sh
# Lane job for T-1313 (146 half): musl release build, then
# 146-podvm-initramfs.sh with the fetch ceiling and the stalled-origin
# clause. Evidence leaves through /out; the container is removed on exit.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools qemu vmtools || exit 2
cargo build --release --target x86_64-unknown-linux-musl || exit 1
export PODBOX_BIN="/work/target/x86_64-unknown-linux-musl/release/podbox"
sh experiments/146-podvm-initramfs.sh
echo "146_RC=$?"
mkdir -p /out
cp experiments/results/podvm-initramfs.txt /out/podvm-initramfs.txt
cp "$PODBOX_BIN" /out/podbox
echo "== job done"
