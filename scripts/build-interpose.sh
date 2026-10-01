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
BIN="$ROOT/target/release/podbox-interpose-build"
if [ ! -x "$BIN" ]; then
	echo "build-interpose.sh: $BIN is not built. cargo build --release -p podbox-gate" >&2
	exit 2
fi
exec "$BIN" "$@"
