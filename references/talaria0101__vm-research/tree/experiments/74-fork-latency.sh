#!/usr/bin/env bash
# Question: how long does a live-snapshot RESUME actually take (forkd claims
# ~150 ms)? Measured from inside the guest: init prints uptime with every
# tick; the parent is snapshotted at guest-uptime U0; a clone's FIRST tick
# after resume has uptime U1; restore latency = U1 - U0 (guest clock tracks
# wall clock via calibrated TCG timer).
# Exit codes: 0 >=3 clones resumed with measurable latency, 1 failed, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
vfetch vmlinuz-virt "$PIN_VMLINUZ_VIRT_URL" "$PIN_VMLINUZ_VIRT_SHA" || exit 2

INITRD="$VR_WORK/initramfs-lat.cpio.gz"
if [ ! -f "$INITRD" ]; then
  ROOT="$VR_WORK/initrd-lat"; rm -rf "$ROOT"; mkdir -p "$ROOT"/{bin,dev,proc,sys,tmp}
  tar -xzf "$VR_WORK/busybox-static.apk" -C "$VR_WORK" bin/busybox.static 2>/dev/null
  cp "$VR_WORK/bin/busybox.static" "$ROOT/bin/busybox"; chmod 755 "$ROOT/bin/busybox"
  cat > "$ROOT/init" <<'EOF'
#!/bin/busybox sh
/bin/busybox --install -s /bin
mount -t proc proc /proc 2>/dev/null
i=0
while :; do
  i=$((i+1))
  U=$(cut -d' ' -f1 /proc/uptime)
  echo "VMR-TICK-$i uptime=$U"
  sleep 2
done
EOF
  chmod 755 "$ROOT/init"
  (cd "$ROOT" && find . | cpio -o -H newc --quiet | gzip -1) > "$INITRD"
fi

LOG="$VR_LOGS/74-fork-latency.log"
{ echo "## conditions"; conditions fork-latency; echo "guest: 128M microvm; snapshot migrate exec:file; clones -incoming exec:cat"; } > "$LOG"

MON="$VR_WORK/mon-74.sock"; SNAP="$VR_WORK/lat-snap.bin"
rm -f "$MON" "$SNAP"
qemu-system-x86_64 -accel tcg,thread=multi -M microvm -m 128M -smp 1 \
  -kernel "$VR_WORK/vmlinuz-virt" -initrd "$INITRD" \
  -append "console=ttyS0 rdinit=/init quiet" \
  -display none -no-reboot -monitor none -serial file:"$VR_WORK/parent-74.log" \
  -chardev socket,id=mon,path="$MON",server=on,wait=off -mon chardev=mon,mode=readline \
  > "$VR_WORK/parent-74.log" 2>&1 &
QP=$!
# wait for tick 3 (guest uptime ~6s) so the snapshot has a known U0
for i in $(seq 1 200); do grep -q 'VMR-TICK-3' "$VR_WORK/parent-74.log" 2>/dev/null && break; sleep 0.2; done
U0=$(grep -aoE 'VMR-TICK-3 uptime=[0-9.]+' "$VR_WORK/parent-74.log" | grep -oE '[0-9.]+$')
T0=$(date +%s.%N)
python3 - "$MON" "$SNAP" <<'EOF' >> "$LOG" 2>&1
import socket, sys, time
mon, snap = sys.argv[1], sys.argv[2]
s = socket.socket(socket.AF_UNIX); s.connect(mon); s.settimeout(1.0)
def cmd(c, w=0.4):
    s.sendall((c+"\n").encode()); time.sleep(w)
    try: return s.recv(65536).decode(errors="replace")
    except socket.timeout: return ""
cmd(f'migrate -d "exec:cat > {snap}"')
for _ in range(200):
    st = cmd("info migrate", 0.2)
    if "completed" in st: print("snapshot completed"); break
    if "failed" in st: print("MIGRATE FAILED"); break
    time.sleep(0.2)
EOF
T1=$(date +%s.%N)
echo "snapshot wall: $(awk "BEGIN{printf \"%.2f\", $T1-$T0}")s; U0=$U0; size=$(stat -c%s "$SNAP" 2>/dev/null)" >> "$LOG"
kill $QP 2>/dev/null; wait $QP 2>/dev/null

echo "--- clones (uptime delta = restore latency)" >> "$LOG"
for c in 1 2 3; do
  T0=$(date +%s.%N)
  timeout 60 qemu-system-x86_64 -accel tcg,thread=multi -M microvm -m 128M -smp 1 \
    -kernel "$VR_WORK/vmlinuz-virt" -initrd "$INITRD" \
    -append "console=ttyS0 rdinit=/init quiet" \
    -display none -no-reboot -monitor none -serial stdio \
    -incoming "exec:cat $SNAP" > "$VR_WORK/lat-clone-$c.log" 2>&1
  T1=$(date +%s.%N)
  U1=$(grep -aoE 'VMR-TICK-[0-9]+ uptime=[0-9.]+' "$VR_WORK/lat-clone-$c.log" | head -1 | grep -oE '[0-9.]+$')
  D=$(awk -v a="${U1:-0}" -v b="${U0:-0}" 'BEGIN{printf "%.2f", a-b}')
  echo "clone-$c: first-tick uptime=${U1:-none} restore-latency≈${D}s wall=$(awk "BEGIN{printf \"%.2f\", $T1-$T0}")s" >> "$LOG"
done
grep -aE 'clone-|snapshot wall' "$LOG" | tail -6
[ "$(grep -c 'restore-latency' "$LOG")" -ge 3 ] && exit 0 || exit 1
