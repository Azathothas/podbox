#!/usr/bin/env bash
# 35-interposition-reach.sh
#
# Question: which payload classes does an LD_PRELOAD interposer
# actually reach, and does reaching a payload's paths also reach its
# ownership operations?
#
# Three victims, one shim, same two syscalls each:
#   dynamic C   -> SEEN by the shim
#   static C    -> INVISIBLE (no dynamic loader, no preload at all)
#   Go          -> INVISIBLE (raw syscalls; cgo does not change this)
#
# Design lesson: interposition coverage is a property of the PAYLOAD.
# And a second lesson visible in the same run: even where the shim
# SEES lchown, the kernel still answers EINVAL — path interposition
# and ownership interposition are different jobs.
#
# Inputs: none. Tools: cc, go.
# Exit: 0 measured · 1 shim failed to intercept the dynamic victim · 2 could not build.
set -u
cd "$(dirname "$0")" || exit 2
. ../scripts/lib.sh

OUT="$SI_LOGS/35-interposition-reach.log"
{
  conditions interposition-reach
  echo
} > "$OUT"

D="$SI_WORK/ldpreload"; mkdir -p "$D"
cc -O2 -shared -fPIC -o "$D/shim.so" "$SI_SCRIPTS/ldpreload/shim.c" -ldl 2>>"$OUT" || { echo "shim build failed" >>"$OUT"; exit 2; }
cc -O2 -o "$D/victim_dyn" "$SI_SCRIPTS/ldpreload/victim_dyn.c" 2>>"$OUT" || exit 2
cc -O2 -static -o "$D/victim_static" "$SI_SCRIPTS/ldpreload/victim_static.c" 2>>"$OUT" || exit 2
(cd "$D" && go build -o victim_go "$SI_SCRIPTS/ldpreload/victim_go.go") >>"$OUT" 2>&1 || { echo "go victim build failed" >>"$OUT"; exit 2; }

{
  echo "## same shim, three payload classes"
  echo
  echo "### dynamic C (lchown/open through libc)"
  LD_PRELOAD="$D/shim.so" "$D/victim_dyn" 2>&1
  echo
  echo "### static C (no dynamic loader -> preload machinery never runs)"
  LD_PRELOAD="$D/shim.so" "$D/victim_static" 2>&1
  echo
  echo "### Go (os package issues raw syscalls, with or without cgo)"
  LD_PRELOAD="$D/shim.so" "$D/victim_go" 2>&1
} 2>&1 | tee -a "$OUT"

rc=0
# gate: the dynamic victim must show at least one interception line
if ! grep -q "SHIM: lchown" "$OUT"; then echo "GATE FAIL: shim did not intercept the dynamic victim" >> "$OUT"; rc=1; fi
# and the static + Go victims must show ZERO interception lines
if [ "$(grep -c 'SHIM:' <<< "$(sed -n '/static C/,/^$/p' "$OUT")")" -ne 0 ] || \
   [ "$(grep -c 'SHIM:' <<< "$(sed -n '/### Go/,/^$/p' "$OUT")")" -ne 0 ]; then
  echo "GATE FAIL: shim saw a payload class it must not reach" >> "$OUT"; rc=1
fi
echo
echo "log: $OUT"
exit $rc
