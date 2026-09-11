#!/usr/bin/env bash
# Question: what does a TCG microVM actually cost and deliver inside this
# sandbox? Numbers: (a) cold-boot latency, n=3; (b) in-guest CPU proxy —
# busybox md5sum over 16 MiB — vs the same binary on the host, same conditions.
# These are TCG numbers by construction (KVM unreachable, see 60-); they size
# what a fleet of microVMs can do here, they do not claim KVM-class speed.
# Exit codes: 0 all measurements taken, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
vfetch vmlinuz-virt "$PIN_VMLINUZ_VIRT_URL" "$PIN_VMLINUZ_VIRT_SHA" || exit 2

INITRD="$VR_WORK/initramfs-bench.cpio.gz"
if [ ! -f "$INITRD" ]; then
  ROOT="$VR_WORK/initrd-bench"; rm -rf "$ROOT"; mkdir -p "$ROOT"/{bin,dev,proc,sys,tmp}
  tar -xzf "$VR_WORK/busybox-static.apk" -C "$VR_WORK" bin/busybox.static 2>/dev/null
  cp "$VR_WORK/bin/busybox.static" "$ROOT/bin/busybox"; chmod 755 "$ROOT/bin/busybox"
  cat > "$ROOT/init" <<'EOF'
#!/bin/busybox sh
/bin/busybox --install -s /bin
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev 2>/dev/null
echo "VMR-BOOT-OK"
dd if=/dev/zero of=/tmp/bench bs=1M count=16 2>/dev/null
T0=$(cut -d' ' -f1 /proc/uptime)
MD=$(md5sum /tmp/bench | cut -d' ' -f1)
T1=$(cut -d' ' -f1 /proc/uptime)
echo "VMR-MD5-16M-SECS: $(echo "$T1 $T0" | awk '{printf "%.2f", $1-$2}')"
echo "VMR-MD5-DIGEST: $MD"
echo "VMR-PROOF-END"
poweroff -f
EOF
  chmod 755 "$ROOT/init"
  (cd "$ROOT" && find . | cpio -o -H newc --quiet | gzip -1) > "$INITRD"
fi

LOG="$VR_LOGS/66-tcg-benchmarks.log"
{ echo "## conditions"; conditions tcg-bench; } > "$LOG"

# (a) cold-boot latency, n=3
for i in 1 2 3; do
  T0=$(date +%s.%N)
  timeout 120 qemu-system-x86_64 -accel tcg,thread=multi -M microvm -m 256M -smp 1 \
    -kernel "$VR_WORK/vmlinuz-virt" -initrd "$INITRD" \
    -append "console=ttyS0 rdinit=/init quiet" \
    -display none -no-reboot -monitor none -serial stdio > "$VR_WORK/bench-$i.log" 2>&1
  T1=$(date +%s.%N)
  echo "boot-$i wall: $(echo "$T1 - $T0" | bc) s" >> "$LOG"
done

# host-side control: same md5 on the same file bytes
dd if=/dev/zero of="$VR_WORK/bench16" bs=1M count=16 2>/dev/null
T0=$(date +%s.%N); MDH=$(md5sum "$VR_WORK/bench16" | cut -d' ' -f1); T1=$(date +%s.%N)
echo "host md5sum-16M wall: $(echo "$T1 - $T0" | bc) s digest=$MDH" >> "$LOG"

grep -h 'VMR-MD5' "$VR_WORK"/bench-*.log >> "$LOG"
grep -h 'boot-' "$LOG" | tail -3
tail -6 "$LOG"
grep -aq 'VMR-MD5-16M-SECS' "$LOG" && exit 0 || exit 1
