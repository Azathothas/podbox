#!/bin/sh
# Lane job: run the new fchmodat regression test against the UNFIXED tree.
# Expect RED (assertion mismatch): the guard must refuse before the fix.
set -u
cd /work || exit 2
mkdir -p /out || exit 2
RUSTFLAGS='-C target-feature=-crt-static' cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu fchmodat_forwards_flags > /out/red.log 2>&1
rc=$?
tail -25 /out/red.log
echo "JOB_RC=$rc"
exit "$rc"
