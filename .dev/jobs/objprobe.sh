#!/bin/sh
# Lane job: objcopy variants on the built gnu object.
set -u
cd /work || exit 2
SO=crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so
TARGETS=x86_64-unknown-linux-gnu ./scripts/build-interpose.sh >/dev/null 2>&1
echo "== baseline dynamic entries"
nm -D "$SO" | grep -w gettid
readelf -sW "$SO" | grep -w gettid | head -5
cp "$SO" /tmp/p.so
echo "== localize-symbol again, verbose stat"
objcopy --localize-symbol=gettid /tmp/p.so /tmp/p1.so && ls -l /tmp/p.so /tmp/p1.so
nm -D /tmp/p1.so | grep -w gettid || echo "p1: gone from dynsym"
echo "== try keep-global inverted: globalize then check --help"
objcopy --help 2>&1 | grep -iE "localize|globalize" | head -8
echo JOB_DONE
