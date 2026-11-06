#!/bin/sh
# Lane job: show the gnu link invocation and whether -ldl reaches it.
set -u
cd /work/crates/podbox-interpose || exit 2
RUSTFLAGS="-C target-feature=-crt-static -C link-arg=-Wl,--no-as-needed -C link-arg=-ldl -C link-arg=-Wl,--as-needed -C link-arg=-Wl,--version-script=/work/crates/podbox-interpose/interpose.map" \
  cargo build --release --target x86_64-unknown-linux-gnu -v > /tmp/link.log 2>&1
echo "build_rc=$?"
grep -oE '\-ldl|\-\-no-as-needed' /tmp/link.log | sort | uniq -c
echo "== the cc line (trimmed)"
grep -oE '"cc" .* -o [^ ]*libpodbox[^ ]*' /tmp/link.log | head -2 | cut -c1-400
echo JOB_DONE
