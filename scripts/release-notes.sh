#!/bin/sh
# release-notes.sh - compat shim for the port (T-1565). The logic lives in
# the release-notes binary in crates/podbox-release; this file only execs
# it with the same arguments, so the nightly publish step keeps its path.
# Usage: sh scripts/release-notes.sh TAG
# Exit: 0 the notes printed, 2 the exact-commit gate or required read is absent.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
ROOT="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN=""
for cand in "$ROOT/target/release/release-notes" \
	"$ROOT/target/x86_64-unknown-linux-musl/release/release-notes" \
	"$ROOT/target/x86_64-pc-windows-msvc/debug/release-notes" \
	"$ROOT/target/debug/release-notes"; do
	if [ -x "$cand" ]; then BIN="$cand"; break; fi
done
if [ -z "$BIN" ]; then
	echo "release-notes: the binary is not built; cargo build -p podbox-release" >&2
	exit 2
fi
exec "$BIN" "$@"
