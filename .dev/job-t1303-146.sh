#!/bin/sh
# Scratch driver for T-1303: testbed (qemu, cpio, python3) plus a podbox
# build, then the committed 146 script. Transcript back through /out.
set -u
export DEBIAN_FRONTEND=noninteractive
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
apt-get update -qq || exit 2
apt-get install -y -qq qemu-system-x86 cpio python3 curl || exit 2
./scripts/dev.sh build || exit 2
PODBOX_BIN=/work/target/x86_64-unknown-linux-musl/release/podbox \
	sh experiments/146-podvm-initramfs.sh
rc=$?
echo "146 rc=$rc"
cp /work/experiments/results/podvm-initramfs.txt /out/ 2>/dev/null || true
exit "$rc"
