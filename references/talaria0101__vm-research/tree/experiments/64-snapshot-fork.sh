#!/usr/bin/env bash
# Question: forkd's pitch is "BRANCH a live VM in ~150ms". The KVM wall blocks
# forkd (35-) — but qemu migration gives the same primitive without KVM: boot
# once, snapshot to a file (migrate exec:), then resume N live clones from
# that file (-incoming). Do clones resume, run, and power off here — and how
# fast is a resume vs a cold boot?
# Exit codes: 0 clones resumed and ran, 1 partial/failed, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
vfetch vmlinuz-virt "$PIN_VMLINUZ_VIRT_URL" "$PIN_VMLINUZ_VIRT_SHA" || exit 2

# guest init: proof at boot, then RESUMED-x ticks, poweroff after 3rd tick
INITRD="$VR_WORK/initramfs-mig.cpio.gz"
if [ ! -f "$INITRD" ]; then
  ROOT="$VR_WORK/initrd-mig"; rm -rf "$ROOT"; mkdir -p "$ROOT"/{bin,dev,proc,sys,tmp}
  tar -xzf "$VR_WORK/busybox-static.apk" -C "$VR_WORK" bin/busybox.static 2>/dev/null
  cp "$VR_WORK/bin/busybox.static" "$ROOT/bin/busybox"; chmod 755 "$ROOT/bin/busybox"
  cat > "$ROOT/init" <<'EOF'
#!/bin/busybox sh
/bin/busybox --install -s /bin
mount -t proc proc /proc 2>/dev/null
mount -t sysfs sysfs /sys 2>/dev/null
mount -t devtmpfs devtmpfs /dev 2>/dev/null
echo "VMR-BOOT-OK"
echo "VMR-PROOF-END"
i=0
while [ $i -lt 12 ]; do
  i=$((i+1))
  sleep 2
  echo "VMR-RESUMED-TICK-$i"
done
poweroff -f
EOF
  chmod 755 "$ROOT/init"
  (cd "$ROOT" && find . | cpio -o -H newc --quiet | gzip -1) > "$INITRD"
fi

LOG="$VR_LOGS/64-snapshot-fork.log"
{ echo "## conditions"; conditions snapshot-fork; echo "parent: 1 vcpu, 128M, -M microvm tcg; snapshot via migrate exec:cat"; } > "$LOG"
MON="$VR_WORK/mon-64.sock"
SNAP="$VR_WORK/vmr-snap.bin"
rm -f "$MON" "$SNAP"

# 1. boot parent with monitor on unix socket
qemu-system-x86_64 -accel tcg,thread=multi -M microvm -m 128M -smp 1 \
  -kernel "$VR_WORK/vmlinuz-virt" -initrd "$INITRD" \
  -append "console=ttyS0 rdinit=/init quiet" \
  -display none -no-reboot -monitor none \
  -chardev socket,id=mon,path="$MON",server=on,wait=off -mon chardev=mon,mode=readline \
  > "$VR_WORK/parent-64.log" 2>&1 &
QPID=$!
# wait for the parent's proof marker in its serial log? serial not captured (none);
# instead: wait for monitor socket, then extra fixed delay for userspace
for i in $(seq 1 100); do [ -S "$MON" ] && break; sleep 0.2; done
sleep 12   # TCG boot to proof observed ~5-8s; 12s is safely inside tick window

python3 - "$MON" "$SNAP" <<'EOF' >> "$LOG" 2>&1
import socket, sys, time
mon, snap = sys.argv[1], sys.argv[2]
s = socket.socket(socket.AF_UNIX); s.connect(mon); s.settimeout(1.0)
def cmd(c, wait=0.5):
    s.sendall((c + "\n").encode()); time.sleep(wait)
    try: return s.recv(65536).decode(errors="replace")
    except socket.timeout: return ""
cmd("")  # banner
t0 = time.time()
r = cmd(f'migrate -d "exec:cat > {snap}"', 0.5)
for _ in range(100):
    st = cmd("info migrate", 0.3)
    if "completed" in st: break
    if "failed" in st: print("MIGRATE FAILED:", st); break
    time.sleep(0.3)
print(f"snapshot: {time.time()-t0:.1f}s, file size pending")
EOF
ls -la "$SNAP" >> "$LOG" 2>&1
kill $QPID 2>/dev/null; wait $QPID 2>/dev/null
SNAPSIZE=$(stat -c%s "$SNAP" 2>/dev/null || echo 0)
[ "$SNAPSIZE" -gt 1000000 ] || { echo "snapshot missing/too small" >> "$LOG"; tail -5 "$LOG"; exit 1; }

# 2. resume 3 live clones from the file, measure each to first RESUMED tick
echo "--- resuming 3 clones" >> "$LOG"
for c in 1 2 3; do
  T0=$(date +%s.%N)
  timeout 90 qemu-system-x86_64 -accel tcg,thread=multi -M microvm -m 128M -smp 1 \
    -kernel "$VR_WORK/vmlinuz-virt" -initrd "$INITRD" \
    -append "console=ttyS0 rdinit=/init quiet" \
    -display none -no-reboot -monitor none -serial stdio \
    -incoming "exec:cat $SNAP" > "$VR_WORK/clone-$c.log" 2>&1
  T1=$(date +%s.%N)
  D=$(echo "$T1 - $T0" | bc)
  GOT=$(grep -c 'VMR-RESUMED-TICK' "$VR_WORK/clone-$c.log")
  echo "clone-$c: wall=${D}s resumed_ticks=$GOT" >> "$LOG"
done
OK=$(grep -l 'VMR-RESUMED-TICK' "$VR_WORK"/clone-*.log 2>/dev/null | wc -l)
echo "--- clones resumed and ran: $OK/3" >> "$LOG"
tail -6 "$LOG"
[ "$OK" -eq 3 ] && exit 0 || exit 1
