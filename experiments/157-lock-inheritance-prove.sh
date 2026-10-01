#!/usr/bin/env bash
# Compat shim for the port (T-1563). The lock-inheritance proof lives in the
# podbox-prove-t0211 binary in crates/podbox-release; this file only execs
# it with the matching subcommand, so TODO/image.md T-0211 keeps its path.
# The binary encodes the current anchors: the guard in `Lock::try_acquire`
# and the one flag in `Lock::open`. A stale anchor passes vacuously.
#
#   ./157-lock-inheritance-prove.sh
#   PODBOX_PROVE_RUNS=30 ./157-lock-inheritance-prove.sh
#
# Exit: 0 every attempt passed and both mutations were caught exactly once,
#       1 an attempt failed or a mutation was not caught, 2 could not run.
set -uo pipefail

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
	echo "157-lock-inheritance-prove: podbox-prove-t0211 is not built; cargo build -p podbox-release" >&2
	exit 2
fi
exec "$BIN" lock-inheritance "$@"
