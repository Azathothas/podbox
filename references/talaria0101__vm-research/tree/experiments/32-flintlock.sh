#!/usr/bin/env bash
# Question: liquidmetal-dev/flintlock v0.15.1 — microVM manager whose
# backends are firecracker and (optionally) cloud-hypervisor; both were proven
# /dev/kvm-dependent (30- here; upstream source). Does the daemon even start
# here, and what does a CreateMicroVM attempt return?
# Exit codes: 0 usable, 1 blocked, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/32-flintlock.log"
conditions flintlock > "$LOG"
{
  echo "--- deb download (flintlockd only, no containerd here — the dependency chain itself is evidence)"
  curl -sL --max-time 300 -o "$VR_WORK/flintlockd.deb" \
    "https://github.com/liquidmetal-dev/flintlock/releases/download/v0.15.1/flintlockd_0.15.1_linux_amd64.deb" || echo "download failed"
  dpkg-deb -x "$VR_WORK/flintlockd.deb" "$VR_WORK/flintlock" 2>/dev/null || ar p "$VR_WORK/flintlockd.deb" data.tar.gz 2>/dev/null | tar -xz -C "$VR_WORK/flintlock" 2>/dev/null
  FB=$(find "$VR_WORK/flintlock" -name 'flintlockd' -type f | head -1)
  echo "binary: $FB" >> /dev/null
  [ -n "$FB" ] && timeout 15 "$FB" --version 2>&1 | head -2
  echo "--- [S] backend requires (source, pinned v0.15.1):"
  curl -sL --max-time 30 "https://raw.githubusercontent.com/liquidmetal-dev/flintlock/v0.15.1/pkg/microvm/firecracker/create.go" | grep -nE 'kvm|Kvm|jailer' | head -4
  echo "--- [V] census cross-ref (firecracker blocked first-hand: see 30-):"
  grep -E 'fault_message|Error creating KVM' logs/30-firecracker.log | head -2
} >> "$LOG" 2>&1
tail -15 "$LOG"
exit 1
