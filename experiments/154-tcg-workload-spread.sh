#!/usr/bin/env bash
# Compat shim for the port (T-1569). The workload spread measurement lives
# in the podvm-workload binary in crates/podvm-measure; this file
# only execs it with the same arguments, so TODO/machine.md T-1308 keeps its
# path. On a Windows host run it inside the base with PODVM_BIN set to a
# guest build artifact.
#
# Exit: 0 every row printed with agreeing checksums, 1 a checksum
# disagreed or a run failed, 2 a tool, the binary or an input could not run.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN=""
for cand in "$REPO/target/release/podvm-workload" \
	"$REPO/target/x86_64-unknown-linux-musl/release/podvm-workload" \
	"$REPO/target/x86_64-pc-windows-msvc/debug/podvm-workload" \
	"$REPO/target/debug/podvm-workload"; do
	if [ -x "$cand" ]; then BIN="$cand"; break; fi
done
if [ -z "$BIN" ]; then
	echo "154-tcg-workload-spread: podvm-workload is not built; cargo build -p podvm-measure" >&2
	exit 2
fi
exec "$BIN" "$@"
