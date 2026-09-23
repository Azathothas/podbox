#!/bin/sh
# Question: is every byte the binary prints plain ASCII?
#
# TODO/cli.md T-1336. The binary's user-facing text carried marker glyphs
# (U+26D4, U+26A0, U+2B50) on dozens of paths: per-verb usage blocks,
# error messages, banner lines and parity notes. On a constrained host
# with no terminal or pty these are unrenderable bytes in an automated
# caller's stream. The scrub replaced each glyph with the word it stands
# for (`refused:`, `note:`; the star is deleted), comments untouched.
#
# This drives the binary rather than grepping the source: a passing grep
# over the tree cannot see a glyph built at runtime, and a passing binary
# is what the caller reads.
#
# ⛔ THE LANE SHAPE: a bare `run-in-base.sh` with this script as its
# argument stages this file at the checkout root as `.podbox-job.sh` and
# runs `sh` on it with cwd `/`, so `$0`'s directory is `/work` (never
# `experiments/`). The committed script therefore does NOT build: the
# lane job that drives it builds first and exports PODBOX_BIN at the
# guest path. A NATIVE run (`./experiments/352-ascii-output.sh` on
# Linux) builds the default first with
# `cargo build --release --target x86_64-unknown-linux-musl`, or sets
# PODBOX_BIN itself.
#
# ⚠ `set -u` and no `pipefail`: this script also runs under `sh` in a
# Linux job container, where `sh` is dash. Dash has no `pipefail`, and
# the option error would exit the whole job before a clause runs.
#
#   ./352-ascii-output.sh
#
# Exit: 0 every captured output is bytes 0x00-0x7F only, 1 one is not,
#       2 could not run.
set -u

# ⚠ In the lane `$0` is `/work/.podbox-job.sh`, so HERE is /work and REPO
# is `/`: BIN must NOT derive from REPO there. PODBOX_BIN is the lane
# job's export; the /work default covers a manual lane `sh` only.
HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-/work/target/x86_64-unknown-linux-musl/release/podbox}"
# ⚠ OUT is /out-absolute where the lane mounts artifacts, else the
# checkout's results dir natively. A results file that lands only in the
# lane workspace dies with the container; /out is what `--artifacts`
# copies back (docs/containers.md). Native OUT keeps the committed path
# the entry's Prove cites.
if [ -d /out ]; then
  OUT="/out/ascii-output.txt"
else
  OUT="$REPO/experiments/results/ascii-output.txt"
fi
mkdir -p "$(dirname -- "$OUT")" || exit 2
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT INT TERM

fail=0
say() { printf '%s\n' "$*" | tee -a "$OUT"; }

[ -x "$BIN" ] || { printf 'NO-BINARY: %s\n' "$BIN" | tee -a "$OUT"; exit 2; }

: >"$OUT"
say "== T-1336: every printed byte is plain ASCII"
say "== taken $(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || echo unknown-date) on $(uname -sr 2>/dev/null || echo unknown)"
say "== bin: $BIN"
"$BIN" version 2>/dev/null | head -1 >>"$OUT" || { say "NO-BINARY"; exit 2; }

# Byte check is one grep: LC_ALL=C so a UTF-8 locale does not fold a
# three-byte glyph into fragments. Offending lines are reported as line
# numbers with the bytes left in place (never re-printed raw: the
# results file itself must stay ASCII for the gate's own readers).
check_file() {
  # $1 label, $2 file: fail with the first offending lines shown.
  if LC_ALL=C grep -qP '[^\x00-\x7F]' "$2" 2>/dev/null; then
    say "NON-ASCII: $1"
    LC_ALL=C grep -nP '[^\x00-\x7F]' "$2" 2>/dev/null | LC_ALL=C tr -cd '0-9:\n' | head -5 >>"$OUT" || true
    fail=1
  else
    say "ASCII-OK: $1"
  fi
}

say "== every implemented verb's --help"
"$BIN" system info --format '{{json .Parity}}' >"$WORK/parity.json" 2>"$WORK/parity-err.txt" || { say "PARITY-FAILED"; exit 2; }
check_file "parity-json-stdout" "$WORK/parity.json"
# The verbs are read out of the binary's own table, never listed here:
# a new verb appears by construction. jq is the reader: every lane
# bootstraps the `tools` set (bootstrap-env.sh), which installs it.
command -v jq >/dev/null 2>&1 || { say "MISSING-TOOL-jq"; exit 2; }
jq -r '.[] | select(.flag == null) | .verb' "$WORK/parity.json" 2>/dev/null | sort -u >"$WORK/verbs.txt" || { say "VERBS-FAILED"; exit 2; }
while IFS= read -r verb; do
  [ -n "$verb" ] || continue
  case "$verb" in
    image) "$BIN" image --help >"$WORK/help-one.txt" 2>&1 ;;
    system) "$BIN" system --help >"$WORK/help-one.txt" 2>&1 ;;
    *) "$BIN" "$verb" --help >"$WORK/help-one.txt" 2>&1 ;;
  esac
  check_file "help-$verb" "$WORK/help-one.txt"
done <"$WORK/verbs.txt"

say "== podbox man, both pagings"
"$BIN" man --no-pager >"$WORK/man-nopager.txt" 2>"$WORK/man-nopager-err.txt" || { say "MAN-FAILED"; exit 1; }
check_file "man-no-pager-stdout" "$WORK/man-nopager.txt"
check_file "man-no-pager-stderr" "$WORK/man-nopager-err.txt"
"$BIN" man >"$WORK/man-piped.txt" 2>"$WORK/man-piped-err.txt" || { say "MAN-PIPED-FAILED"; exit 1; }
check_file "man-piped-stdout" "$WORK/man-piped.txt"
check_file "man-piped-stderr" "$WORK/man-piped-err.txt"

say "== a catalog of error paths"
"$BIN" run --badflag >"$WORK/err-badflag.txt" 2>&1 || true
check_file "err-flag-refusal" "$WORK/err-badflag.txt"
"$BIN" frobnicate >"$WORK/err-badverb.txt" 2>&1 || true
check_file "err-verb-refusal" "$WORK/err-badverb.txt"
"$BIN" run --rm --memory=1g public.ecr.aws/docker/library/alpine:3.20 true >"$WORK/err-noneflag.txt" 2>&1 || true
check_file "err-none-flag" "$WORK/err-noneflag.txt"
"$BIN" inspect no-such-thing-xyz >"$WORK/err-inspect.txt" 2>&1 || true
check_file "err-inspect-missing" "$WORK/err-inspect.txt"
"$BIN" system install-names --help >"$WORK/err-install.txt" 2>&1 || true
check_file "help-install-names" "$WORK/err-install.txt"

say "== verdict fail=$fail"
exit "$fail"
