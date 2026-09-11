#!/usr/bin/env bash
# Question: the third route — plain `chroot(2)` into an appliance rootfs.
# What does it deliver natively (speed, toolchain installs) and what are its
# hard limitations (no kernel boundary, no /proc, no /dev, escape-ability)?
# Everything measured in THIS sandbox; the escape demonstration uses our own
# appliance and workspace only.
# Exit codes: 0 route characterized, 1 partial, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/76-chroot-route.log"
{ echo "## conditions"; conditions chroot-route; echo "appliance: alpine 3.22.5 minirootfs + own /etc/passwd (work/alpine-chroot)"; } > "$LOG"

CH="$VR_WORK/alpine-chroot"
if [ ! -x "$CH/bin/sh" ]; then
  MR="$VR_WORK/alpine-minirootfs-3.22.5-x86_64.tar.gz"
  [ -f "$MR" ] || curl -sL --max-time 300 -o "$MR" \
    "https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/alpine-minirootfs-3.22.5-x86_64.tar.gz"
  rm -rf "$CH"; mkdir -p "$CH"
  tar --no-same-owner -xzf "$MR" -C "$CH"
fi
# appliance self-setup: resolv + repos (its own /etc — the passwd fix from 70-)
echo "nameserver 1.1.1.1" > "$CH/etc/resolv.conf"
echo "https://dl-cdn.alpinelinux.org/alpine/v3.22/main" > "$CH/etc/apk/repositories" 2>/dev/null || \
  echo "https://dl-cdn.alpinelinux.org/alpine/v3.22/main" > "$CH/etc/apk/repositories"

BENCH="$VR_WORK/bench-common/bench.c"
[ -f "$BENCH" ] || { echo "bench.c missing (run 72-)" >> "$LOG"; exit 2; }
mkdir -p "$CH/root"
cp "$BENCH" "$CH/root/bench.c"

# ---- the chroot session: runs INSIDE chroot; output to a file on the workspace
cat > "$CH/session.sh" <<'EOF'
export PATH=/bin:/sbin:/usr/bin:/usr/sbin
echo "=== inside chroot"
head -1 /etc/passwd
echo "--- /proc mount test:"
mount -t proc proc /proc 2>&1 | head -1; echo "mount rc=$?"
echo "--- /dev/null test:"
echo x > /dev/null 2>&1; echo "devnull rc=$?"
echo "--- ps:"
ps 2>&1 | head -2
echo "--- apk add gcc (in-chroot, over network):"
apk update >/dev/null 2>&1; apk add --no-cache gcc musl-dev 2>&1 | tail -1
gcc --version | head -1
cd /root
gcc -O2 -o bench bench.c && echo "CHROOT-COMPILE-OK"
./bench 30000000
./bench 30000000
echo "=== session done"
EOF
chmod 755 "$CH/session.sh"

# ---- chroot entry + session execution
cat > /workspace/vm-research/experiments/work/76-driver.sh <<'DRIVER'
#!/bin/bash
chroot "$1" /bin/sh /session.sh
rc=$?
echo "session rc=$rc"
# escape demonstration: classic double-chroot for euid 0 with cwd outside
# (we are uid0 in our userns; the sandbox root is OUTSIDE the chroot and we
#  still hold a cwd there? no — but the WELL-KNOWN structure needs a dir in
#  the chroot + an fd/cwd outside. We demonstrate the STANDARD precondition
#  instead: chroot does not confine a process that can chroot again.)
exit $rc
DRIVER
chmod 755 /workspace/vm-research/experiments/work/76-driver.sh

timeout 900 /workspace/vm-research/experiments/work/76-driver.sh "$CH" > "$VR_WORK/76-session.log" 2>&1
RC=$?
grep -av 'random\|crng' "$VR_WORK/76-session.log" | head -20 >> "$LOG"
echo "--- session rc=$RC" >> "$LOG"

# ---- escape-precondition probe (outside the appliance): can a chrooted
# root re-chroot out? The classic requires: mkdir in chroot + chroot into it
# + chdir.. we test the API availability instead: chroot again after chroot.
cat > /workspace/vm-research/experiments/work/76-escape.c <<'EOF'
#define _GNU_SOURCE
#include <unistd.h>
#include <stdio.h>
#include <errno.h>
#include <string.h>
#include <sys/stat.h>
int main(void){
  /* classic escape preconditions: euid==0, chroot() callable twice */
  if (mkdir("/esc", 0755) && errno != EEXIST){ printf("mkdir /esc: %s\n", strerror(errno)); return 1; }
  if (chroot("/esc")) { printf("second chroot: %s\n", strerror(errno)); return 1; }
  chdir("/");
  /* we are now rooted at an empty dir; escape needs a fd/cwd outside created
     BEFORE the first chroot — the classic works whenever the process can
     chroot twice and holds any dirfd/cwd outside. API availability: */
  printf("VMR-CHROOT-TWICE-OK (double-chroot precondition present)\n");
  return 0;
}
EOF
gcc -O2 -static -o "$CH/esc" /workspace/vm-research/experiments/work/76-escape.c 2>/dev/null && \
  timeout 30 chroot "$CH" /esc >> "$LOG" 2>&1

grep -aq 'CHROOT-COMPILE-OK' "$LOG" && exit 0 || exit 1
