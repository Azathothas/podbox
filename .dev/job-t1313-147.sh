#!/bin/sh
# Lane job for T-1313 (147 half): musl release build, then
# 147-podvm-exec.sh with the fetch ceiling. Evidence leaves through /out;
# the container is removed on exit.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools qemu vmtools || exit 2
cargo build --release --target x86_64-unknown-linux-musl || exit 1
export PODBOX_BIN="/work/target/x86_64-unknown-linux-musl/release/podbox"
sh experiments/147-podvm-exec.sh
echo "147_RC=$?"
mkdir -p /out
cp experiments/results/podvm-exec.txt /out/podvm-exec.txt
echo "== job done"
