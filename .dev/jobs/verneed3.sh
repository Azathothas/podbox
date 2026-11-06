#!/bin/sh
# Lane job: name the symbols behind the GLIBC_2.30 and GLIBC_2.34 needs.
set -u
cd /work || exit 2
SO=crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so
TARGETS=x86_64-unknown-linux-gnu ./scripts/build-interpose.sh >/dev/null 2>&1 || exit 2
echo "== dynamic symbols needing 2.30 or newer"
readelf -sW "$SO" | grep -E '@@GLIBC_2\.(3[0-9]|[4-9][0-9])' || echo none-new
echo "== all UND with versions, for context"
readelf -sW "$SO" | awk '$7=="UND"' | grep -oE '[A-Za-z_][A-Za-z0-9_]*(@@GLIBC_[0-9.]+)?' | sort -u
echo JOB_DONE
