#!/bin/sh
# CI lane job: the gate.yml lint and unit-test steps on the current tree.
# Exit 0 all green, 1 a step failed, 2 the harness could not run.
set -u
cd /work || exit 2

rc_all=0
run() {
    label="$1"
    shift
    log="/out/$label.log"
    "$@" >"$log" 2>&1
    rc=$?
    echo "== $label rc=$rc"
    if [ "$rc" -ne 0 ]; then
        tail -n 40 "$log"
        rc_all=1
    fi
    return 0
}

echo "== conditions"
rustc --version
cargo --version
git rev-parse --short HEAD 2>/dev/null || echo "head: unknown"

./scripts/common/bootstrap-env.sh rust cc zig tools >/out/bootstrap.log 2>&1
rc=$?
echo "== bootstrap rc=$rc"
[ "$rc" -ne 0 ] && { tail -n 20 /out/bootstrap.log; exit 2; }

run fmt cargo fmt --all -- --check
run fmt-interpose cargo fmt --manifest-path crates/podbox-interpose/Cargo.toml -- --check
run clippy cargo clippy --workspace --all-targets -- -D warnings

# The interposer objects embed through build.rs, so the static build is its
# own step in gate.yml. Build the binary that builds them first.
run interpose-build cargo build --release -p podbox-gate --bin podbox-interpose-build
run interpose-objects sh scripts/build-interpose.sh
run test cargo test --workspace

echo "== verdict rc_all=$rc_all"
exit "$rc_all"