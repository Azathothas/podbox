#!/bin/sh
# Scratch spike for T-1303: can this lane install qemu, fetch the pinned
# Alpine virt kernel, and boot a trivial initramfs under TCG to a marker?
# Answers whether experiments/146 has a testbed before it is written.
set -u
export DEBIAN_FRONTEND=noninteractive
echo "== install"
apt-get update -qq || exit 2
apt-get install -y -qq qemu-system-x86 busybox-static cpio gzip curl || exit 2
qemu-system-x86_64 --version || exit 2
echo "== fetch pinned kernel"
KURL="https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/netboot/vmlinuz-virt"
KSHA="12eb24189f3eb30bd0dcd919248caaa054ed4e87b799a53fdcc3999f157933e4"
curl -fsSL -o /tmp/vmlinuz-virt "$KURL" || exit 2
printf '%s  /tmp/vmlinuz-virt\n' "$KSHA" | sha256sum -c - || exit 2
ls -la /tmp/vmlinuz-virt
echo "== trivial initramfs"
W=/tmp/spike-root
rm -rf "$W"
mkdir -p "$W/bin" || exit 2
cp /bin/busybox "$W/bin/busybox" || exit 2
cat >"$W/init" <<'INITEOF'
#!/bin/busybox sh
mount -t proc proc /proc 2>/dev/null
mount -t sysfs sysfs /sys 2>/dev/null
echo SPIKE-READY
poweroff -f 2>/dev/null || reboot -f 2>/dev/null || while true; do sleep 60; done
INITEOF
chmod +x "$W/init" || exit 2
(cd "$W" && find . -print0 | cpio --quiet -o -H newc --null | gzip -9 > /tmp/spike.cpio.gz) || exit 2
ls -la /tmp/spike.cpio.gz
echo "== boot under TCG"
timeout 120 qemu-system-x86_64 \
  -M pc,acpi=off -m 256 -nographic -no-reboot \
  -accel tcg,thread=multi \
  -kernel /tmp/vmlinuz-virt \
  -initrd /tmp/spike.cpio.gz \
  -append "console=ttyS0 panic=-1" \
  >/tmp/spike-boot.log 2>&1
echo "qemu rc=$?"
grep -a "SPIKE-READY" /tmp/spike-boot.log && echo "MARKER: yes" || echo "MARKER: no"
tail -5 /tmp/spike-boot.log
