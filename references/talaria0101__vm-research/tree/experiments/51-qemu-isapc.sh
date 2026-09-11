#!/usr/bin/env bash
# Question: does QEMU's minimal "-M isapc" (ISA-only PC, no PCI) boot the
# pinned kernel under TCG — the most stripped vanilla machine type available?
# Exit codes: 0 proof markers, 1 ran but failed, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
vfetch vmlinuz-virt "$PIN_VMLINUZ_VIRT_URL" "$PIN_VMLINUZ_VIRT_SHA" || exit 2
INITRD="$VR_WORK/initramfs-bb.cpio.gz"
[ -f "$INITRD" ] || { echo "run 20- first"; exit 2; }

QEMU_LOG="$VR_LOGS/51-qemu-isapc-boot.log"
{ echo "## conditions"; conditions "qemu-tcg-isapc"; } > "$QEMU_LOG"
timeout 240 qemu-system-x86_64 \
  -accel tcg,thread=multi \
  -M isapc -cpu qemu64 -m 256M -smp 1 \
  -kernel "$VR_WORK/vmlinuz-virt" \
  -initrd "$INITRD" \
  -append "console=ttyS0 rdinit=/init quiet" \
  -nographic -no-reboot -monitor none \
  >> "$QEMU_LOG" 2>&1
QRC=$?
echo "### qemu exit: $QRC" >> "$QEMU_LOG"
echo "### markers: BOOT=$(grep -c 'VMR-BOOT-OK' "$QEMU_LOG") PROOF=$(grep -c 'VMR-PROOF-END' "$QEMU_LOG")" >> "$QEMU_LOG"
tail -8 "$QEMU_LOG"
grep -q 'VMR-PROOF-END' "$QEMU_LOG" && exit 0 || exit 1
