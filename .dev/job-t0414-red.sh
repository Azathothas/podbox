#!/bin/sh
# Lane job: RED run of the T-0414 root-listing test. Strips the three
# registrations inside the disposable container, then runs the test, which
# must FAIL (legs missing). The checkout is untouched.
set -u
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 1
awk 'index($0, "name: \"readdir(/") || index($0, "name: \"open(/bin") || index($0, "name: \"creat(/,") { skip = 2 } skip > 0 { skip--; next } 1' crates/podbox-probe/src/probes.rs > /tmp/probes-red.rs || exit 1
mv /tmp/probes-red.rs crates/podbox-probe/src/probes.rs || exit 1
cargo test --target x86_64-unknown-linux-musl -p podbox-probe root_listing
rc=$?
echo "== cargo test exited $rc (non-zero is the RED this job exists for)"
exit "$rc"
