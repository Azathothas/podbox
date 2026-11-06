#!/usr/bin/env sh
# Bisect v4: print the three tool versions under cwd=/work and under a
# scratch cwd. The retired script probes them in its invocation cwd
# (no cwd= argument); the binary probes them in --root. Any difference
# by cwd is the cross-read divergence.
set -u

OUT=/out/bisect4-proof.log
: >"$OUT" || exit 2

say() { printf '%s\n' "$*" >>"$OUT"; }

S4="$(mktemp -d)"
say "job cwd: $(pwd)"
say ""
say "== tool probes with cwd=/work"
cd /work || exit 2
say "  rustc: $(rustc --version 2>&1)"
say "  cargo: $(cargo --version 2>&1)"
say "  zig: $(zig version 2>&1)"
say "  which rustc: $(command -v rustc)"
say "  rustup toolchains: $(rustup toolchain list 2>&1 || echo no-rustup)"
say ""
say "== tool probes with cwd=$S4"
cd "$S4" || exit 2
say "  rustc: $(rustc --version 2>&1)"
say "  cargo: $(cargo --version 2>&1)"
say "  zig: $(zig version 2>&1)"
say "  which rustc: $(command -v rustc)"
rm -rf "$S4"
say ""
say "verdict BISECT4-OK"
exit 0
