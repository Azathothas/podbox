#!/usr/bin/env bash
# Boot the nix-experiment VM under TCG (no /dev/kvm needed) with user-mode
# networking, run the whole experiment on the serial console.
set -euo pipefail
cd "$(dirname "$0")/.."
MEM="${MEM:-4096}"
SMP="${SMP:-8}"
qemu-system-x86_64 \
  -M pc,accel=tcg -cpu max -m "$MEM" -smp "$SMP" \
  -kernel vm/root/boot/vmlinuz-virt \
  -initrd vm/initramfs.cpio.gz \
  -append "console=ttyS0 rdinit=/init panic=-1 quiet loglevel=3" \
  -device virtio-net-pci,netdev=n0 -netdev user,id=n0 \
  -nographic -no-reboot
