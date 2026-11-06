#!/usr/bin/env sh
# Bisect the buildstate cross-read mismatch: binary record read by the
# retired script (and vice versa) reports `stale: build inputs changed`
# on a scratch root in the lane, while the same pair agrees on /work
# and on a native scratch root. Prints both digests and every condition.
set -u

OUT=/out/bisect-proof.log
: >"$OUT" || exit 2

say() { printf '%s\n' "$*" >>"$OUT"; }
fail=0
miss() { fail=1; say "  FAIL: $1"; }

BIN1=""
cargo build --release -p podbox-buildstate >>"$OUT" 2>&1
say "cargo build rc=$? (want 0)"
for cand in /work/target/release/podbox-buildstate /work/target/x86_64-unknown-linux-musl/release/podbox-buildstate /work/target/debug/podbox-buildstate; do
  [ -x "$cand" ] && BIN1="$cand" && break
done
[ -n "$BIN1" ] || { miss "no podbox-buildstate binary"; say "verdict BISECT-FAIL"; exit 1; }
git show HEAD:scripts/build-state.py >"$OUT.orig.py" 2>&1

S4="$(mktemp -d)"
mkdir -p "$S4/crates/a"
echo "scratch-input" >"$S4/crates/a/f.rs"
for n in podbox node operator proxy shell; do echo "scratch-out" >"$S4/$n"; done
say "scratch: $S4"
say "tmpdir: ${TMPDIR:-unset}"
say "resolved: $(cd "$S4" && pwd -P)"
say ""

say "== conditions both sides hash"
for key in RUSTFLAGS CARGO_ENCODED_RUSTFLAGS CC AR TARGETS; do
  eval "say \"  env $key=[\${$key:-unset}]\""
done
env | grep -E "^(CARGO_TARGET_|CC_|AR_)" >>"$OUT" 2>&1 || say "  no CARGO_TARGET_/CC_/AR_ keys"
say "  rustc: $(rustc --version 2>&1)"
say "  cargo: $(cargo --version 2>&1)"
say "  zig: $(zig version 2>&1)"
say "  git rev-parse in scratch rc=$? below:"
git -C "$S4" rev-parse HEAD >>"$OUT" 2>&1
say "  rev-parse rc=$?"
say ""

say "== the two digests on the scratch root"
A="$("$BIN1" inputs --root "$S4" --target t 2>>"$OUT")"
say "  binary inputs: [$A]"
B="$(python3 "$OUT.orig.py" inputs --root "$S4" --target t 2>>"$OUT")"
say "  script inputs: [$B]"
if [ -n "$A" ] && [ "$A" = "$B" ]; then say "  inputs agree"; else miss "inputs differ on scratch"; fi
say ""

say "== self reads on the scratch root"
"$BIN1" record --root "$S4" --target t --binary "$S4/podbox" >>"$OUT" 2>&1
say "  binary record rc=$?"
"$BIN1" status --root "$S4" --target t --binary "$S4/podbox" >>"$OUT" 2>&1
say "  binary status of binary record rc=$? (want 0)"
rm -f "$S4/.dev/build-state.json"
python3 "$OUT.orig.py" record --root "$S4" --target t --binary "$S4/podbox" >>"$OUT" 2>&1
say "  script record rc=$?"
python3 "$OUT.orig.py" status --root "$S4" --target t --binary "$S4/podbox" >>"$OUT" 2>&1
say "  script status of script record rc=$? (want 0)"
say ""

say "== cross reads on the scratch root"
"$BIN1" record --root "$S4" --target t --binary "$S4/podbox" >>"$OUT" 2>&1
python3 "$OUT.orig.py" status --root "$S4" --target t --binary "$S4/podbox" >>"$OUT" 2>&1
say "  script status of binary record rc=$? (want 0)"
rm -f "$S4/.dev/build-state.json"
python3 "$OUT.orig.py" record --root "$S4" --target t --binary "$S4/podbox" >>"$OUT" 2>&1
"$BIN1" status --root "$S4" --target t --binary "$S4/podbox" >>"$OUT" 2>&1
say "  binary status of script record rc=$? (want 0)"
rm -rf "$S4"
say ""

if [ "$fail" -eq 0 ]; then say "verdict BISECT-OK"; else say "verdict BISECT-FAIL"; fi
[ "$fail" -eq 0 ] || exit 1
exit 0
