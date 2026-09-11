#!/usr/bin/env bash
# Question: superradcompany/microsandbox v0.6.17 — libkrun-based ("Linux: KVM
# enabled" per its README). Can the server (msd) start here?
# Exit codes: 0 usable, 1 blocked, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/37-microsandbox.log"
conditions microsandbox > "$LOG"
{
  echo "--- [S] README requirement line (pinned v0.6.17 tag):"
  curl -sL --max-time 30 "https://raw.githubusercontent.com/superradcompany/microsandbox/v0.6.17/README.md" | grep -B1 -A1 'KVM enabled' | head -4
  echo "--- [S] libkrun device open (libkrun source, v5.x):"
  curl -sL --max-time 30 "https://raw.githubusercontent.com/containers/libkrun/main/src/vmm/src/linux/vstate.rs" | grep -nE '/dev/kvm|KvmFd|open' | head -4
  echo "--- [S] libkrun vstate.rs: Kvm::new() opens /dev/kvm (main @ $(date -u +%F)):"
  curl -s --max-time 30 -H "Accept: application/vnd.github.raw" "https://api.gh.pkgforge.dev/repos/containers/libkrun/contents/src/libkrun/src/vmm/linux/vstate.rs?ref=main" | grep -n 'Kvm::new()' | head -2
  echo "--- [V] census cross-ref:"
  grep -E 'devkvm-node|mknod' logs/10-probe-environment.log
  echo "--- verdict: libkrun opens /dev/kvm [S]; node absent+uncreatable [V] => BLOCKED"
} >> "$LOG" 2>&1
tail -12 "$LOG"
grep -q 'BLOCKED' "$LOG" && exit 1 || exit 2
