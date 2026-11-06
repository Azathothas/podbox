#!/bin/sh
# Scratch driver for T-1304: testbed plus a podbox build, then the committed
# 146 (re-drive after the lib refactor) and the new 147. Transcripts back
# through /out.
set -u
export DEBIAN_FRONTEND=noninteractive
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
apt-get update -qq || exit 2
apt-get install -y -qq qemu-system-x86 cpio python3 curl || exit 2
./scripts/dev.sh build || exit 2
export PODBOX_BIN=/work/target/x86_64-unknown-linux-musl/release/podbox
sh experiments/146-podvm-initramfs.sh
r146=$?
echo "146 rc=$r146"
sh experiments/147-podvm-exec.sh
r147=$?
echo "147 rc=$r147"
cp /work/experiments/results/podvm-initramfs.txt /work/experiments/results/podvm-exec.txt /out/ 2>/dev/null || true
[ "$r146" -eq 0 ] && [ "$r147" -eq 0 ]
