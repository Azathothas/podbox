#!/usr/bin/env bash
# Compat shim for the port (T-1568). The podvm parity proof lives in the
# podbox-podvm binary in crates/podbox-podvm; this file only execs it with
# the same arguments, so TODO/podvm.md T-1302 keeps its path. Nothing here
# pulls an image; on a Windows host run it inside the base with PODBOX_BIN
# set to a guest build artifact.
#
# Exit: 0 every row green, 1 a row disagreed, 2 the binary or the table
# could not run.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN=""
for cand in "$REPO/target/release/podbox-podvm" \
	"$REPO/target/x86_64-unknown-linux-musl/release/podbox-podvm" \
	"$REPO/target/x86_64-pc-windows-msvc/debug/podbox-podvm" \
	"$REPO/target/debug/podbox-podvm"; do
	if [ -x "$cand" ]; then BIN="$cand"; break; fi
done
if [ -z "$BIN" ]; then
	echo "145-podvm-parity: podbox-podvm is not built; cargo build -p podbox-podvm" >&2
	exit 2
fi
exec "$BIN" "$@"
