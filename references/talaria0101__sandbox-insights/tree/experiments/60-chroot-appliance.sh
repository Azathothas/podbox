#!/usr/bin/env bash
# 60-chroot-appliance.sh
#
# Question: what does the CHEAPEST isolated-ish route — a plain
# chroot(2) into a real distribution rootfs — buy, and what does it
# cost, measured?
#
# Buys (proven here): a real Alpine appliance, entered by chroot,
# running the same benchmark at NATIVE speed with an identical
# checksum; package management usable in principle (the rootfs ships
# its own /etc/passwd, unlike the sandbox).
#
# Costs (all in the same run): mount(2) is denied so there is no /proc
# (ps sees nothing); mknod is denied so /dev/null cannot exist as a
# device (redirection fails); and there is NO kernel boundary — the
# workload shares the tenant's kernel, PID space and every namespace,
# and the classic double-chroot escape PRECONDITION holds for uid 0
# (a second chroot() succeeds; the demo stops at the precondition).
#
# Inputs: pinned Alpine minirootfs + host-built static bench. Tools: chroot, tar, cc.
# Exit: 0 measured · 1 unexpected · 2 could not run.
set -u
cd "$(dirname "$0")" || exit 2
. ../scripts/lib.sh

OUT="$SI_LOGS/60-chroot-appliance.log"
{
  conditions chroot-appliance
  echo
} > "$OUT"

vfetch minirootfs.tar.gz "$PIN_MINIROOTFS_URL" "$PIN_MINIROOTFS_SHA" >>"$OUT" 2>&1 || { echo "pin fetch failed" >>"$OUT"; exit 2; }
cc -O2 -static -o "$SI_WORK/bench" "$SI_SCRIPTS/bench.c" 2>>"$OUT" || exit 2
HOST_BENCH="$("$SI_WORK/bench")"
HOST_CKSUM="$(sed -n 's/.*checksum=\([0-9a-f]*\).*/\1/p' <<< "$HOST_BENCH")"

A="$SI_WORK/appliance"; rm -rf "$A"; mkdir -p "$A"
# ownership-neutral extraction (the wall of experiment 25 applies here too)
tar -xzf "$SI_WORK/minirootfs.tar.gz" --no-same-owner -C "$A" 2>>"$OUT" || { echo "extract failed" >> "$OUT"; exit 2; }
# copy-in transfer: mounts are impossible, so seeding is cp, not bind
cp "$SI_WORK/bench" "$A/bench"
cp /etc/resolv.conf "$A/etc/resolv.conf" 2>/dev/null || true

{
  echo "## host bench: $HOST_BENCH"
  echo
  echo "## in-appliance (chroot, native code, no kernel boundary):"
  chroot "$A" /bench
  echo "chroot bench rc: $?"
  echo
  echo "## what the appliance lacks, measured from inside:"
  chroot "$A" /bin/sh -c '
    echo "ps:"; ps 2>&1 | head -3
    echo "redirection to /dev/null:"; echo hi > /dev/null 2>&1 || echo "  FAILS: /dev/null is absent (mknod denied; no devtmpfs mount)"
    echo "identity inside:"; id
    echo "own /etc/passwd (rootfs ships one):"; head -2 /etc/passwd
  '
  echo
  echo "## the escape PRECONDITION (not an escape): a second chroot from inside"
  mkdir -p "$A/jail"
  cp "$SI_WORK/bb/bin/busybox.static" "$A/jail/busybox"
  chroot "$A" /bin/sh -c '/jail/busybox chroot /jail /busybox echo VMR-CHROOT-TWICE-OK uid-$(id -u)-chroots-again-precondition-holds' 2>&1
} 2>&1 | tee -a "$OUT"

rc=0
grep -q "checksum=" "$OUT" || rc=1
# host and chroot checksums must match
HOST_CK="$(sed -n 's/.*checksum=\([0-9a-f]*\).*/\1/p' <<< "$HOST_BENCH")"
GUEST_CK="$(sed -n '/## in-appliance/,/chroot bench rc/p' "$OUT" | sed -n 's/.*checksum=\([0-9a-f]*\).*/\1/p' | head -1)"
if [ "$HOST_CK" != "$GUEST_CK" ]; then
  echo "GATE FAIL: chroot checksum $GUEST_CK differs from host $HOST_CK" >> "$OUT"; rc=1
fi
grep -q "VMR-CHROOT-TWICE-OK" "$OUT" || { echo "note: double-chroot precondition not reached" >> "$OUT"; }
echo
echo "log: $OUT"
exit $rc
