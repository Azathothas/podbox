#!/bin/sh
# Lane job: gettid in static vs dynamic tables, gnu object.
set -u
cd /work || exit 2
TARGETS=x86_64-unknown-linux-gnu ./scripts/build-interpose.sh >/dev/null 2>&1 || exit 2
SO=crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so
echo "== static symtab gettid"
nm "$SO" | grep -w gettid || echo none
echo "== dynamic dynsym gettid"
nm -D "$SO" | grep -w gettid || echo none
echo JOB_DONE
