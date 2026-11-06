#!/bin/sh
# Lane job: fmt, clippy and the full interpose unit suite AFTER the fix.
# Expect GREEN throughout, with the new test named in the output.
set -u
cd /work || exit 2
mkdir -p /out || exit 2
{
echo "== fmt"
cargo fmt --manifest-path crates/podbox-interpose/Cargo.toml -- --check
echo "fmt_rc=$?"
echo "== clippy"
RUSTFLAGS='-C target-feature=-crt-static' cargo clippy --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
echo "clippy_rc=$?"
echo "== test"
RUSTFLAGS='-C target-feature=-crt-static' cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu
echo "test_rc=$?"
} > /out/green.log 2>&1
rc=$?
tail -30 /out/green.log
echo "JOB_RC=$rc"
exit "$rc"
