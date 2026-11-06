#!/bin/sh
# T-1109 step-clause triage (ignored, not committed). A/B: --strict against
# the floating opensuse tag 250 uses versus the digest 240 pins, on a
# lane-built binary. Prints the refusal reasons each names.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
./scripts/build-interpose.sh || exit 1
./scripts/dev.sh build || exit 1
BIN="target/x86_64-unknown-linux-musl/release/podbox"
export PODBOX_STORE=/tmp/stepstore
FLOAT="registry.opensuse.org/opensuse/leap:15.6"
PINNED="registry.opensuse.org/opensuse/leap:15.6@sha256:e4d84bbc5e0fa0df64381e95ad7ea8e3867cc34f3b1d174e9948d0f559d223e2"
"$BIN" pull "$FLOAT" >/dev/null 2>&1 || { echo "PULL_FLOAT_FAILED"; exit 2; }
echo "== floating tag:"
"$BIN" run --strict --rm "$FLOAT" true >/tmp/float.out 2>&1
echo "FLOAT_RC=$?"
grep -E "^  - |degraded in" /tmp/float.out | cut -c1-110
"$BIN" pull "$PINNED" >/dev/null 2>&1 || { echo "PULL_PINNED_FAILED"; exit 2; }
echo "== pinned digest:"
"$BIN" run --strict --rm "$PINNED" true >/tmp/pinned.out 2>&1
echo "PINNED_RC=$?"
grep -E "^  - |degraded in" /tmp/pinned.out | cut -c1-110
echo "== triage job done"
