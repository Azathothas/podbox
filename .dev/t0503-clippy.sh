#!/bin/sh
# Clippy with visible errors (ignored, not committed).
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
cargo clippy --workspace --all-targets -- -D warnings > /out/clippy.log 2>&1
rc=$?
tail -n 30 /out/clippy.log
echo "CLIPPY_RC=$rc"
exit $rc
