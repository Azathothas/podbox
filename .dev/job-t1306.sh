#!/bin/sh
# Lane job for T-1306: fmt, musl release build, probe unit tests, 149.
# Evidence leaves through /out; the container is removed when it exits.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools || exit 2
cargo fmt --all -- --check || exit 1
cargo build --release --target x86_64-unknown-linux-musl || exit 1
cargo test -p podbox-probe || exit 1
export PODBOX_BIN="/work/target/x86_64-unknown-linux-musl/release/podbox"
sh experiments/149-podvm-non-goals.sh
echo "149_RC=$?"
mkdir -p /out
cp experiments/results/podvm-non-goals.txt /out/podvm-non-goals.txt
echo "== job done"
