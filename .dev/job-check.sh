#!/bin/sh
# Ignored wrapper: the staged job file runs under sh, and dev.sh needs
# bash (`set -o pipefail`). Bootstrap first (the toolchain, the musl
# target, zig for the musl interpose link), then call dev.sh with bash.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
bash ./scripts/dev.sh check
