#!/bin/sh
# Lane job for T-1309 (ignored, not committed). Probes compat presence on
# the lane libc, builds both interpose objects with the ceiling assertion,
# then runs the interpose unit suite on the gnu target. Evidence leaves
# through /out and the job's own stdout.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
echo "== probe: realpath versions on the lane libc"
if command -v nm >/dev/null 2>&1; then
  nm -D /lib/x86_64-linux-gnu/libc.so.6 2>/dev/null | grep -E " realpath" | head -5
else
  echo "no nm on lane"
fi
echo "== build the interpose objects"
./scripts/build-interpose.sh || exit 1
echo "== interpose unit tests, gnu target"
RUSTFLAGS='-C target-feature=-crt-static' cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu > /tmp/itest.log 2>&1
rc=$?
tail -25 /tmp/itest.log
cp /tmp/itest.log /out/ 2>/dev/null || true
echo "== rc=$rc"
exit $rc
