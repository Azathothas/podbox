#!/usr/bin/env bash
# Question: user-mode Linux (Debian's user-mode-linux 7.1um1, UML v7.1.3,
# kernel 6.12-based) needs NO /dev/kvm — the "kernel facility" it needs is
# only plain process syscalls. But its guest mounts execute as THIS process's
# syscalls, which the seccomp filter EPGERMs. Can it still boot an initramfs
# (initramfs needs no mount(2) to populate rootfs) and run userspace to a
# proof marker, tolerating failed mounts?
# Exit codes: 0 proof marker printed by guest init, 1 ran but failed, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/50-uml.log"
conditions uml > "$LOG"

UML="$VR_WORK/linux-uml"
INITRD="$VR_WORK/initramfs-bb.cpio.gz"
[ -x "$UML" ] || { echo "UML binary missing" >> "$LOG"; exit 2; }
[ -f "$INITRD" ] || { echo "initramfs missing (run 20-)" >> "$LOG"; exit 2; }
"$UML" --help 2>&1 | head -2 >> "$LOG"

START=$(date +%s)
( cd "$VR_WORK" && timeout 120 "$UML" \
    mem=256M \
    initrd=initramfs-bb.cpio.gz \
    root=/dev/ram0 \
    con=fd:0,fd:1 ssl=fd:0,fd:1 \
    umid=vmr50 2>&1 ) >> "$LOG"
RC=$?
END=$(date +%s)
echo "--- rc=$RC wall=$((END-START))s" >> "$LOG"
grep -m5 -E 'VMR-|Kernel panic|mount' "$LOG" >> "$LOG"
tail -8 "$LOG"
grep -q 'VMR-PROOF-END' "$LOG" && exit 0 || exit 1
