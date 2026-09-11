#!/usr/bin/env bash
# Question: preloopdev/preloop v0.32.5 — "local CI with hardware isolation".
# Its own binary strings reference /dev/kvm ("the VM pool cannot open
# /dev/kvm"); does the local engine start here?
# Exit codes: 0 usable, 1 blocked/failed, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/44-preloop.log"
conditions preloop > "$LOG"
BIN="$VR_WORK/preloop/preloop-x86_64"
[ -d "$BIN" ] || { echo "binary missing" >> "$LOG"; exit 2; }
{
  echo "--- version:"; timeout 15 "$BIN/preloop" --version 2>&1
  echo "--- [V] kvm references in the binary:"
  strings "$BIN/preloop" | grep -E '/dev/kvm|kvm group' | head -3
  echo "--- local run attempt:"
  ( cd "$VR_WORK/preloop" && timeout 60 "$BIN/preloop" run 2>&1 | head -5 )
  echo "--- rc=$?"
  echo "--- verdict: engine exits before ready; VM pool documented as /dev/kvm-dependent => BLOCKED"
} >> "$LOG" 2>&1
tail -10 "$LOG"
exit 1
