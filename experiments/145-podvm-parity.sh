#!/usr/bin/env bash
# Compat shim for the port (T-1568). The help parity proof lives in the
# podvm-parity binary in crates/podvm-measure; this file only execs it with
# the same arguments, so TODO/machine.md T-1302 keeps its path. Nothing here
# pulls an image; on a Windows host run it inside the base with PODVM_BIN
# set to a guest build artifact.
#
# Exit: 0 every row green, 1 a row disagreed, 2 the binary or the table
# could not run.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN=""
for cand in "$REPO/target/release/podvm-parity" \
	"$REPO/target/x86_64-unknown-linux-musl/release/podvm-parity" \
	"$REPO/target/x86_64-pc-windows-msvc/debug/podvm-parity" \
	"$REPO/target/debug/podvm-parity"; do
	if [ -x "$cand" ]; then BIN="$cand"; break; fi
done
if [ -z "$BIN" ]; then
	echo "145-podvm-parity: podvm-parity is not built; cargo build -p podvm-measure" >&2
	exit 2
fi
exec "$BIN" "$@"
