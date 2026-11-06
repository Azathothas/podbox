#!/bin/sh
# Lane job retry: stream everything, trace on, no redirect that can hide
# an early death.
set -ux
cd /work || exit 2
echo "PWD=$(pwd)"
ls /out 2>&1 || mkdir -p /out || exit 2
echo "== rustup targets"
rustup target list --installed 2>&1
echo "== readelf present?"
command -v readelf || echo "NO readelf"
echo "== build gnu object only"
TARGETS=x86_64-unknown-linux-gnu ./scripts/build-interpose.sh
echo "build_rc=$?"
SO=crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so
ls -l "$SO"
readelf -dW "$SO" | grep -E 'NEEDED|SONAME'
readelf -VW "$SO" | grep -oE 'GLIBC_[0-9.]+' | sort -uV
echo JOB_DONE
