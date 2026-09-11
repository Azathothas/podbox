#!/usr/bin/env bash
# Question: the KVM wall blocked the firecracker VMM (30-) — but do FIRECRACKER'S
# OWN microVM artifacts run in this sandbox? Their official CI kernel
# (vmlinux-6.1.102) + CI initramfs, from the public spec.ccfc.min bucket, on
# qemu. Sub-findings that had to be engineered on the way (all logged):
#   a) firecracker's initramfs ships an EMPTY /dev — PID1 gets no console
#      ("Warning: unable to open an initial console") — fixed by appending a
#      cpio record for dev/console 5:1 (no host mknod needed; archives concat);
#   b) on -M microvm the kernel prints via the 16550 but USERSPACE writes to
#      ttyS0 never surface; on -M pc the same kernel+initramfs is fully
#      interactive (mknod=0 open=3, fd1 writes visible);
#   c) -M microvm also needed acpi=off (ACPI table parse hang) — pc+acpi=off
#      avoids both.
# Verdict shape: their kernel runs OUR static init (reboot(2) proof), their
# initramfs shell comes up interactive, guest executes uname/id + marker.
# Exit codes: 0 marker printed by guest shell, 1 ran but failed, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/65-firecracker-artifacts.log"
conditions firecracker-artifacts > "$LOG"

FCI="$VR_WORK/firecracker-ci"
mkdir -p "$FCI"
URLBASE="https://s3.amazonaws.com/spec.ccfc.min/firecracker-ci/v1.10/x86_64"
for f in vmlinux-6.1.102 initramfs.cpio; do
  if [ ! -f "$FCI/$f" ]; then
    curl -sL --max-time 300 -o "$FCI/$f" "$URLBASE/$f" || { echo download-failed >> "$LOG"; exit 2; }
  fi
  echo "sha256($f)=$(sha256sum "$FCI/$f" | cut -d' ' -f1) size=$(stat -c%s "$FCI/$f")" >> "$LOG"
done

# /dev/console record (generated; see (a) above)
if [ ! -f "$FCI/initramfs-full.cpio" ]; then
  printf '#include <unistd.h>\nint main(){while(1)pause();}\n' > "$FCI/h.c"
  gcc -static -O0 -o "$FCI/hello-keepalive" "$FCI/h.c" || exit 2
  python3 - "$FCI" <<'PYEOF'
import struct, sys
def newc(name, mode, filesize=0, rdevmaj=0, rdevmin=0, data=b""):
    name_b = name.encode() + b"\0"
    f = ["070701","00000000",format(mode,"08x"),"00000000","00000000","00000001","00000000",
         format(filesize,"08x"),"00000000","00000000",format(rdevmaj,"08x"),format(rdevmin,"08x"),
         format(len(name_b),"08x"),"00000000"]
    out = "".join(f).encode() + name_b
    out += b"\0" * ((4 - len(out) % 4) % 4)
    out += data + b"\0" * ((4 - len(data) % 4) % 4)
    return out
d = sys.argv[1]
data = open(d + "/hello-keepalive","rb").read()
rec  = newc("dev", 0o040755)
rec += newc("dev/console", 0o020600, rdevmaj=5, rdevmin=1)
rec += newc("hello", 0o100755, filesize=len(data), data=data)
rec += newc("TRAILER!!!", 0)
open(d + "/fc-extras.cpio","wb").write(rec)
PYEOF
  cat "$FCI/initramfs.cpio" "$FCI/fc-extras.cpio" > "$FCI/initramfs-full.cpio"
fi
echo "initramfs-full: $(stat -c%s "$FCI/initramfs-full.cpio") bytes" >> "$LOG"

START=$(date +%s)
( cd "$FCI" && { for i in $(seq 1 22); do echo ""; echo "uname -a"; echo "id"; echo "echo VMR-FC-ARTIFACTS-OK"; sleep 6; done; echo "poweroff -f"; sleep 10; } | \
  timeout 180 qemu-system-x86_64 -accel tcg,thread=multi -smp 2 -cpu max \
    -M pc,acpi=off -m 512M \
    -kernel vmlinux-6.1.102 -initrd initramfs-full.cpio \
    -append "console=ttyS0 reboot=k panic=-1 acpi=off rdinit=/bin/sh" \
    -display none -no-reboot -monitor none -serial stdio ) >> "$LOG" 2>&1
RC=$?
END=$(date +%s)
echo "--- rc=$RC wall=$((END-START))s" >> "$LOG"
grep -aE 'VMR-FC-ARTIFACTS-OK|uid=0|Welcome to fcinitrd' "$LOG" | head -4
grep -aq 'VMR-FC-ARTIFACTS-OK' "$LOG" && exit 0 || exit 1
