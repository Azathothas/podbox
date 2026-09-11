#!/usr/bin/env bash
# Question: deeplethe/forkd v0.5.3 (Firecracker-fork based) — its `quickstart`
# preflights the host; which check fails first here?
# Exit codes: 0 usable, 1 blocked, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/35-forkd.log"
conditions forkd > "$LOG"
BIN="$VR_WORK/bins/forkd/forkd"
[ -x "$BIN" ] || { echo "binary missing" >> "$LOG"; exit 2; }
timeout 60 "$BIN" quickstart --help >> "$LOG" 2>&1
echo "--- quickstart (preflight + first fork attempt)" >> "$LOG"
timeout 90 "$BIN" quickstart -y >> "$LOG" 2>&1
RC=$?
echo "--- exit: $RC" >> "$LOG"
tail -20 "$LOG"
exit 1
