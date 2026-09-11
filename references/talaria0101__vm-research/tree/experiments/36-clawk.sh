#!/usr/bin/env bash
# Question: clawkwork/clawk v0.4.0 — on Linux it boots a disposable VM
# (firecracker per README); what is the first failure here?
# Exit codes: 0 usable, 1 blocked, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/36-clawk.log"
conditions clawk > "$LOG"
BIN="$VR_WORK/bins/clawk/clawk"
[ -x "$BIN" ] || { echo "binary missing" >> "$LOG"; exit 2; }
timeout 15 "$BIN" --version >> "$LOG" 2>&1
mkdir -p /tmp/clawk-cwd && cd /tmp/clawk-cwd
echo "--- vm boot attempt (cwd mode, no runner attached)" >> "$LOG"
timeout 60 "$BIN" up --help >> "$LOG" 2>&1
CLAWK_HOME=/tmp/clawk-home timeout 90 "$BIN" up --provider firecracker >> "$LOG" 2>&1
RC=$?
echo "--- exit: $RC" >> "$LOG"
tail -20 "$LOG"
exit 1
