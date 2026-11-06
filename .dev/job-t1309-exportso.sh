#!/bin/sh
# Lane job for T-1309: export the two built interpose objects. Evidence
# leaves through /out.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools qemu vmtools || exit 2
./scripts/build-interpose.sh || exit 1
mkdir -p /out
cp crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so /out/interpose-gnu.so
cp crates/podbox-interpose/target/x86_64-unknown-linux-musl/release/libpodbox_interpose.so /out/interpose-musl.so
ls -la /out/
echo "== job done"
