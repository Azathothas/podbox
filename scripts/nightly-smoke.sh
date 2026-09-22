#!/bin/sh
# nightly-smoke.sh - the per-arch smoke TODO/packaging.md T-1314 promises.
#
# Usage: sh scripts/nightly-smoke.sh BINARY TRIPLE [QEMU]
#
# BINARY is the release binary for TRIPLE. QEMU names the user-mode emulator
# that runs it, and is empty where the binary runs natively. Four assertions,
# each read from the process that produced it:
#   1. `version` exits 0 and prints the artefact's version line;
#   2. `version --verbose` exits 0 and names TRIPLE as its target;
#   3. both interposer digests are 64 lowercase hex digits, never `absent`;
#   4. `crt-static` reads `yes`, and the binary carries no PT_INTERP.
#
# Exit: 0 the smoke is green, 1 it ran and something failed, 2 it could not run.
set -u

BIN="${1:-}"
TRIPLE="${2:-}"
QEMU="${3:-}"

fail() { echo "SMOKE-FAIL $1" >&2; exit 1; }
unrun() { echo "SMOKE-FAIL $1" >&2; exit 2; }

[ -n "$BIN" ] || unrun "usage: nightly-smoke.sh BINARY TRIPLE [QEMU]"
[ -n "$TRIPLE" ] || unrun "usage: nightly-smoke.sh BINARY TRIPLE [QEMU]"
[ -f "$BIN" ] || unrun "$TRIPLE: no binary at $BIN"
if [ -n "$QEMU" ]; then
  command -v "$QEMU" >/dev/null 2>&1 || unrun "$TRIPLE: no emulator $QEMU on PATH"
fi

run() {
  if [ -n "$QEMU" ]; then
    "$QEMU" "$BIN" "$@"
  else
    "$BIN" "$@"
  fi
}

out="$(run version 2>&1)"; rc=$?
[ "$rc" -eq 0 ] || fail "$TRIPLE: version exits $rc, not 0: $out"
case "$out" in
"podbox "*) ;;
*) fail "$TRIPLE: version prints [$out], not the version line" ;;
esac

doc="$(run version --verbose 2>&1)"; rc=$?
[ "$rc" -eq 0 ] || fail "$TRIPLE: version --verbose exits $rc, not 0"
printf '%s\n' "$doc"

line() { printf '%s\n' "$doc" | awk -F': ' -v k="$1" '$1 == k { print $2 }'; }

[ "$(line target)" = "$TRIPLE" ] || fail "$TRIPLE: verbose names target [$(line target)]"
[ "$(line crt-static)" = "yes" ] || fail "$TRIPLE: verbose names crt-static [$(line crt-static)], not yes"

hex64() {
  v="$(line "$1")"
  [ "${#v}" -eq 64 ] || fail "$TRIPLE: $1 is [$v], not a 64-digit digest"
  case "$v" in
  *[!0-9a-f]*) fail "$TRIPLE: $1 is [$v], not lowercase hex" ;;
  esac
}
hex64 interpose-gnu
hex64 interpose-musl

if command -v readelf >/dev/null 2>&1; then
  n="$(readelf -l "$BIN" | grep -c INTERP || true)"
  [ "$n" -eq 0 ] || fail "$TRIPLE: $n PT_INTERP entries, so the binary is not static"
else
  unrun "$TRIPLE: readelf is not installed, so static-ness is unchecked"
fi

echo "SMOKE-OK $TRIPLE qemu=${QEMU:-native} bytes=$(wc -c <"$BIN")"
