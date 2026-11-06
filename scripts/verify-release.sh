#!/bin/sh
# verify-release.sh - compat shim for the port (T-1561). The logic lives in
# the podvm-verify binary in crates/podvm-release; this file only execs
# it with the same arguments, so the downloader's half keeps its path.
# Usage: sh scripts/verify-release.sh TAG ARCH [binary|ssh]
# Exit: 0 the bundle verifies, 1 it did not, 2 it could not run.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
ROOT="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN=""
for cand in "$ROOT/target/release/podvm-verify" \
	"$ROOT/target/x86_64-unknown-linux-musl/release/podvm-verify" \
	"$ROOT/target/x86_64-pc-windows-msvc/debug/podvm-verify" \
	"$ROOT/target/debug/podvm-verify"; do
	if [ -x "$cand" ]; then BIN="$cand"; break; fi
done
if [ -z "$BIN" ]; then
	echo "verify-release: podvm-verify is not built; cargo build -p podvm-release" >&2
	exit 2
fi
exec "$BIN" "$@"
