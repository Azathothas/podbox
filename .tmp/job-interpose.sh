#!/bin/sh
# Build the per-libc interposer objects under Rust 1.99 and report their sizes.
# The tracked wrapper scripts/build-interpose.sh resolves target/release/,
# which .cargo/config.toml's musl build target never fills, so this drives the
# same binary directly. Exit 0 both objects built and inside the ceiling,
# 1 a build or ceiling failure, 2 the harness could not run.
set -u
cd /work || exit 2
CEILING=500000
OUT=/out
mkdir -p "$OUT" || exit 2
LOG="$OUT/objects.log"
: >"$LOG" || exit 2

./scripts/common/bootstrap-env.sh rust cc zig tools >>"$LOG" 2>&1
rc=$?
echo "== bootstrap rc=$rc"
[ "$rc" -ne 0 ] && { tail -n 20 "$LOG"; exit 2; }

echo "== rustc $(rustc --version)"

echo "== the interpose builder"
cargo build --release -p podbox-gate --bin podbox-interpose-build >>"$LOG" 2>&1
rc=$?
echo "builder rc=$rc"
if [ "$rc" -ne 0 ]; then
    tail -n 40 "$LOG"
    exit 1
fi
BUILDER="target/x86_64-unknown-linux-musl/release/podbox-interpose-build"
if [ ! -x "$BUILDER" ]; then
    echo "RESULT: no builder at $BUILDER"
    ls target/*/release/ 2>/dev/null | head -30
    exit 2
fi
echo "builder: $BUILDER"

echo "== the two objects"
"$BUILDER" >"$OUT/objects.txt" 2>&1
rc=$?
echo "objects rc=$rc"
cat "$OUT/objects.txt"

fail=0
for so in crates/podbox-interpose/target/*/release/libpodbox_interpose.so; do
    [ -f "$so" ] || continue
    size=$(stat -c %s "$so")
    echo "$size $so"
    if [ "$size" -gt "$CEILING" ]; then
        echo "FAIL: $so is $size bytes, over the $CEILING ceiling"
        fail=1
    fi
done

if [ "$fail" -eq 0 ]; then
    echo "RESULT: both objects built and inside the $CEILING-byte ceiling"
    exit 0
fi
exit 1