#!/bin/sh
# Lane plant run for T-1309 (ignored, not committed). Runs only the new
# guard against the planted defect (unversioned realpath resolution) and
# expects it red with the defect's own signature. The job exits 0 when the
# guard fires (red test observed) and 1 when it does not, so the lane run
# going green means the plant missed.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
echo "== plant run: realpath guard against unversioned resolution"
RUSTFLAGS='-C target-feature=-crt-static' cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu realpath_null_resolved > /tmp/plant.log 2>&1
rc=$?
tail -12 /tmp/plant.log
cp /tmp/plant.log /out/ 2>/dev/null || true
if [ $rc -ne 0 ] && grep -q "realpath_null_resolved_matches_libc" /tmp/plant.log; then
  echo "PLANT_FIRED: guard red as planted"
  exit 0
fi
echo "PLANT_MISSED: guard did not fire"
exit 1
