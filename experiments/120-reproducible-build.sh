#!/bin/sh
# Compat shim for the port (T-1563). The reproducible-build proof lives in
# the podbox-prove-t0211 binary in crates/podbox-release; this file only
# execs it with the matching subcommand, so TODO/packaging.md T-1004 keeps
# its path.
#
#   ./120-reproducible-build.sh
#
# Exit: 0 the bytes match, 1 they do not, 2 the builds could not run.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN=""
for cand in "$REPO/target/release/podbox-prove-t0211" \
	"$REPO/target/x86_64-unknown-linux-musl/release/podbox-prove-t0211" \
	"$REPO/target/x86_64-pc-windows-msvc/debug/podbox-prove-t0211" \
	"$REPO/target/debug/podbox-prove-t0211"; do
	if [ -x "$cand" ]; then BIN="$cand"; break; fi
done
if [ -z "$BIN" ]; then
	echo "120-reproducible-build: podbox-prove-t0211 is not built; cargo build -p podbox-release" >&2
	exit 2
fi
exec "$BIN" reproducible "$@"
