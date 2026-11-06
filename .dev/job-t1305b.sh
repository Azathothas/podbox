#!/bin/sh
# Lane evidence job for T-1305: musl release build, 148, 145, 325.
# Evidence leaves through /out; the container is removed when it exits.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
cargo build --release --target x86_64-unknown-linux-musl || exit 1
export PODBOX_BIN="/work/target/x86_64-unknown-linux-musl/release/podbox"
sh experiments/148-podvm-fleet.sh
echo "148_RC=$?"
sh experiments/145-podvm-parity.sh
echo "145_RC=$?"
sh experiments/325-parity-drive.sh
echo "325_RC=$?"
mkdir -p /out
cp experiments/results/podvm-fleet.txt /out/podvm-fleet.txt
cp experiments/results/podvm-parity.txt /out/podvm-parity.txt
cp experiments/results/parity-drive.txt /out/parity-drive.txt
echo "== job done"
