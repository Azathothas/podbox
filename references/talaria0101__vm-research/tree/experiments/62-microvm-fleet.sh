#!/usr/bin/env bash
# Question: microVMs are for FLEETS. Can this sandbox run N concurrent TCG
# microVMs, and what is the aggregate boot throughput? N=8 -M microvm guests
# (1 vcpu, 128M each, pinned Alpine kernel + busybox initramfs), staggered
# start, wall time to all-8 proof markers = the number.
# Exit codes: 0 all 8 guests printed proof, 1 partial, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
vfetch vmlinuz-virt "$PIN_VMLINUZ_VIRT_URL" "$PIN_VMLINUZ_VIRT_SHA" || exit 2
INITRD="$VR_WORK/initramfs-bb.cpio.gz"
[ -f "$INITRD" ] || { echo "run 20- first"; exit 2; }

LOG="$VR_LOGS/62-microvm-fleet.log"
{ echo "## conditions"; conditions microvm-fleet; echo "fleet: 8 guests x (1 vcpu, 128M, -M microvm, tcg)"; } > "$LOG"

START=$(date +%s.%N)
PIDS=()
for i in $(seq 0 7); do
  ( timeout 240 qemu-system-x86_64 -accel tcg,thread=multi \
      -M microvm -m 128M -smp 1 \
      -kernel "$VR_WORK/vmlinuz-virt" -initrd "$INITRD" \
      -append "console=ttyS0 rdinit=/init quiet" \
      -nographic -no-reboot -monitor none > "$VR_WORK/fleet-$i.log" 2>&1 ) &
  PIDS+=($!)
done
FAIL=0
for i in $(seq 0 7); do
  wait ${PIDS[$i]} || FAIL=$((FAIL+1))
  grep -q 'VMR-PROOF-END' "$VR_WORK/fleet-$i.log" || FAIL=$((FAIL+1))
done
END=$(date +%s.%N)
SECS=$(echo "$END - $START" | bc)
OK=$(grep -l 'VMR-PROOF-END' "$VR_WORK"/fleet-*.log 2>/dev/null | wc -l)
{
  echo "### guests booted with proof: $OK/8"
  echo "### wall seconds for the whole fleet: $SECS"
  echo "### aggregate: $(echo "scale=2; 8 / $SECS" | bc) boots/s"
} >> "$LOG"
grep -h 'VMR-GUEST-UPTIME' "$VR_WORK"/fleet-*.log >> "$LOG"
tail -5 "$LOG"
[ "$OK" -eq 8 ] && exit 0 || exit 1
