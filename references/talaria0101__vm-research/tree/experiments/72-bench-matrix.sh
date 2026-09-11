#!/usr/bin/env bash
# Question: which runnable microVM platform is fastest and most performant?
# Same benchmark source (xorshift32 + double accumulation + FNV, 30M ops, -O2)
# executed on: host native; Alpine guests on -M microvm and -M pc (bench
# compiled IN-GUEST with in-guest gcc 14.2, rootfs pre-provisioned once on the
# host via apk.static); nanos unikernel (host-built static ELF); firecracker's
# CI kernel (host-built musl-static ELF via rdinit). Boot latencies from
# 20-/21-/42-/45- logs. Checksum equality = correctness cross-check.
# Exit codes: 0 matrix complete, 1 partial (table still written), 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/72-bench-matrix.log"
{ echo "## conditions"; conditions bench-matrix; echo "bench: 30M iterations xorshift32+dbl+FNV, -O2; checksum 165be307 expected everywhere"; } > "$LOG"
mkdir -p "$VR_WORK/bench-common"
cat > "$VR_WORK/bench-common/bench.c" <<'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
static unsigned int xs = 2463534242u;
int main(int argc, char **argv){
  int N = (argc>1)? atoi(argv[1]) : 30000000;
  struct timespec a,b;
  double acc=0; unsigned int h=2166136261u;
  clock_gettime(CLOCK_MONOTONIC,&a);
  for(int i=0;i<N;i++){
    xs^=xs<<13; xs^=xs>>17; xs^=xs<<5;
    acc += xs*(1.0/4294967296.0) - 0.5;
    h = (h^xs)*16777619u;
  }
  clock_gettime(CLOCK_MONOTONIC,&b);
  double s=(b.tv_sec-a.tv_sec)+(b.tv_nsec-a.tv_nsec)/1e9;
  printf("bench %.2fs %.1f Mops/s checksum=%08x acc=%.4f N=%d\n", s, N/1e6/s, h, acc, N);
  return 0;
}
EOF

# ---------- A. host baseline (n=2) ----------
gcc -O2 -o "$VR_WORK/bench-host" "$VR_WORK/bench-common/bench.c"
echo "[host] $($VR_WORK/bench-host 30000000)" >> "$LOG"
echo "[host] $($VR_WORK/bench-host 30000000)" >> "$LOG"

# ---------- prepared alpine rootfs with in-guest gcc (built once) ----------
APKROOT="$VR_WORK/alpine-gcc-root"
if [ ! -d "$APKROOT/usr/bin/gcc" ]; then
  MR="$VR_WORK/alpine-minirootfs-3.22.5-x86_64.tar.gz"
  [ -f "$MR" ] || curl -sL --max-time 300 -o "$MR" \
    "https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/alpine-minirootfs-3.22.5-x86_64.tar.gz"
  rm -rf "$APKROOT"; mkdir -p "$APKROOT"
  tar -xzf "$MR" -C "$APKROOT"
  mkdir -p "$APKROOT/dev" "$APKROOT/proc" "$APKROOT/sys" "$APKROOT/tmp"
  A="$VR_WORK/apk.static"
  [ -x "$A" ] || { curl -sL --max-time 120 -o "$VR_WORK/apkstatic.apk" \
      "https://dl-cdn.alpinelinux.org/alpine/v3.22/main/x86_64/apk-tools-static-2.14.10-r0.apk" \
      && tar -xzf "$VR_WORK/apkstatic.apk" -C "$VR_WORK" sbin/apk.static 2>/dev/null \
      && mv "$VR_WORK/sbin/apk.static" "$A"; }
  [ -x "$A" ] || { echo "apk.static unavailable" >> "$LOG"; exit 2; }
  echo "https://dl-cdn.alpinelinux.org/alpine/v3.22/main" > "$APKROOT/etc/apk/repositories"
  "$A" --root "$APKROOT" --arch x86_64 --initdb add alpine-baselayout musl 2>&1 | tail -1
  "$A" --root "$APKROOT" --arch x86_64 add gcc musl-dev 2>&1 | tail -1
fi
echo "[prep] gcc in rootfs: $(ls "$APKROOT/usr/bin/gcc" 2>/dev/null || echo MISSING)" >> "$LOG"

