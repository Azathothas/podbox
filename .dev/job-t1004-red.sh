#!/bin/sh
# Lane job: RED run of the T-1004 version unit tests. Expects failure
# (todo!() stubs); the log is the proof the tests can catch the gap.
set -u
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 1
cargo test --target x86_64-unknown-linux-musl -p podbox-cli version
rc=$?
echo "== cargo test exited $rc (non-zero is the RED this job exists for)"
exit "$rc"
