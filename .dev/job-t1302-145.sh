#!/bin/sh
# Scratch driver for T-1302: build the binary in the lane, run the committed
# 145 script against it, hand the transcript back through /out.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
./scripts/dev.sh build || exit 2
PODBOX_BIN=/work/target/x86_64-unknown-linux-musl/release/podbox \
	sh experiments/145-podvm-parity.sh
rc=$?
echo "145 rc=$rc"
cp /work/experiments/results/podvm-parity.txt /out/ 2>/dev/null || true
exit "$rc"
