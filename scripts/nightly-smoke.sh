#!/bin/sh
# nightly-smoke.sh - the per-arch smoke TODO/packaging.md T-1314 promises,
# extended by TODO/packaging.md T-1329.
#
# Usage: sh scripts/nightly-smoke.sh BINARY TRIPLE [QEMU]
#
# BINARY is the release binary for TRIPLE. QEMU names the user-mode emulator
# that runs it, and is empty where the binary runs natively. Five assertion
# groups, each read from the process that produced it:
#   1. `version` exits 0 and prints the artefact's version line;
#   2. `version --verbose` exits 0 and names TRIPLE as its target;
#   3. both interposer digests are 64 lowercase hex digits, never `absent`;
#   4. `crt-static` reads `yes`, and the binary carries no PT_INTERP;
#   5. pull plus extract asserts the payload bytes (T-1329): a seeded
#      one-file image travels by save/load through the binary under
#      test (import, save, load, extract, payload bytes read back). A
#      fixture whose layer bytes are flipped fails the load with a
#      digest mismatch; the honest one passes. No registry, no quota,
#      no network. Native legs only: groups 1-4 run on all seven legs,
#      group 5 where the binary runs natively.
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

# Group 5, TODO/packaging.md T-1329: the binary pulls and extracts for
# its actual job, not only versions. A synthetic one-file image travels
# by save/load through the binary under test: `save` writes it as an
# OCI-layout tarball, `load` reads it into a scratch store, `extract`
# unpacks it, and the payload bytes are read back. Then the tarball's
# layer bytes are flipped and the same load must refuse: a smoke that
# passes a corrupted fixture is the green publish of a broken binary
# the entry exists to stop. No registry, no quota, no network; the
# store is scratch (`PODBOX_STORE` pointed at a temp dir) so no state
# leaks between legs. `run` stays out: qemu-user execution waits on
# T-1327's per-arch objects.
if [ -z "$QEMU" ]; then
  SMOKE_WORK="$(mktemp -d)" || unrun "$TRIPLE: no temp dir"
  # shellcheck disable=SC2064
  trap "rm -rf '$SMOKE_WORK'" EXIT INT TERM
  export PODBOX_STORE="$SMOKE_WORK/store"
  SEED_DIR="$SMOKE_WORK/seed"
  mkdir -p "$SEED_DIR" || unrun "$TRIPLE: no seed dir"
  printf 'smoke-payload\n' >"$SEED_DIR/payload.txt" || unrun "$TRIPLE: no seed file"
  (cd "$SEED_DIR" && tar -cf "$SMOKE_WORK/rootfs.tar" payload.txt) || unrun "$TRIPLE: no seed tar"
  "$BIN" import "$SMOKE_WORK/rootfs.tar" "smoke:t1" >/dev/null 2>&1 || fail "$TRIPLE: import of the seed tar exits non-zero"
  "$BIN" save -o "$SMOKE_WORK/seed.tar" "smoke:t1" >/dev/null 2>&1 || fail "$TRIPLE: save of the seed image exits non-zero"
  "$BIN" load -i "$SMOKE_WORK/seed.tar" >/dev/null 2>&1 || fail "$TRIPLE: load of the honest tarball exits non-zero"
  "$BIN" extract "smoke:t1" >/dev/null 2>&1 || fail "$TRIPLE: extract of the loaded image exits non-zero"
  GOT="$("$BIN" inspect --format '{{.RootfsPath}}' "smoke:t1" 2>/dev/null)" || fail "$TRIPLE: inspect of the loaded image exits non-zero"
  [ "$(cat "$GOT/payload.txt" 2>/dev/null)" = "smoke-payload" ] || fail "$TRIPLE: payload bytes differ after extract"
  echo "SMOKE-PULL-EXTRACT-OK $TRIPLE honest leg: pull-by-load plus extract reads the payload bytes"
  cp "$SMOKE_WORK/seed.tar" "$SMOKE_WORK/evil.tar"
  # Flip the layer blob inside the store-format tarball, not the outer
  # tar framing: `load` hashes each entry's bytes against the descriptor
  # that names it, so a flipped ENTRY fails closed with a digest
  # mismatch. Flipping the outer tar's own bytes only corrupts GNU tar
  # headers the loader never hashes.
  EVIL_BLOB="$(tar -tf "$SMOKE_WORK/seed.tar" | grep 'blobs/sha256/' | head -1)"
  [ -n "$EVIL_BLOB" ] || fail "$TRIPLE: no blob entry in the seed tarball"
  mkdir -p "$SMOKE_WORK/evil" && tar -xf "$SMOKE_WORK/seed.tar" -C "$SMOKE_WORK/evil" || fail "$TRIPLE: no repack dir"
  printf 'X' | dd of="$SMOKE_WORK/evil/$EVIL_BLOB" bs=1 seek=100 count=1 conv=notrunc 2>/dev/null || unrun "$TRIPLE: no dd"
  (cd "$SMOKE_WORK/evil" && tar -cf "$SMOKE_WORK/evil.tar" oci-layout index.json blobs) || fail "$TRIPLE: no repack"
  if "$BIN" load -i "$SMOKE_WORK/evil.tar" >/dev/null 2>&1; then
    fail "$TRIPLE: load of the flipped tarball exits 0"
  fi
  echo "SMOKE-PULL-EXTRACT-OK $TRIPLE flipped leg: the corrupted fixture refuses"
  rm -rf "$SMOKE_WORK"
  trap - EXIT INT TERM
fi

echo "SMOKE-OK $TRIPLE qemu=${QEMU:-native} bytes=$(wc -c <"$BIN")"
