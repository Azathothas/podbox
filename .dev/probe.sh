#!/bin/sh
echo "== toolchain"
command -v cargo || echo NO-CARGO
cargo --version 2>&1
rustc --version 2>&1
echo "== target dir"
ls /work/target 2>&1 | head -5
echo "== disk"
df -h /work 2>&1 | tail -2
df -i /work 2>&1 | tail -1
echo "== registry reachability"
timeout 25 curl -sI https://static.crates.io/crates/ring/ring-0.17.14.crate 2>&1 | head -4
echo "curl-exit=$?"
echo "== fetch test"
cd /work || exit 2
timeout 240 cargo fetch 2>&1 | tail -8
echo "fetch-exit=$?"
exit 0
