#!/usr/bin/env bash
# Question: can a microVM in this sandbox BUILD software — install a compiler
# from the network inside the guest, compile, run? Alpine 3.22.5 minirootfs
# guest, apk add gcc musl-dev over slirp https. Also isolates the observed
# "posix_spawnp('cc1') ENOENT" (gcc driver) with a musl-static posix_spawn
# probe built by the bootlin musl toolchain on the host, testing spawnp of
# /bin/echo (control) and of cc1 (absolute).
# Exit codes: 0 compiled AND ran in-guest, 1 ran but failed, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/71-compile-in-guest.log"
{ echo "## conditions"; conditions compile-in-guest; echo "guest root: alpine-minirootfs-3.22.5-x86_64; apk gcc over slirp https; machine pc,acpi=off (microvm UART drops userspace writes)"; } > "$LOG"

MR="$VR_WORK/alpine-minirootfs-3.22.5-x86_64.tar.gz"
[ -f "$MR" ] || curl -sL --max-time 300 -o "$MR" \
  "https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/alpine-minirootfs-3.22.5-x86_64.tar.gz"
echo "sha256(minirootfs)=$(sha256sum "$MR" | cut -d' ' -f1)" >> "$LOG"

# host-built musl-static probe (bootlin toolchain, pinned path)
PROBE="$VR_WORK/spawn-probe"
MUSL_GCC=$(ls /opt/bootlin/x86-64-musl/bin/*-gcc | head -1)
[ -x "$MUSL_GCC" ] || { echo "no bootlin musl gcc" >> "$LOG"; exit 2; }
cat > "$VR_WORK/spawn-probe.c" <<'EOF'
#define _GNU_SOURCE
#include <spawn.h>
#include <stdio.h>
#include <errno.h>
#include <string.h>
#include <unistd.h>
#include <sys/wait.h>
extern char **environ;
static void try(const char *path, char *const argv[]) {
  pid_t pid; int rc;
  rc = posix_spawnp(&pid, path, NULL, NULL, argv, environ);
  printf("spawnp(%s) rc=%d errno=%d (%s)\n", path, rc, rc ? errno : 0, rc ? strerror(errno) : "");
  if (rc == 0) { int st; waitpid(pid, &st, 0); printf("  child exit=%d\n", WEXITSTATUS(st)); }
}
int main(void) {
  char *e1[] = {"echo", "SPAWN-ECHO-OK", NULL};
  char *cc1 = "/usr/libexec/gcc/x86_64-alpine-linux-musl/14.2.0/cc1";
  char *e2[] = {"cc1", "--version", NULL};
  try("/bin/echo", e1);
  try(cc1, e2);
  return 0;
}
EOF
"$MUSL_GCC" -static -O0 -o "$PROBE" "$VR_WORK/spawn-probe.c" 2>>"$LOG" || exit 2
echo "probe: $(stat -c%s "$PROBE") bytes static-musl" >> "$LOG"

ROOT="$VR_WORK/initrd-alpine"
rm -rf "$ROOT"; mkdir -p "$ROOT"
tar -xzf "$MR" -C "$ROOT"
mkdir -p "$ROOT"/{dev,proc,sys,tmp,root}
mkdir -p "$ROOT/lib/modules"
cp -r "$VR_WORK/vmod/lib/modules/6.12.94-0-virt" "$ROOT/lib/modules/" 2>/dev/null
cp "$PROBE" "$ROOT/spawn-probe"
cat > "$ROOT/root/bench.c" <<'EOF'
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
cat > "$ROOT/root/hello.c" <<'EOF'
#include <stdio.h>
int main(void){ printf("VMR-COMPILE-OK\n"); return 0; }
EOF
cat > "$ROOT/init" <<'EOF'
#!/bin/sh
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
modprobe virtio_net 2>/dev/null
ifconfig eth0 10.0.2.15 netmask 255.255.255.0 up
route add default gw 10.0.2.2
echo "nameserver 10.0.2.3" > /etc/resolv.conf
echo "https://dl-cdn.alpinelinux.org/alpine/v3.22/main" > /etc/apk/repositories
echo "VMR-GUEST-NET-UP"
apk update 2>&1 | tail -1
apk add --no-cache gcc musl-dev 2>&1 | tail -2 || apk add --no-cache gcc musl-dev 2>&1 | tail -2
gcc --version | head -1
echo "VMR-GUEST-GCC-VERSION"
echo "--- spawn probe (musl-static, host-built):"
/spawn-probe 2>&1
echo "--- end spawn probe"
CC1=/usr/libexec/gcc/x86_64-alpine-linux-musl/14.2.0/cc1
echo "--- access test:"
test -x "$CC1" && echo X-OK || echo X-FAIL
"$CC1" --version > /tmp/cc1v 2>&1; echo "direct rc=$? bytes=$(wc -c < /tmp/cc1v)"; cat /tmp/cc1v
echo "--- prefix diagnostics:"
echo "dumpmachine: $(gcc -dumpmachine)"
echo "prog-name(cc1): $(gcc -print-prog-name=cc1)"
gcc -print-search-dirs | head -2
echo "--- fix: expose cc1 on PATH (driver's relative prefix resolution fails under PID1)"
V=/usr/libexec/gcc/x86_64-alpine-linux-musl/14.2.0
ln -sf $V/cc1 /usr/bin/cc1
ln -sf $V/collect2 /usr/bin/collect2
cd /root
echo "--- absolute-argv0 invocation (driver prefix fix):"
/usr/bin/gcc -O2 -fno-use-linker-plugin -o hello hello.c && echo "VMR-GCC-COMPILE-OK"
/usr/bin/gcc -O2 -fno-use-linker-plugin -o bench bench.c && echo "VMR-BENCH-COMPILE-OK"
./hello
./bench 30000000
echo "VMR-BENCH-DONE"
echo o > /dev/ttyS0 2>/dev/null
poweroff -f
while :; do sleep 1; done
EOF
chmod 755 "$ROOT/init"
(cd "$ROOT" && find . | cpio -o -H newc --quiet | gzip -1) > "$VR_WORK/initramfs-alpine.cpio.gz"
echo "initramfs-alpine: $(stat -c%s "$VR_WORK/initramfs-alpine.cpio.gz") bytes" >> "$LOG"

START=$(date +%s)
SLOG="$VR_WORK/71-serial.log"
rm -f "$SLOG"
cd "$VR_WORK"
qemu-system-x86_64 -accel tcg,thread=multi \
    -M pc,acpi=off -m 1024M -smp 2 \
    -kernel vmlinuz-virt -initrd initramfs-alpine.cpio.gz \
    -append "console=ttyS0 rdinit=/init" \
    -display none -no-reboot -monitor none \
    -netdev user,id=n0 -device virtio-net-pci,netdev=n0 \
    -serial file:"$SLOG" &
QPID=$!
cd "$VR_ROOT"
# poll: done when guest halts (pc+acpi=off cannot power off) or timeout
while :; do
  sleep 5
  kill -0 $QPID 2>/dev/null || break
  grep -aq 'reboot: System halted' "$SLOG" 2>/dev/null && break
  NOW=$(date +%s); [ $((NOW-START)) -gt 1500 ] && break
done
sleep 2
kill $QPID 2>/dev/null
END=$(date +%s)
echo "--- wall=$((END-START))s" >> "$LOG"
cp "$SLOG" "$VR_LOGS/71-serial-capture.log"
grep -aE 'VMR-GCC-COMPILE-OK|VMR-COMPILE-OK|bench |spawnp' "$SLOG" | head -8 >> "$LOG"
grep -aq 'VMR-COMPILE-OK' "$SLOG" && exit 0 || exit 1
