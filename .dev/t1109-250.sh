#!/bin/sh
# T-1109 re-drive of 250 (ignored, not committed). Builds the lane binary
# with both interpose objects, then drives every refusal. The report rides
# home through /out; the exit code is the script's own.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
./scripts/build-interpose.sh || exit 1
./scripts/dev.sh build || exit 1
PODBOX_BIN=target/x86_64-unknown-linux-musl/release/podbox \
	bash experiments/250-negative-tests.sh
rc=$?
cp experiments/results/negative-tests.txt /out/negative-tests.txt 2>/dev/null || true
echo "250_RC=$rc"
exit $rc
