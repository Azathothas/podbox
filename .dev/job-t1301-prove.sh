#!/bin/sh
# T-1301 driven pass: build the binary in the lane, run its probe, assert the
# entry's Prove against the document. Evidence lands in /out, which
# run-in-base.sh copies to PODBOX_ARTIFACTS on the host.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
./scripts/dev.sh build || exit 2
BIN=/work/target/x86_64-unknown-linux-musl/release/podbox
"$BIN" probe --json >/out/probe.json 2>/out/probe.stderr
echo "probe rc=$?"
echo "--- machine legs"
jq -c '.tiers.machine | {refusal: .refusal, legs: [.legs[] | {name: .name, verdict: .verdict}]}' /out/probe.json || exit 2
echo "--- the entry Prove"
jq -e '.tiers.machine.legs | length >= 5 and (map(select(.verdict == null)) | length == 0)' /out/probe.json
echo "prove rc=$?"
