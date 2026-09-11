#!/bin/sh
# Run the complete local release check.
set -e
cd "$(dirname "$0")/.."

echo "== formatting =="
cargo fmt --all --check

echo "== clippy (workspace, all targets, all features) =="
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "== tests (default features) =="
cargo test --no-fail-fast

echo "== tests (test-hooks) =="
cargo test --no-fail-fast --features test-hooks

echo "== tests (cli feature) =="
cargo test --no-fail-fast --features cli

echo "== FFI tests (Rust side) =="
cargo test -p memfd-ng-ffi --no-fail-fast

echo "== C interface integration test =="
sh scripts/ffi-smoke.sh

echo "== release build =="
cargo build --release --features cli

echo "== crates.io package check =="
cargo publish --dry-run --allow-dirty -p memfd-ng
cargo package --list -p memfd-ng-ffi

echo "== bench (spawn latency) =="
cargo bench

echo "all checks passed"
