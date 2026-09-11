#!/usr/bin/env bash
# Question: virtkit-dev/virtkit v0.66.0 ("Rootless microVM toolkit") — the
# README says vk needs read/write access to /dev/kvm; rootless here still has
# no node, so which line does it die on?
# Exit codes: 0 usable, 1 blocked, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/41-virtkit.log"
conditions virtkit > "$LOG"
BIN="$VR_WORK/bins/vk"
[ -x "$BIN" ] || { echo "binary missing" >> "$LOG"; exit 2; }
echo "--- every verb segfaults (rc 139):" >> "$LOG"
for v in --version version --help list new run; do
  echo -n "vk $v -> " >> "$LOG"
  timeout 15 "$BIN" $v >> "$LOG" 2>&1
  echo "rc=$?" >> "$LOG"
done
echo "--- strings | grep kvm (what it looks for):" >> "$LOG"
strings "$BIN" | grep -iE "^/dev/kvm|kvm" | sort -u | head -5 >> "$LOG"
tail -20 "$LOG"
exit 1
