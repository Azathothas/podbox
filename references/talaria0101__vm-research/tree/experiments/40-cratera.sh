#!/usr/bin/env bash
# Question: cratera-project/cratera v1.2.0 — its own `doctor` command reports
# exactly which host facilities the engine needs (/dev/kvm, jailer, kernel);
# which ones pass in this sandbox?
# Exit codes: 0 engine usable, 1 blocked (doctor output is the evidence), 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/40-cratera.log"
conditions cratera > "$LOG"
BIN="$VR_WORK/bins/cratera/cratera-v1.2.0-linux-x86_64/cratera"
[ -x "$BIN" ] || { echo "binary missing" >> "$LOG"; exit 2; }
timeout 30 "$BIN" doctor >> "$LOG" 2>&1
RC=$?
echo "--- exit(rc of doctor; doctor returns 0 even with failures): $RC" >> "$LOG"
echo "--- decision:" >> "$LOG"
grep -E '/dev/kvm not found|Work directory' "$LOG" >> "$LOG" || true
tail -25 "$LOG"
grep -q '/dev/kvm not found' "$LOG" && exit 1
exit 0
