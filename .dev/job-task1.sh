#!/bin/sh
# task-1 re-measurement for TODO/image.md T-1310, on the unmodified tree.
# Parallel runs of the podbox-image suite to first refusal or N, then two
# serial controls. Exit 0: refusal reproduced. 1: N runs, no refusal.
# 2: could not run.
set -u
cd /work || exit 2
OUT=/out
mkdir -p "$OUT" || exit 2
LOG="$OUT/task1-measure.txt"
N=10
: >"$LOG" || exit 2

say() {
    printf '%s\n' "$*" >>"$LOG"
    printf '%s\n' "$*"
}

say "== conditions"
say "date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
say "head: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
say "nproc: $(nproc 2>/dev/null || echo unknown)"
say "rustc: $(rustc --version 2>/dev/null || echo missing)"
say "cargo: $(cargo --version 2>/dev/null || echo missing)"
say "suite tests: (counted after compile)"

say "== bootstrap (the gate's own setup: no docker; the lane has no NET_ADMIN)"
./scripts/common/bootstrap-env.sh rust cc zig tools >>"$LOG" 2>&1
rc=$?
say "bootstrap rc=$rc"
if [ "$rc" -ne 0 ]; then
    say "RESULT: could not run (bootstrap)"
    exit 2
fi

say "== compile the suite once"
timeout 1200 cargo test -p podbox-image --no-run >>"$LOG" 2>&1
rc=$?
say "compile rc=$rc"
if [ "$rc" -ne 0 ]; then
    say "RESULT: could not run (compile)"
    exit 2
fi
say "suite tests: $(cargo test -p podbox-image -- --list 2>>"$LOG" | grep -c ': test' || true)"

found=0
i=1
while [ "$i" -le "$N" ]; do
    say "== parallel run $i of $N"
    timeout 600 cargo test -p podbox-image >"$OUT/run-parallel-$i.log" 2>&1
    rc=$?
    hits=$(grep -c "already holds 16 locks" "$OUT/run-parallel-$i.log" || true)
    passed=$(grep -c "^test .* ok$" "$OUT/run-parallel-$i.log" || true)
    failed=$(grep -c "^test .* FAILED$" "$OUT/run-parallel-$i.log" || true)
    say "run $i: rc=$rc passed=$passed failed=$failed slot16_hits=$hits"
    if [ "$hits" -gt 0 ]; then
        found=$((found + 1))
        say "refusing tests:"
        grep "^test .* FAILED$" "$OUT/run-parallel-$i.log" >>"$LOG" 2>&1 || true
    fi
    if [ "$rc" -ne 0 ] && [ "$OUT/run-parallel-$i.log" != "" ]; then
        tail -5 "$OUT/run-parallel-$i.log" >>"$LOG" 2>&1
    fi
    i=$((i + 1))
done

say "== serial controls (2 runs, --test-threads=1)"
j=1
serial_bad=0
while [ "$j" -le 2 ]; do
    timeout 900 cargo test -p podbox-image -- --test-threads=1 >"$OUT/run-serial-$j.log" 2>&1
    rc=$?
    say "serial $j: rc=$rc"
    if [ "$rc" -ne 0 ]; then
        serial_bad=$((serial_bad + 1))
    fi
    j=$((j + 1))
done

say "== summary"
say "parallel: $N runs, $found with the 16-slot signature"
say "serial: 2 runs, $serial_bad failed"
if [ "$found" -gt 0 ]; then
    say "RESULT: refusal reproduced ($found of $N)"
    exit 0
fi
say "RESULT: no refusal in $N parallel runs"
exit 1
