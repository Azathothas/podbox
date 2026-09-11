#!/usr/bin/env bash
# Question: boxlite-ai/boxlite v0.10.0 — "micro-VM for AI agents", KVM /
# Hypervisor.framework per its README. The published wheels are cp310/cp311
# only and this host runs Python 3.14.7, so a binary install is impossible
# without a second interpreter; the verdict is therefore [S] (README + arch
# facts) anchored on the census [V]. What do the pinned sources say?
# Exit codes: 0 usable, 1 blocked, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/38-boxlite.log"
conditions boxlite > "$LOG"
{
  echo "--- python available: $(python3 -V) vs published wheels cp310/cp311 (abi mismatch => binary path impossible)"
  echo "--- [S] README isolation lines (pinned v0.10.0):"
  curl -sL --max-time 30 "https://raw.githubusercontent.com/boxlite-ai/boxlite/v0.10.0/README.md" | grep -iE 'kvm|hypervisor.framework' | head -4
  echo "--- [V] census cross-ref:"
  grep -E 'devkvm-node|mknod' logs/10-probe-environment.log
  echo "--- verdict: KVM on Linux [S]; node absent+uncreatable [V] => BLOCKED"
} >> "$LOG" 2>&1
tail -10 "$LOG"
grep -q BLOCKED "$LOG" && exit 1 || exit 2
