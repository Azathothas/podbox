#!/bin/sh
# Lane job: GREEN run of the T-1004 version unit tests. Expects all pass.
set -u
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 1
cargo test --target x86_64-unknown-linux-musl -p podbox-cli version
