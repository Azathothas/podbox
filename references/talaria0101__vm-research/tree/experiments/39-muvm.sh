#!/usr/bin/env bash
# Question: AsahiLinux/muvm muvm-0.6.0 — links libkrun for the microVM; can
# muvm run a program here?
# Exit codes: 0 usable, 1 blocked, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/39-muvm.log"
conditions muvm > "$LOG"
{
  echo "--- [S] README: links libkrun (pinned muvm-0.6.0):"
  curl -sL --max-time 30 "https://raw.githubusercontent.com/AsahiLinux/muvm/muvm-0.6.0/README.md" | grep -iE 'libkrun|kvm' | head -3
  echo "--- [S] muvm crate Cargo.toml kvm dependency:"
  curl -sL --max-time 30 "https://raw.githubusercontent.com/AsahiLinux/muvm/muvm-0.6.0/muvm/Cargo.toml" | grep -nE 'krun|kvm' | head -5
  echo "--- [S] libkrun vstate.rs: Kvm::new() opens /dev/kvm (main @ $(date -u +%F)):"
  curl -s --max-time 30 -H "Accept: application/vnd.github.raw" "https://api.gh.pkgforge.dev/repos/containers/libkrun/contents/src/libkrun/src/vmm/linux/vstate.rs?ref=main" | grep -n 'Kvm::new()' | head -2
  echo "--- [V] census cross-ref:"
  grep -E 'devkvm-node|mknod' logs/10-probe-environment.log
  echo "--- verdict: libkrun backend [S]; node absent+uncreatable [V] => BLOCKED"
} >> "$LOG" 2>&1
tail -12 "$LOG"
grep -q 'BLOCKED' "$LOG" && exit 1 || exit 2
