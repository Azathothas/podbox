#!/bin/sh
# Lane job: rebuild the shipped binary with the T-1311 fchmodat fix and hand
# it back through /out. Mirrors the shape recorded in
# .dev/artifacts-build/build.txt: bootstrap, both interposer objects, then
# the musl release binary that embeds them.
set -u
cd /work || exit 2
mkdir -p /out || exit 2
{
echo "== conditions"
echo "date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "head: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "rustc: $(rustc --version 2>/dev/null || echo missing)"
echo "== bootstrap (no docker; the lane has no NET_ADMIN)"
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
echo "== interposer objects (build.rs embeds them)"
./scripts/build-interpose.sh || exit 2
echo "== musl release build"
cargo build --release --target x86_64-unknown-linux-musl || exit 2
BIN=/work/target/x86_64-unknown-linux-musl/release/podbox
cp "$BIN" /out/podbox || exit 2
file /out/podbox || exit 2
echo "version: $(/out/podbox version 2>&1 || true)"
echo "RESULT: /out/podbox ready"
} > /out/build.txt 2>&1
rc=$?
tail -12 /out/build.txt
echo "JOB_RC=$rc"
exit "$rc"
