#!/bin/sh
# T-1208 evidence job: the full gate plus the plant harness on the committed
# tree. Exit 0 iff both are green. 1 if either ran and failed. 2 on setup
# failure.
set -u
cd /work || exit 2
OUT=/out
mkdir -p "$OUT" || exit 2
LOG="$OUT/job-gateplant.txt"
: >"$LOG" || exit 2

say() {
    printf '%s\n' "$*" >>"$LOG"
    printf '%s\n' "$*"
}

say "== conditions"
say "date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
say "head: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
say "tree clean: $(git status --porcelain 2>/dev/null | wc -l | tr -d ' ') dirty files"

say "== bootstrap (no docker; the lane has no NET_ADMIN)"
./scripts/common/bootstrap-env.sh rust cc zig tools >>"$LOG" 2>&1
rc=$?
say "bootstrap rc=$rc"
if [ "$rc" -ne 0 ]; then
    say "RESULT: could not run (bootstrap)"
    exit 2
fi

say "== dev.sh check (the full gate)"
./scripts/dev.sh check >>"$LOG" 2>&1
rccheck=$?
say "check rc=$rccheck"

say "== plant.sh (every check planted against)"
./scripts/plant.sh >>"$LOG" 2>&1
rcplant=$?
say "plant rc=$rcplant"
grep -E "plants|controls|every check" "$LOG" | tail -n 5

say "== summary: check=$rccheck plant=$rcplant"
if [ "$rccheck" -eq 0 ] && [ "$rcplant" -eq 0 ]; then
    say "RESULT: both green"
    exit 0
fi
say "RESULT: ran and something failed"
exit 1
