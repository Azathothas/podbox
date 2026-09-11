#!/usr/bin/env bash
# Question: NetBSDfr/smolBSD — qemu-backed NetBSD microVMs with native TCG
# fallback ("KVM unavailable, falling back to TCG"). Does a real NetBSD microVM
# (prebuilt base image + SMOL kernel, both upstream-published) boot to a login
# prompt on the serial console in this sandbox?
# Inputs: smolBSD @ <pinned commit> (recorded below), build-amd64.img.xz from
# their release "latest" (50,196,548 bytes), netbsd-SMOL kernel (sha256-checked
# by their kernfetch). Console = mon:stdio; accelerator forced tcg.
# Exit codes: 0 boot reached NetBSD userspace markers, 1 ran but failed, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/42-smolbsd.log"
conditions smolbsd > "$LOG"

SRC="$VR_WORK/smolbsd-src"
[ -d "$SRC" ] || exit 2
( cd "$SRC" && git rev-parse HEAD ) >> "$LOG" 2>&1
IMG="$VR_WORK/build-amd64.img"
[ -f "$IMG" ] || exit 2
echo "image: $IMG ($(stat -c%s "$IMG") bytes)" >> "$LOG"

START=$(date +%s)
( cd "$SRC" && cp -f "$IMG" ./build-amd64.img && { sleep 310; echo ""; sleep 8; echo "uname -a; echo VMR-SMOLBSD-OK"; sleep 12; echo "/sbin/shutdown -p now"; sleep 60; } | QEMU_ACCEL=tcg timeout 420 ./startnb.sh -i build-amd64.img ) >> "$LOG" 2>&1
RC=$?
END=$(date +%s)
echo "--- rc=$RC wall=$((END-START))s" >> "$LOG"
echo "--- boot markers:" >> "$LOG"
NB=$(grep -cE 'NetBSD|net\.inet' "$LOG"); echo "netbsd-marker-lines: $NB" >> "$LOG"
grep -m3 -E 'Enter pathname of shell|VMR-SMOLBSD-OK|NetBSD \(' "$LOG" >> "$LOG"
tail -12 "$LOG"
grep -qE 'Enter pathname of shell|VMR-SMOLBSD-OK' "$LOG" && exit 0 || exit 1
