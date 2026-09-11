#!/usr/bin/env bash
# 50-tcg-boot.sh
#
# Question: with KVM unreachable, does software emulation deliver a
# booting machine — and how fast, to what, with which quirks?
#
# Proven here:
#   - a pinned Alpine linux-virt kernel boots to userspace under TCG
#   - the initramfs is assembled WITHOUT host mknod: the /dev/console
#     node is written byte-wise into the newc cpio and the kernel's own
#     unpacker creates it guest-side (this host denies mknod)
#   - completion is detected by MARKER on the serial console, then the
#     driver kills qemu: on -M pc,acpi=off poweroff HALTS instead of
#     exiting qemu — the driver must own the lifecycle
#
# Inputs: pinned kernel + busybox-static. Tools: qemu-system-x86_64, python3, gzip.
# Exit: 0 marker seen · 1 boot failed · 2 could not run.
set -u
cd "$(dirname "$0")" || exit 2
. ../scripts/lib.sh

OUT="$SI_LOGS/50-tcg-boot.log"
{
  conditions tcg-boot
  echo "qemu: $(qemu-system-x86_64 --version | head -1)"
  echo
} > "$OUT"

vfetch vmlinuz-virt "$PIN_VMLINUZ_VIRT_URL" "$PIN_VMLINUZ_VIRT_SHA" >>"$OUT" 2>&1 || { echo "pin fetch failed" >>"$OUT"; exit 2; }
vfetch busybox.apk "$PIN_BUSYBOX_URL" "$PIN_BUSYBOX_SHA" >>"$OUT" 2>&1 || { echo "pin fetch failed" >>"$OUT"; exit 2; }

# busybox.static out of the apk (a plain tar.gz with a signature prefix)
rm -rf "$SI_WORK/bb"; mkdir -p "$SI_WORK/bb"
tar -xzf "$SI_WORK/busybox.apk" -C "$SI_WORK/bb" bin/busybox.static 2>>"$OUT" || { echo "busybox extract failed" >>"$OUT"; exit 2; }

# initramfs: dirs + /dev/console (5:1) + busybox + /init — no host mknod anywhere
INIT="$SI_WORK/init.sh"
cat > "$INIT" <<'EOF'
#!/bin/busybox sh
/bin/busybox --install -s /bin
mount -t proc proc /proc 2>/dev/null
echo "VMR-BOOT-OK kernel=$(uname -r) pid=$$"
poweroff -f
EOF
( cd "$SI_WORK" && ln -sf bb/bin/busybox.static bin-busybox ) 2>/dev/null
mkdir -p "$SI_WORK/bin"
cp "$SI_WORK/bb/bin/busybox.static" "$SI_WORK/bin/busybox.static"
python3 "$SI_SCRIPTS/mkrootfs.py" "$SI_WORK/boot.cpio" \
  --dir dev --dir bin --dir proc --dir sys --dir tmp \
  --char dev/console:5:1 \
  --file "$SI_WORK/bin/busybox.static:bin/busybox" \
  --file "$INIT:init" > /dev/null 2>>"$OUT" || { echo "mkrootfs failed" >>"$OUT"; exit 2; }
gzip -1 -kc "$SI_WORK/boot.cpio" > "$SI_WORK/boot.cpio.gz" 2>>"$OUT"

# the driver owns the lifecycle: boot in the background, poll the serial
# log for the completion marker, then kill qemu — on -M pc,acpi=off the
# guest's poweroff HALTS the vCPU instead of exiting qemu, so waiting for
# qemu to exit means waiting for the timeout.
rm -f "$SI_WORK/50-serial.log"
T0=$(date +%s.%N)
qemu-system-x86_64 -accel tcg,thread=multi \
  -M pc,acpi=off -m 256M -smp 1 \
  -kernel "$SI_WORK/vmlinuz-virt" -initrd "$SI_WORK/boot.cpio.gz" \
  -append "console=ttyS0 rdinit=/init panic=-1 quiet" \
  -display none -no-reboot -monitor none \
  -serial file:"$SI_WORK/50-serial.log" >/dev/null 2>>"$OUT" &
QPID=$!
MARKER_SEEN=""
for _ in $(seq 1 300); do
  if grep -q "VMR-BOOT-OK" "$SI_WORK/50-serial.log" 2>/dev/null; then
    T1=$(date +%s.%N); MARKER_SEEN=yes
    break
  fi
  sleep 0.1
done
kill "$QPID" 2>/dev/null; wait "$QPID" 2>/dev/null
T1=${T1:-$(date +%s.%N)}

{
  if [ -n "$MARKER_SEEN" ]; then
    echo "time-to-marker: $(python3 -c "print(f'{$T1-$T0:.2f}')")s (guest userspace reached; qemu killed by the driver — acpi=off halts, never exits)"
  else
    echo "time-to-marker: NEVER (30s cap)"
  fi
  echo "## serial console tail:"
  grep -E "VMR-" "$SI_WORK/50-serial.log" || echo "(no marker — boot failed)"
  echo
  grep -E "Kernel panic|BUG" "$SI_WORK/50-serial.log" | head -3 || true
} >> "$OUT"

if grep -q "VMR-BOOT-OK" "$OUT"; then
  tail -4 "$OUT"
  echo
  echo "log: $OUT"
  exit 0
fi
tail -6 "$OUT"
exit 1
