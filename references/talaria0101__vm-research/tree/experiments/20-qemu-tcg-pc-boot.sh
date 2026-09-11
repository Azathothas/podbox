#!/usr/bin/env bash
# Question: does software emulation (TCG) boot a real Linux kernel to userspace
# and a controlled poweroff in this sandbox — i.e. is "microVM under TCG" a
# viable substitute for the denied KVM path at all?
#
# Artefacts: pinned Alpine 3.22 netboot kernel 6.12.94-0-virt + initramfs
# (see lib.sh); initramfs is Alpine's own netboot image, booted offline into a
# busybox shell via rdinit trick — actually Alpine's initramfs needs its init,
# so this experiment uses a *minimal custom initramfs* built from pinned
# busybox-static instead (see 20-helper below); 25- then boots Alpine's full
# initramfs. Nothing unpinned runs.
#
# Exit codes: 0 boot + proof markers seen, 1 ran but boot failed, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"

vfetch busybox-static.apk "$PIN_BUSYBOX_URL" "$PIN_BUSYBOX_SHA" || exit 2
vfetch vmlinuz-virt "$PIN_VMLINUZ_VIRT_URL" "$PIN_VMLINUZ_VIRT_SHA" || exit 2

# --- build deterministic offline initramfs from pinned busybox --------------
INITRD="$VR_WORK/initramfs-bb.cpio.gz"
if [ ! -f "$INITRD" ]; then
  ROOT="$VR_WORK/initrd-root"; rm -rf "$ROOT"; mkdir -p "$ROOT"/{bin,dev,proc,sys,tmp}
  # busybox-static apk is a gzipped tarball; extract the static binary
  tar -xzf "$VR_WORK/busybox-static.apk" -C "$VR_WORK" bin/busybox.static 2>/dev/null \
    || tar -xzf "$VR_WORK/busybox-static.apk" -C "$VR_WORK" ./bin/busybox.static
  cp "$VR_WORK/bin/busybox.static" "$ROOT/bin/busybox"; chmod 755 "$ROOT/bin/busybox"
  cat > "$ROOT/init" <<'EOF'
#!/bin/busybox sh
/bin/busybox --install -s /bin
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev 2>/dev/null
echo "VMR-BOOT-OK kernel=$(uname -r)"
echo "VMR-GUEST-UNAME: $(uname -a)"
echo "VMR-GUEST-CPU: $(grep -m1 'model name' /proc/cpuinfo)"
echo "VMR-GUEST-HYPERVISOR-FLAG: $(grep -c hypervisor /proc/cpuinfo)"
echo "VMR-GUEST-MEM: $(grep MemTotal /proc/meminfo)"
echo "VMR-GUEST-UPTIME: $(cat /proc/uptime)"
echo "VMR-PROOF-END"
poweroff -f
EOF
  chmod 755 "$ROOT/init"
  (cd "$ROOT" && find . | cpio -o -H newc --quiet | gzip -1) > "$INITRD"
fi
echo "[20] initramfs: $(stat -c%s "$INITRD") bytes"

# --- the boot: TCG, vanilla machine type, serial console --------------------
QEMU_LOG="$VR_LOGS/20-qemu-tcg-pc-boot.log"
{
  echo "## conditions"; conditions "qemu-tcg-pc"
  echo "kernel: $PIN_VMLINUZ_VIRT_URL (sha256 $PIN_VMLINUZ_VIRT_SHA)"
  echo "initramfs: custom busybox-static (sha256-of-apk $PIN_BUSYBOX_SHA)"
  echo "measured: wall seconds to end marker from guest serial"
} > "$QEMU_LOG"
START=$(date +%s.%N)
timeout 240 qemu-system-x86_64 \
  -accel tcg,thread=multi \
  -M pc -m 512M -smp 2 \
  -kernel "$VR_WORK/vmlinuz-virt" \
  -initrd "$INITRD" \
  -append "console=ttyS0 rdinit=/init quiet" \
  -nographic -no-reboot -monitor none \
  >> "$QEMU_LOG" 2>&1
QRC=$?
END=$(date +%s.%N)
SECS=$(echo "$END - $START" | bc)
{
  echo "### qemu exit: $QRC (0 under -no-reboot = guest powered off)"
  echo "### wall seconds to qemu exit: $SECS"
  echo "### markers: BOOT=$(grep -c 'VMR-BOOT-OK' "$QEMU_LOG") PROOF=$(grep -c 'VMR-PROOF-END' "$QEMU_LOG")"
} >> "$QEMU_LOG"
tail -14 "$QEMU_LOG"
[ "$(grep -c 'VMR-PROOF-END' "$QEMU_LOG")" -ge 1 ] && exit 0 || exit 1
