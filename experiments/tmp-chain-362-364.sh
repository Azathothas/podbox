#!/bin/sh
# Temp chain, never committed: 362 builds the debug binary that 364 reuses,
# and both regenerate their results files with this lane's paths.
set -u
sh experiments/362-windows-refusal.sh || exit $?
sh experiments/364-windows-guest.sh || exit $?
if [ -d /out ]; then
	cp experiments/.sweep362-work/out-362.txt /out/windows-refusal.txt 2>/dev/null || true
	cp experiments/.sweep364-work/out-364.txt /out/windows-364.txt 2>/dev/null || true
fi