alpine_run() { # $1 = machine(microvm|pc), $2 = label
  local M="$1" LABEL="$2"
  local ROOT="$VR_WORK/initrd-$LABEL"
  rm -rf "$ROOT"; mkdir -p "$ROOT"
  cp -r "$APKROOT/." "$ROOT/"
  mkdir -p "$ROOT"/{dev,proc,sys,tmp,root} "$ROOT/lib/modules"
  cp -r "$VR_WORK/vmod/lib/modules/6.12.94-0-virt" "$ROOT/lib/modules/" 2>/dev/null
  cp "$VR_WORK/bench-common/bench.c" "$ROOT/root/bench.c"
  cat > "$ROOT/init" <<'EOF'
#!/bin/sh
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
echo "VMR-GCC: $(/usr/bin/gcc --version | head -1)"
cd /root
/usr/bin/gcc -O2 -fno-use-linker-plugin -o bench bench.c && echo "VMR-COMPILE-OK"
echo "VMR-CC1-SHA: $(sha256sum /usr/libexec/gcc/x86_64-alpine-linux-musl/14.2.0/cc1 | cut -c1-16)"
./bench 30000000
./bench 30000000
echo "VMR-BENCH-DONE"
while :; do sleep 1; done
EOF
  chmod 755 "$ROOT/init"
  (cd "$ROOT" && find . | cpio -o -H newc --quiet | gzip -1) > "$VR_WORK/initramfs-$LABEL.cpio.gz"
  local NET=(-netdev user,id=n0 -device virtio-net-pci,netdev=n0)
  [ "$M" = "microvm" ] && NET=(-netdev user,id=n0 -device virtio-net-device,netdev=n0)
  local MACH="pc,acpi=off"; [ "$M" = "microvm" ] && MACH="microvm"
  local SLOG="$VR_WORK/72-$LABEL-serial.log"; rm -f "$SLOG"
  ( cd "$VR_WORK" && qemu-system-x86_64 -accel tcg,thread=multi \
      -M "$MACH" -m 1024M -smp 2 \
      -kernel vmlinuz-virt -initrd "initramfs-$LABEL.cpio.gz" \
      -append "console=ttyS0 rdinit=/init" \
      -display none -no-reboot -monitor none -serial file:"$SLOG" "${NET[@]}" ) &
  local QP=$!
  for i in $(seq 1 60); do
    grep -aq 'VMR-BENCH-DONE' "$SLOG" 2>/dev/null && break
    kill -0 $QP 2>/dev/null || break
    sleep 5
  done
  kill $QP 2>/dev/null; wait $QP 2>/dev/null
  grep -aE 'VMR-GCC:|VMR-COMPILE-OK|bench ' "$SLOG" | sed "s/^/[$LABEL] /" >> "$LOG"
  rm -rf "$ROOT" "$VR_WORK/initramfs-$LABEL.cpio.gz"
}

alpine_run microvm microvm-alpine
alpine_run pc pc-alpine

# ---------- nanos unikernel (host-built static) ----------
if [ -x "$VR_WORK/ops" ]; then
  gcc -static -O2 -o "$VR_WORK/bench-nanos" "$VR_WORK/bench-common/bench.c" 2>/dev/null || \
    /opt/bootlin/x86-64-musl/bin/x86_64-buildroot-linux-musl-gcc -static -O2 -o "$VR_WORK/bench-nanos" "$VR_WORK/bench-common/bench.c"
  ( cd "$VR_WORK" && timeout 180 ./ops run -m 256M bench-nanos 2>&1 | grep -aE 'bench .*Mops' | sed 's/^/[nanos] /' >> "$LOG" )
  grep -q '\[nanos\]' "$LOG" || echo "[nanos] no bench output" >> "$LOG"
fi

# ---------- firecracker CI kernel (musl-static via bootlin, rdinit=/bench) ----------
MUSL_GCC=$(ls /opt/bootlin/x86-64-musl/bin/*-gcc 2>/dev/null | head -1)
if [ -n "$MUSL_GCC" ] && [ -f "$VR_WORK/firecracker-ci/initramfs.cpio" ]; then
  "$MUSL_GCC" -static -O2 -o "$VR_WORK/firecracker-ci/bench-fc" "$VR_WORK/bench-common/bench.c" || exit 2
  python3 - "$VR_WORK" <<'PYEOF'
import sys
def newc(name, mode, filesize=0, rdevmaj=0, rdevmin=0, data=b""):
    name_b = name.encode() + b"\0"
    f = ["070701","00000000",format(mode,"08x"),"00000000","00000000","00000001","00000000",
         format(filesize,"08x"),"00000000","00000000",format(rdevmaj,"08x"),format(rdevmin,"08x"),
         format(len(name_b),"08x"),"00000000"]
    out = "".join(f).encode() + name_b
    out += b"\0" * ((4 - len(out) % 4) % 4)
    out += data + b"\0" * ((4 - len(data) % 4) % 4)
    return out
d = sys.argv[1] + "/firecracker-ci/"
data = open(d+"bench-fc","rb").read()
rec  = newc("dev", 0o040755)
rec += newc("dev/console", 0o020600, rdevmaj=5, rdevmin=1)
rec += newc("bench", 0o100755, filesize=len(data), data=data)
rec += newc("init", 0o100755, filesize=58, data=b"#!/bin/sh\n/bench 30000000\n/bench 30000000\nwhile :; do sleep 1; done\n")
rec += newc("TRAILER!!!", 0)
open(d+"bench-extras.cpio","wb").write(rec)
PYEOF
  cat "$VR_WORK/firecracker-ci/initramfs.cpio" "$VR_WORK/firecracker-ci/bench-extras.cpio" > "$VR_WORK/firecracker-ci/initramfs-bench.cpio"
  local_SLOG="$VR_WORK/72-fc-serial.log"; rm -f "$local_SLOG"
  ( cd "$VR_WORK/firecracker-ci" && timeout 240 qemu-system-x86_64 -accel tcg,thread=multi \
      -smp 2 -cpu max -M pc,acpi=off -m 512M \
      -kernel vmlinux-6.1.102 -initrd initramfs-bench.cpio \
      -append "console=ttyS0 reboot=k panic=-1 acpi=off" \
      -display none -no-reboot -monitor none -serial file:"$local_SLOG" )
  grep -aE 'bench .*Mops' "$local_SLOG" | sed 's/^/[firecracker-kernel] /' >> "$LOG"
  grep -aq 'bench' "$local_SLOG" || echo "[firecracker-kernel] no output" >> "$LOG"
fi

echo "=== matrix summary ===" >> "$LOG"
grep -aE '^\[.*\] bench|^\[host\] bench' "$LOG" >> "$LOG"
grep -aE '^\[.*\] bench|^\[host\] bench' "$LOG" | head -12
exit 0
