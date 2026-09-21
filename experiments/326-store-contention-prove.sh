#!/bin/sh
# 326-store-contention-prove.sh: is the podbox-image suite deterministic under
# default parallelism, and is the sixteen-slot ceiling pinned by test?
#
# Acceptance instrument for TODO/image.md T-1310. It runs the suite N times
# with libtest's default thread count and counts the 16-slot signature, runs
# the ceiling test alone, runs one serial control, and audits structurally
# that every test in the store module takes the suite mutex.
#
# Exit 0 the measurement ran and matched, 1 it ran and something under test
# failed, 2 it could not run.
set -u
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 2
ROOT=$(CDPATH= cd -- "$HERE/.." && pwd) || exit 2
cd "$ROOT" || exit 2
RESULTS="$ROOT/experiments/results/store-contention-prove.txt"
RUNS="${PODBOX_PROVE_RUNS:-10}"

mkdir -p "$ROOT/experiments/results" || exit 2
: >"$RESULTS" || exit 2

say() {
    printf '%s\n' "$*" >>"$RESULTS"
    printf '%s\n' "$*"
}

say "== conditions"
say "date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
say "host: $(uname -sr 2>/dev/null || echo unknown)"
say "head: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
say "nproc: $(nproc 2>/dev/null || echo unknown)"
say "rustc: $(rustc --version 2>/dev/null || echo missing)"
say "cargo: $(cargo --version 2>/dev/null || echo missing)"
say "runs: $RUNS"

say "== compile the suite once"
if ! timeout 1200 cargo test -p podbox-image --no-run >>"$RESULTS" 2>&1; then
    say "RESULT: could not run (compile)"
    exit 2
fi
say "suite tests: $(cargo test -p podbox-image -- --list 2>>"$RESULTS" | grep -c ': test' || true)"

red=0

say "== clause 1: the suite $RUNS times with default parallelism"
i=1
found=0
while [ "$i" -le "$RUNS" ]; do
    log="$ROOT/experiments/results/.prove-326-parallel-$i.log"
    if timeout 600 cargo test -p podbox-image >"$log" 2>&1; then
        rc=0
    else
        rc=$?
    fi
    hits=$(grep -c "already holds 16 locks" "$log" || true)
    failed=$(grep -c "^test .* FAILED$" "$log" || true)
    say "run $i: rc=$rc failed=$failed slot16_hits=$hits"
    if [ "$hits" -gt 0 ]; then
        found=$((found + 1))
        grep "^test .* FAILED$" "$log" >>"$RESULTS" 2>&1 || true
    fi
    if [ "$rc" -ne 0 ]; then
        red=1
        tail -5 "$log" >>"$RESULTS" 2>&1
    fi
    rm -f "$log"
    i=$((i + 1))
done
say "clause 1: $found of $RUNS runs carry the 16-slot signature"

say "== clause 2: the ceiling test alone, three times"
i=1
while [ "$i" -le 3 ]; do
    if timeout 300 cargo test -p podbox-image the_seventeenth_concurrent_registration_is_refused_by_name >>"$RESULTS" 2>&1; then
        say "ceiling $i: pass"
    else
        say "ceiling $i: FAIL"
        red=1
    fi
    i=$((i + 1))
done

say "== clause 3: one serial control"
if timeout 900 cargo test -p podbox-image -- --test-threads=1 >>"$RESULTS" 2>&1; then
    say "serial: pass"
else
    say "serial: FAIL"
    red=1
fi

say "== clause 4: every store test takes the suite mutex"
tests=$(awk '/^    #\[test\]$/{t=1;next} t==1 && /    fn [a-z]/{c++;t=0} END{print c+0}' crates/podbox-image/src/store.rs)
takes=$(grep -c "STORE_TESTS.lock()" crates/podbox-image/src/store.rs || true)
say "test functions: $tests, acquisitions: $takes"
if [ "$tests" -eq "$takes" ] && [ "$tests" -gt 0 ]; then
    say "audit: equal"
else
    say "audit: MISMATCH (a test without the mutex reintroduces the flake silently)"
    red=1
fi

say "== summary"
if [ "$red" -eq 0 ] && [ "$found" -eq 0 ]; then
    say "RESULT: $RUNS of $RUNS parallel runs green, no 16-slot signature; ceiling, serial and audit green"
    exit 0
fi
say "RESULT: ran and something failed (red=$red, slot16 runs=$found of $RUNS)"
exit 1
