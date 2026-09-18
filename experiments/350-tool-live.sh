#!/bin/sh
# 350-tool-live.sh - prove text-tool and wsl-toolkit answer live on this host.
#
# Question: do the two ToolKit programs the docs name answer on this machine,
# and does the documented procedure match the binaries in front of us.
#
# Inputs: the `text-tool` and `wsl-toolkit` binaries on PATH. No image, no
# network, no base state is assumed. Every version is printed on the way out.
#
# The saved results redact the home prefix to `~`. The public gate refuses
# absolute home paths in tracked files, and the prefix carries no claim here.
# Byte evidence is read before redaction, and no redacted line carries one.
#
# Exit: 0 every leg that could run matched, 1 a leg ran and did not match,
# 2 a leg could not run (tool absent, base unusable).
set -u

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 2
ROOT=$(CDPATH= cd -- "$HERE/.." && pwd) || exit 2
OUT="$ROOT/experiments/results/tool-live.txt"
mkdir -p "$(dirname -- "$OUT")" || exit 2

TMPWORK=$(mktemp -d) || exit 2
trap 'rm -rf "$TMPWORK"' EXIT HUP INT TERM

FAIL=0
NORUN=0
note_fail() { FAIL=1; echo "MISMATCH: $1"; }
note_norun() { NORUN=1; echo "NORUN: $1"; }

