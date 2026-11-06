#!/bin/sh
# T-0415 focused tests (ignored, not committed). Devices suite only.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
cargo test -p podbox-complete 2>&1 | tail -12
