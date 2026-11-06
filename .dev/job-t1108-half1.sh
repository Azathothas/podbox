#!/bin/sh
# T-1108 Prove half 1: the shipped binary is static (no PT_INTERP).
# Exit 0: built and no INTERP. 1: built with an INTERP. 2: could not run.
set -u
cd /work || exit 2
OUT=/out
mkdir -p "$OUT" || exit 2
LOG="$OUT/t1108-half1.txt"
: >"$LOG" || exit 2

say() {
    printf '%s\n' "$*" >>"$LOG"
    printf '%s\n' "$*"
}

say "== conditions"
say "date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
say "head: $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
say "rustc: $(rustc --version 2>/dev/null || echo missing)"

say "== bootstrap (no docker; the lane has no NET_ADMIN)"
./scripts/common/bootstrap-env.sh rust cc zig tools >>"$LOG" 2>&1
rc=$?
say "bootstrap rc=$rc"
if [ "$rc" -ne 0 ]; then
    say "RESULT: could not run (bootstrap)"
    exit 2
fi

say "== interposer objects (build.rs embeds them)"
./scripts/build-interpose.sh >>"$LOG" 2>&1
rc=$?
say "interpose rc=$rc"
if [ "$rc" -ne 0 ]; then
    say "RESULT: build failed (interpose)"
    exit 1
fi

say "== musl release build"
if cargo build --release --target x86_64-unknown-linux-musl >>"$LOG" 2>&1; then
    say "build ok"
else
    say "RESULT: build failed"
    exit 1
fi

BIN="target/x86_64-unknown-linux-musl/release/podbox"
say "file: $(file "$BIN" 2>/dev/null || echo unknown)"
say "size: $(stat -c%s "$BIN" 2>/dev/null || echo unknown)"
n=$(readelf -l "$BIN" 2>>"$LOG" | grep -c INTERP || true)
say "INTERP count: $n"
if [ "$n" -eq 0 ]; then
    say "RESULT: static, no PT_INTERP"
    exit 0
fi
say "RESULT: INTERP present"
exit 1
