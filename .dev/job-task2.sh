#!/bin/sh
# job 2 for TODO/image.md T-1310: run the 326 prove script, then the full gate.
# Exit 0 iff both are green. 1 if either ran and failed. 2 if setup failed.
set -u
cd /work || exit 2
OUT=/out
mkdir -p "$OUT" || exit 2
LOG="$OUT/job2.txt"
: >"$LOG" || exit 2

say() {
    printf '%s\n' "$*" >>"$LOG"
    printf '%s\n' "$*"
}

say "== conditions"
say "date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
say "head: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
say "nproc: $(nproc 2>/dev/null || echo unknown)"

say "== bootstrap (no docker; the lane has no NET_ADMIN)"
./scripts/common/bootstrap-env.sh rust cc zig tools >>"$LOG" 2>&1
rc=$?
say "bootstrap rc=$rc"
if [ "$rc" -ne 0 ]; then
    say "RESULT: could not run (bootstrap)"
    exit 2
fi

say "== 326-store-contention-prove.sh"
sh experiments/326-store-contention-prove.sh >>"$LOG" 2>&1
rc326=$?
say "326 rc=$rc326"
cp experiments/results/store-contention-prove.txt "$OUT/" 2>/dev/null || true

say "== dev.sh check (the full gate)"
./scripts/dev.sh check >>"$LOG" 2>&1
rccheck=$?
say "check rc=$rccheck"

say "== summary: 326=$rc326 check=$rccheck"
if [ "$rc326" -eq 0 ] && [ "$rccheck" -eq 0 ]; then
    say "RESULT: both green"
    exit 0
fi
say "RESULT: ran and something failed"
exit 1
