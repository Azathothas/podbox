#!/bin/sh
# Scratch driver for the 147 debug round: testbed plus a podbox build, then
# 147 only (146 is proven on this tree). Transcripts back through /out.
set -u
export DEBIAN_FRONTEND=noninteractive
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
apt-get update -qq || exit 2
apt-get install -y -qq qemu-system-x86 cpio python3 curl || exit 2
./scripts/dev.sh build || exit 2
export PODBOX_BIN=/work/target/x86_64-unknown-linux-musl/release/podbox
sh experiments/147-podvm-exec.sh
r147=$?
echo "147 rc=$r147"
cp /work/experiments/results/podvm-exec.txt /work/experiments/results/podvm-exec-guest.log /work/experiments/results/podvm-exec-qemu.log /out/ 2>/dev/null || true
exit "$r147"
