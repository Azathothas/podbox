#!/bin/sh
# Lane job: per-symbol versions via objdump.
set -u
cd /work || exit 2
SO=crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so
TARGETS=x86_64-unknown-linux-gnu ./scripts/build-interpose.sh >/dev/null 2>&1 || exit 2
objdump -T "$SO" | grep -E 'GLIBC_2\.(30|34)' || echo none
echo JOB_DONE
