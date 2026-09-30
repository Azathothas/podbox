#!/bin/bash
# Does the audit tree pass the full Linux gate and every record-check plant?
# Run through scripts/windows/run-in-base.sh. Only the copied /work tree changes.
# The copied staged tree is committed locally so plant.sh has a clean baseline.
# Exit: 0 matched, 1 failed, 2 could not run.
set -u
[ "$PWD" = /work ] && [ -d .git ] || exit 2
mkdir -p /out || exit 2
echo '== audit conditions'
date -u +%Y-%m-%dT%H:%M:%SZ
uname -sr
echo "input commit: $(git rev-parse HEAD)"
echo "input tree identity: $(git write-tree)"
./scripts/common/bootstrap-env.sh rust cc zig tools openssh || exit 2
if ! git diff --quiet || ! git diff --cached --quiet; then
    git -c commit.gpgsign=false commit -m 'Record audit snapshot for verification' || exit 2
fi
echo "copied snapshot commit: $(git rev-parse HEAD)"
./scripts/dev.sh check >/out/linux-check.txt 2>&1
rc=$?
cat /out/linux-check.txt
[ "$rc" -eq 0 ] || exit "$rc"
./scripts/plant.sh >/out/plant.txt 2>&1
rc=$?
cat /out/plant.txt
[ "$rc" -eq 0 ] || exit "$rc"
for binary in podbox node operator proxy shell; do
    cp "target/x86_64-unknown-linux-musl/release/$binary" "/out/$binary" || exit 1
done
cp .dev/build-state.json /out/build-state.json || exit 1
cp dist/podbox-ssh-x86_64.tar.gz /out/ || exit 1
cp dist/podbox-ssh-x86_64.tar.gz.sha256 /out/ || exit 1
echo 'verdict AUDIT-LINUX-OK'
