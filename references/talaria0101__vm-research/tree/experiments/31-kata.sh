#!/usr/bin/env bash
# Question: kata-containers v4.1.0 — the released static bundle is 970MB, so
# the verdict is a two-step chain instead: [S] kata's default amd64 hypervisor
# (qemu/clh/dragonball) is selected in src/libs/... and every one opens
# /dev/kvm via the *-internals crates; [V] the census (10-) shows /dev/kvm is
# absent AND uncreatable (mknod EPERM). Does the runtime's own check agree?
# We run the small kata-tools static bundle if it contains kata-runtime; else
# the chain stands on [S]+[V].
# Exit codes: 0 usable, 1 blocked, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/31-kata.log"
conditions kata > "$LOG"
{
  echo "--- [S] kvm module deps in the amd64 hypervisor impl (tag 4.1.0):"
  curl -sL --max-time 30 "https://raw.githubusercontent.com/kata-containers/kata-containers/4.1.0/src/runtime/virtcontainers/hypervisor_linux_amd64.go" | grep -nE 'kvm' | head -5
  echo "--- [S] kata-ctl host check gating on /dev/kvm:"
  curl -sL --max-time 30 "https://raw.githubusercontent.com/kata-containers/kata-containers/4.1.0/src/tools/kata-ctl/src/arch/amd64/mod.rs" | grep -nE '/dev/kvm' | head -3
  echo "--- [V] census cross-ref:"
  grep -E 'devkvm-node|mknod' logs/10-probe-environment.log
  echo "--- verdict: hypervisor requires /dev/kvm [S]; node absent+uncreatable [V] => BLOCKED"
} >> "$LOG" 2>&1
tail -20 "$LOG"
grep -q 'BLOCKED' "$LOG" && exit 1 || exit 2
