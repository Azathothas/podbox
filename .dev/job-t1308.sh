#!/bin/sh
# Lane job for T-1308: musl release build, then 154-tcg-workload-spread.sh.
# Evidence leaves through /out; the container is removed when it exits.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools qemu vmtools || exit 2
cargo build --release --target x86_64-unknown-linux-musl || exit 1
export PODBOX_BIN="/work/target/x86_64-unknown-linux-musl/release/podbox"
# The 2026-09-21 run still had qemu inside the batch at the 600 s wall, so
# the ceiling moves to 1200 for this run. A ceiling is not a measurement:
# the rows carry their own elapsed times.
export PODBOX_154_TIMEOUT=1200
sh experiments/154-tcg-workload-spread.sh
echo "154_RC=$?"
mkdir -p /out
cp experiments/results/tcg-workload-spread.txt /out/tcg-workload-spread.txt
echo "== job done"