{
echo "== conditions"
date -u +%Y-%m-%dT%H:%M:%SZ
uname -sr
echo "instance=podbox"
if command -v text-tool >/dev/null 2>&1; then command -v text-tool; else note_norun "text-tool is not on PATH"; fi
if command -v wsl-toolkit >/dev/null 2>&1; then command -v wsl-toolkit; else note_norun "wsl-toolkit is not on PATH"; fi

echo ""
echo "== text-tool answers"
if command -v text-tool >/dev/null 2>&1; then
  text-tool --help >"$TMPWORK/help.txt" 2>&1; rc=$?
  echo "help_exit=$rc"
  [ "$rc" = 0 ] || note_fail "text-tool --help exited $rc"
else
  note_norun "text-tool help"
fi

echo ""
echo "== text-tool write, append, edit with --expect, eol"
if command -v text-tool >/dev/null 2>&1; then
  SCRATCH=$(mktemp -d) || note_norun "could not create scratch"
  if [ -n "${SCRATCH:-}" ] && [ -d "$SCRATCH" ]; then
    F="$SCRATCH/notes.md"
    text-tool write "$F" --b64 SGVsbG8sICJ3b3JsZCIK >"$TMPWORK/write.txt" 2>&1; rc=$?
    echo "write_exit=$rc"
    [ "$rc" = 0 ] || note_fail "write exited $rc"
    text-tool append "$F" --text 'plain line' >"$TMPWORK/append.txt" 2>&1; rc=$?
    echo "append_exit=$rc"
    [ "$rc" = 0 ] || note_fail "append exited $rc"
    text-tool edit "$F" --replace 'plain line' --text 'second line' --expect 1 >"$TMPWORK/edit.txt" 2>&1; rc=$?
    echo "edit_exit=$rc"
    [ "$rc" = 0 ] || note_fail "edit exited $rc"
    text-tool edit "$F" --replace 'second line' --count >"$TMPWORK/count.txt" 2>&1; rc=$?
    echo "count_exit=$rc"
    cat "$TMPWORK/count.txt"
    text-tool edit "$F" --replace 'no-such-text' --text 'x' --expect 1 >"$TMPWORK/refuse.txt" 2>&1; rc=$?
    echo "wrong_expect_exit=$rc (want 1, nothing written)"
    [ "$rc" = 1 ] || note_fail "wrong --expect exited $rc, want 1"
    cat "$TMPWORK/refuse.txt"
    text-tool eol "$F" --lf >"$TMPWORK/eol.txt" 2>&1; rc=$?
    echo "eol_exit=$rc"
    [ "$rc" = 0 ] || note_fail "eol exited $rc"
    echo "--- file bytes ---"
    od -c "$F" | head -n 10
    rm -rf "$SCRATCH"
  fi
else
  note_norun "text-tool round trip"
fi

echo ""
echo "== wsl-toolkit answers"
if command -v wsl-toolkit >/dev/null 2>&1; then
  wsl-toolkit --version >"$TMPWORK/version.txt" 2>&1; rc=$?
  echo "version_exit=$rc"
  cat "$TMPWORK/version.txt"
  [ "$rc" = 0 ] || note_fail "version exited $rc"
  wsl-toolkit run --help >"$TMPWORK/runhelp.txt" 2>&1; rc=$?
  echo "run_help_exit=$rc"
  [ "$rc" = 0 ] || note_fail "run --help exited $rc"
  wsl-toolkit base --help >"$TMPWORK/basehelp.txt" 2>&1; rc=$?
  echo "base_help_exit=$rc"
  [ "$rc" = 0 ] || note_fail "base --help exited $rc"
  wsl-toolkit hostaddress --help >"$TMPWORK/hosthelp.txt" 2>&1; rc=$?
  echo "hostaddress_help_exit=$rc"
  [ "$rc" = 0 ] || note_fail "hostaddress --help exited $rc"
  wsl-toolkit man --no-pager >"$TMPWORK/man.txt" 2>&1; rc=$?
  echo "man_exit=$rc"
  [ "$rc" = 0 ] || note_fail "man exited $rc"
  head -n 5 "$TMPWORK/man.txt"
else
  note_norun "wsl-toolkit help"
fi

echo ""
echo "== wsl-toolkit doctor and readiness"
if command -v wsl-toolkit >/dev/null 2>&1; then
  timeout 60 wsl-toolkit doctor >"$TMPWORK/doctor.txt" 2>&1; rc=$?
  echo "doctor_exit=$rc"
  head -n 30 "$TMPWORK/doctor.txt"
  timeout 120 wsl-toolkit --instance podbox ready >"$TMPWORK/ready.txt" 2>&1; rc=$?
  echo "ready_exit=$rc"
  head -n 20 "$TMPWORK/ready.txt"
else
  note_norun "doctor and ready"
fi

echo ""
echo "== base status with probe"
if command -v wsl-toolkit >/dev/null 2>&1; then
  timeout 180 wsl-toolkit --instance podbox base status --probe >"$TMPWORK/status.txt" 2>&1; rc=$?
  echo "base_status_exit=$rc (want 0: usable true)"
  head -n 30 "$TMPWORK/status.txt"
  if [ "$rc" != 0 ]; then
    note_norun "base status --probe exited $rc: the container leg cannot run"
  fi
else
  note_norun "base status"
fi

echo ""
echo "== one tiny container through the whole path"
if [ "$NORUN" = 0 ]; then
  timeout 600 wsl-toolkit --instance podbox ready --smoke >"$TMPWORK/smoke.txt" 2>&1; rc=$?
  echo "smoke_exit=$rc (want 0)"
  head -n 20 "$TMPWORK/smoke.txt"
  if [ "$rc" != 0 ]; then
    note_norun "ready --smoke exited $rc: the container leg cannot run"
  fi
else
  echo "skipped: base is not usable"
fi

echo ""
echo "== verdict"
echo "fail=$FAIL norun=$NORUN"
echo "home_prefix_redacted_to=~"
} >"$TMPWORK/raw.txt" 2>&1
rc=$?
esc_sed() { printf '%s' "$1" | sed -e 's/[][\.*^$/\\|]/\\&/g'; }
SED_ARGS=""
if [ -n "${HOME:-}" ]; then SED_ARGS="$SED_ARGS -e s|$(esc_sed "$HOME")|~|g"; fi
if [ -n "${USERPROFILE:-}" ]; then
  SED_ARGS="$SED_ARGS -e s|$(esc_sed "$USERPROFILE")|~|g"
  SED_ARGS="$SED_ARGS -e s|$(esc_sed "$(printf '%s' "$USERPROFILE" | tr '\\' '/')")|~|g"
fi
if [ -n "$SED_ARGS" ]; then
  # shellcheck disable=SC2086
  sed $SED_ARGS "$TMPWORK/raw.txt" >"$OUT"
else
  cp "$TMPWORK/raw.txt" "$OUT"
fi
cat "$OUT"
if [ "$NORUN" = 1 ]; then exit 2; fi
if [ "$FAIL" = 1 ]; then exit 1; fi
exit "$rc"
