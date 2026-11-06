#!/usr/bin/env sh
# Bisect v3: empty file set. If the two digests agree here, the scratch
# difference is in values (the file side); if they still differ, it is
# in conditions or rendering. Hand digest for the empty set with the
# lane's conditions is 628234c2c0a643f02463558f593b6d4e460426043b7694865816bcce890f8835.
set -u

OUT=/out/bisect3-proof.log
: >"$OUT" || exit 2

say() { printf '%s\n' "$*" >>"$OUT"; }
fail=0
miss() { fail=1; say "  FAIL: $1"; }

BIN1=""
cargo build --release -p podbox-buildstate >>"$OUT" 2>&1
for cand in /work/target/release/podbox-buildstate /work/target/x86_64-unknown-linux-musl/release/podbox-buildstate /work/target/debug/podbox-buildstate; do
  [ -x "$cand" ] && BIN1="$cand" && break
done
[ -n "$BIN1" ] || { miss "no podbox-buildstate binary"; say "verdict BISECT3-FAIL"; exit 1; }
git show HEAD:scripts/build-state.py >"$OUT.orig.py" 2>&1

S4="$(mktemp -d)"
mkdir -p "$S4/crates/a"
for n in podbox node operator proxy shell; do echo "scratch-out" >"$S4/$n"; done

say "== empty file set under scratch/crates"
find "$S4" -mindepth 1 | sort >>"$OUT" 2>&1
A="$("$BIN1" inputs --root "$S4" --target t 2>>"$OUT")"
say "  binary inputs: [$A]"
B="$(python3 "$OUT.orig.py" inputs --root "$S4" --target t 2>>"$OUT")"
say "  script inputs: [$B]"
say "  hand empty-set: [628234c2c0a643f02463558f593b6d4e460426043b7694865816bcce890f8835]"
if [ -n "$A" ] && [ "$A" = "$B" ]; then say "  inputs agree on the empty set"; else miss "inputs differ on the empty set"; fi
rm -rf "$S4"
say ""

if [ "$fail" -eq 0 ]; then say "verdict BISECT3-OK"; else say "verdict BISECT3-FAIL"; fi
[ "$fail" -eq 0 ] || exit 1
exit 0
