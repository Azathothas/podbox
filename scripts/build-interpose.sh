#!/usr/bin/env sh
# build-interpose.sh - declaration stub (T-1557). The build lives in the
# podbox-interpose-build binary in the gate crate; this file stays because
# the per-object byte ceiling is declared here once and the gate reads it
# from here (TODO/gate.md T-1207 item 3, D-1 option c).
#
# THE PER-OBJECT CEILING. Both objects embed in the release binary, so
# their growth hides in that binary's headroom. The committed per-libc
# reading is experiments/results/bloat-interpose.txt. No Rust source
# repeats the digits below: the gate scans for them, so a literal turns
# the gate red.

INTERPOSE_CEILING_BYTES=500000

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
ROOT="$(CDPATH= cd -- "$HERE/.." && pwd)"
# ⛔ THE PER-TARGET SUBDIRECTORY IS NOT OPTIONAL. `.cargo/config.toml` sets
# `build.target = x86_64-unknown-linux-musl`, so a plain `cargo build --release`
# lands in `target/<triple>/release/` and `target/release/` is never written.
# Resolving only the flat directory is why this script exited 2 with the binary
# built, in a lane and in CI (TODO/enter.md T-1604). The same candidate list
# and the same first-match order that `scripts/release-notes.sh` and
# `scripts/verify-release.sh` use, so one search serves every compat shim.
# The triples are the ones the release matrix builds, read rather than guessed.
BIN=""
for cand in "$ROOT/target/release/podbox-interpose-build" \
	"$ROOT/target/x86_64-unknown-linux-musl/release/podbox-interpose-build" \
	"$ROOT/target/aarch64-unknown-linux-musl/release/podbox-interpose-build" \
	"$ROOT/target/riscv64gc-unknown-linux-musl/release/podbox-interpose-build" \
	"$ROOT/target/loongarch64-unknown-linux-musl/release/podbox-interpose-build" \
	"$ROOT/target/powerpc64le-unknown-linux-musl/release/podbox-interpose-build" \
	"$ROOT/target/armv7-unknown-linux-musleabihf/release/podbox-interpose-build" \
	"$ROOT/target/i686-unknown-linux-musl/release/podbox-interpose-build" \
	"$ROOT/target/debug/podbox-interpose-build"; do
	if [ -x "$cand" ]; then BIN="$cand"; break; fi
done
if [ -z "$BIN" ]; then
	echo "build-interpose.sh: podbox-interpose-build is not built in target/release/ or in any target/<triple>/release/; cargo build --release -p podbox-gate --bin podbox-interpose-build" >&2
	exit 2
fi
exec "$BIN" "$@"
