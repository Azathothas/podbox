#!/bin/sh
# Build the shipped musl binary of the current tree and hand it out.
# Exit 0 with /out/podbox executable, 1 on build failure, 2 on setup failure.
set -u
cd /work || exit 2
OUT=/out
mkdir -p "$OUT" || exit 2
LOG="$OUT/build.txt"
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
cp "$BIN" "$OUT/podbox" || exit 2
chmod +x "$OUT/podbox"
say "file: $(file "$OUT/podbox" 2>/dev/null || echo unknown)"
say "version: $("$OUT/podbox" version 2>&1 || echo 'could not run')"
say "RESULT: /out/podbox ready"
exit 0
