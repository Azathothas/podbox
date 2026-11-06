#!/bin/sh
# Build the shipped musl binary of the current tree and hand it out.
# Same shape as .dev/job-build.sh, with the target path the lane's
# .cargo/config.toml actually selects (build-interpose.sh resolves
# target/release/, which the musl target does not fill).
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
say "cargo: $(cargo --version 2>/dev/null || echo missing)"
say "host target: $(rustc -vV 2>/dev/null | sed -n 's/^host: //p')"

say "== bootstrap (no docker; the lane has no NET_ADMIN)"
./scripts/common/bootstrap-env.sh rust cc zig tools >>"$LOG" 2>&1
rc=$?
say "bootstrap rc=$rc"
if [ "$rc" -ne 0 ]; then
    say "RESULT: could not run (bootstrap)"
    exit 2
fi

say "== musl release build (the interposer objects embed through build.rs)"
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
say "sha256: $(sha256sum "$OUT/podbox" | cut -d' ' -f1)"
say "version: $("$OUT/podbox" version 2>&1 || echo 'could not run')"
say "RESULT: /out/podbox ready"
exit 0