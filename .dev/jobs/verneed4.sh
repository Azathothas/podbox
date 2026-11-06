#!/bin/sh
# Lane job: full Verneed table with the symbols under each version.
set -u
cd /work || exit 2
SO=crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so
TARGETS=x86_64-unknown-linux-gnu ./scripts/build-interpose.sh >/dev/null 2>&1 || exit 2
readelf -VW "$SO" | sed -n '/Version needs/,/^$/p' | head -60
echo JOB_DONE
