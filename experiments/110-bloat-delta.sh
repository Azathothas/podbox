#!/usr/bin/env bash
# Question: how large is the release artefact, what is in it, and how much did
# the dependency under test add?
#
# TODO/deps.md T-0910. Declaration stub with a compat exec (T-1562). The
# measurement lives in the podbox-size binary in crates/podbox-release;
# this file stays because the total ceiling is declared here once and the
# gate, the plant harness, and the gate workflow all read it from here.
# (D-1 option c.)
#
#   ./110-bloat-delta.sh baseline       the "before": no dependencies at all
#   ./110-bloat-delta.sh <area>         the "after": one entry from TODO/deps.md
#   ./110-bloat-delta.sh ci             the gate's own area: measured, never asserted
#
# THE CEILING LIVES HERE AND NOWHERE ELSE. It is on the TOTAL, not on the
# delta: a delta ceiling permits an unbounded number of small dependencies,
# which is how a binary gets large without any single decision being wrong.
# Check 17 of the record gate holds that there is exactly one declaration
# of it in the tree. No Rust source repeats the digits below: the gate
# scans for them, so a literal turns the gate red.
#
# Exit: 0 measured and under the ceiling, 1 over the ceiling or the build
#       failed, 2 could not run.
set -uo pipefail

CEILING_BYTES=8000000

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN=""
for cand in "$REPO/target/release/podbox-size" \
	"$REPO/target/x86_64-unknown-linux-musl/release/podbox-size" \
	"$REPO/target/x86_64-pc-windows-msvc/debug/podbox-size" \
	"$REPO/target/debug/podbox-size"; do
	if [ -x "$cand" ]; then BIN="$cand"; break; fi
done
if [ -z "$BIN" ]; then
	echo "110-bloat-delta: podbox-size is not built; cargo build -p podbox-release" >&2
	exit 2
fi
exec "$BIN" "$@"
