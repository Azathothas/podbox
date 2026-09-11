#!/usr/bin/env bash
# Question: does QEMU's dedicated "microvm" machine type (minimal device
# model, virtio-mmio, the vanilla microVM the task asks for) boot the same
# pinned kernel under TCG, and is it faster than the pc machine type (20-)?
#
# Exit codes: 0 boot + proof markers, 1 ran but failed, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"

vfetch vmlinuz-virt "$PIN_VMLINUZ_VIRT_URL" "$PIN_VMLINUZ_VIRT_SHA" || exit 2
# reuse the deterministic initramfs built by 20- if present, else build it
INITRD="$VR_WORK/initramfs-bb.cpio.gz"
if [ ! -f "$INITRD" ]; then echo "run 20- first (builds initramfs)"; exit 2; fi

QEMU_LOG="$VR_LOGS/21-qemu-tcg-microvm-boot.log"
{
  echo "## conditions"; conditions "qemu-tcg-microvm"
  echo "kernel: $PIN_VMLINUZ_VIRT_URL (sha256 $PIN_VMLINUZ_VIRT_SHA)"
  echo "machine: microvm (virtio-mmio, no PCI/ISA legacy beyond serial)"
  echo "control: experiments/logs/20-qemu-tcg-pc-boot.log"
} > "$QEMU_LOG"
START=$(date +%s.%N)
timeout 240 qemu-system-x86_64 \
  -accel tcg,thread=multi \
  -M microvm -m 512M -smp 2 \
  -kernel "$VR_WORK/vmlinuz-virt" \
  -initrd "$INITRD" \
  -append "console=ttyS0 rdinit=/init quiet" \
  -nographic -no-reboot -monitor none \
  >> "$QEMU_LOG" 2>&1
QRC=$?
END=$(date +%s.%N)
SECS=$(echo "$END - $START" | bc)
{
  echo "### qemu exit: $QRC"
  echo "### wall seconds to qemu exit: $SECS"
  echo "### markers: BOOT=$(grep -c 'VMR-BOOT-OK' "$QEMU_LOG") PROOF=$(grep -c 'VMR-PROOF-END' "$QEMU_LOG")"
} >> "$QEMU_LOG"
tail -14 "$QEMU_LOG"
[ "$(grep -c 'VMR-PROOF-END' "$QEMU_LOG")" -ge 1 ] && exit 0 || exit 1
