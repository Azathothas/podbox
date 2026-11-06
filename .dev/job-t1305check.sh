#!/bin/sh
# Full lane gate for the T-1305 change: ./scripts/dev.sh check.
# Evidence leaves through /out; the container is removed when it exits.
set -u
cd /work || exit 2
./scripts/dev.sh check
echo "CHECK_RC=$?"
mkdir -p /out
cp .dev/build.log /out/build.log 2>/dev/null || true
echo "== job done"
