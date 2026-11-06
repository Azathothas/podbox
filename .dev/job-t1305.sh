#!/bin/sh
# Lane job for T-1305: fmt, musl release build, cli+probe unit tests,
# 148-podvm-fleet.sh, and 145-podvm-parity.sh as the tier regression.
# Evidence leaves through /out; the container is removed when it exits.
set -u
cd /work || exit 2
echo "== bootstrap (no docker: the guest lane holds no NET_ADMIN, so the daemon"
echo "== cannot come up here; engine clauses run on host podman instead)"
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
echo "== fmt"
cargo fmt --all -- --check || exit 1
echo "== build"
cargo build --release --target x86_64-unknown-linux-musl || exit 1
echo "== unit tests (cli + probe)"
cargo test -p podbox-cli -p podbox-probe || exit 1
export PODBOX_BIN="/work/target/x86_64-unknown-linux-musl/release/podbox"
echo "== 148"
sh experiments/148-podvm-fleet.sh
echo "148_RC=$?"
echo "== 145 regression"
sh experiments/145-podvm-parity.sh
echo "145_RC=$?"
mkdir -p /out
cp experiments/results/podvm-fleet.txt /out/podvm-fleet.txt
cp experiments/results/podvm-parity.txt /out/podvm-parity.txt
echo "== job done"
