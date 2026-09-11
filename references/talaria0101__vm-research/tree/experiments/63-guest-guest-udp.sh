#!/usr/bin/env bash
# Question: with TCP bind denied (11-), can two microVMs still TALK? slirp's
# UDP hostfwd only needs a UDP bind — which the sandbox allows. Guest A sends
# UDP to its slirp gateway (10.0.2.2:9002); that lands on the host UDP socket
# bound by guest B's slirp hostfwd, which delivers into guest B's nc listener.
# No TCP anywhere. Proof: B's serial shows A's payload.
# Exit codes: 0 payload crossed A->host->B, 1 failed, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
vfetch vmlinuz-virt "$PIN_VMLINUZ_VIRT_URL" "$PIN_VMLINUZ_VIRT_SHA" || exit 2

INITRD="$VR_WORK/initramfs-net.cpio.gz"
if [ ! -f "$INITRD" ]; then
  ROOT="$VR_WORK/initrd-net"; rm -rf "$ROOT"; mkdir -p "$ROOT"/{bin,dev,proc,sys,tmp}
  tar -xzf "$VR_WORK/busybox-static.apk" -C "$VR_WORK" bin/busybox.static 2>/dev/null
  cp "$VR_WORK/bin/busybox.static" "$ROOT/bin/busybox"; chmod 755 "$ROOT/bin/busybox"
  # virtio_net is a module in the alpine virt kernel; ship it + dep from the
  # pinned netboot initramfs's module tree
  mkdir -p "$ROOT/lib/modules"
  cp -r "$VR_WORK/vmod/lib/modules/6.12.94-0-virt" "$ROOT/lib/modules/"
  cat > "$ROOT/init" <<'EOF'
#!/bin/busybox sh
/bin/busybox --install -s /bin
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev 2>/dev/null
echo "VMR-BOOT-OK"
modprobe virtio_net 2>&1
ifconfig eth0 10.0.2.15 netmask 255.255.255.0 up
route add default gw 10.0.2.2
ROLE=$(cat /proc/cmdline | tr ' ' '\n' | grep '^role=' | cut -d= -f2)
echo "VMR-ROLE-$ROLE"
if [ "$ROLE" = "b" ]; then
  echo "VMR-B-LISTENING"
  nc -lu -p 7000
  poweroff -f
else
  i=0
  while [ $i -lt 20 ]; do
    i=$((i+1))
    echo "PING-from-A-seq$i" | nc -u -w1 10.0.2.2 9002
    sleep 2
  done
  echo "VMR-A-DONE"
  poweroff -f
EOF
  echo "fi" >> "$ROOT/init"
  chmod 755 "$ROOT/init"
  (cd "$ROOT" && find . | cpio -o -H newc --quiet | gzip -1) > "$INITRD"
fi

LOG="$VR_LOGS/63-guest-guest-udp.log"
{ echo "## conditions"; conditions guest-guest-udp; echo "path: A slirp -> host udp 9002 (hostfwd bind) -> B slirp hostfwd -> B nc:7000/udp"; } > "$LOG"

NETDEV_B=(-netdev user,id=n0,hostfwd=udp:127.0.0.1:9002-:7000 -device virtio-net-device,netdev=n0)
NETDEV_A=(-netdev user,id=n0 -device virtio-net-device,netdev=n0)

( timeout 90 qemu-system-x86_64 -accel tcg,thread=multi -M microvm -m 128M -smp 1 \
    -kernel "$VR_WORK/vmlinuz-virt" -initrd "$INITRD" \
    -append "console=ttyS0 rdinit=/init quiet role=b" \
    -display none -no-reboot -monitor none -serial stdio "${NETDEV_B[@]}" ) > "$VR_WORK/guest-b.log" 2>&1 &
BPID=$!
sleep 2
( timeout 90 qemu-system-x86_64 -accel tcg,thread=multi -M microvm -m 128M -smp 1 \
    -kernel "$VR_WORK/vmlinuz-virt" -initrd "$INITRD" \
    -append "console=ttyS0 rdinit=/init quiet role=a" \
    -display none -no-reboot -monitor none -serial stdio "${NETDEV_A[@]}" ) > "$VR_WORK/guest-a.log" 2>&1 &
APID=$!
wait $BPID; wait $APID
HITS=$(grep -c 'PING-from-A' "$VR_WORK/guest-b.log")
{
  echo "--- PING payloads seen by guest B: $HITS"
  grep -a 'PING-from-A' "$VR_WORK/guest-b.log" | head -3
  echo "--- A state: $(grep -ac 'VMR-A-DONE' "$VR_WORK/guest-a.log") done-marker"
} >> "$LOG"
tail -4 "$LOG"
[ "$HITS" -gt 0 ] && exit 0 || exit 1
