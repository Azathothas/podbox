#!/usr/bin/env bash
# Question: pullrun/pullrun v0.7.9 — dual-mode (runc container OR firecracker
# microVM from the same OCI image). BOTH modes are kernel-facility dependent
# here: container mode wants namespaces/mounts (seccomp EPERM), VM mode wants
# /dev/kvm (ENOENT). Which errno does each mode actually hit?
# Exit codes: 0 one mode works, 1 both blocked, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/43-pullrun.log"
conditions pullrun > "$LOG"
BIN="$VR_WORK/bins/pullrun/pullrun-linux-amd64"
RT="$VR_WORK/bins/pullrun/pullrun-runtime-linux-amd64"
cp -f "$RT" "$VR_WORK/bins/pullrun/pullrun-runtime"; chmod +x "$VR_WORK/bins/pullrun/pullrun-runtime"
PATH="$VR_WORK/bins/pullrun:$PATH"
[ -x "$BIN" ] || { echo "binary missing" >> "$LOG"; exit 2; }
timeout 15 "$BIN" --version >> "$LOG" 2>&1
timeout 15 "$BIN" run --help >> "$LOG" 2>&1
echo "--- container mode: pullrun run alpine:3.22 echo hi" >> "$LOG"
timeout 180 "$BIN" run docker.io/library/alpine:3.22 >> "$LOG" 2>&1
RC1=$?
echo "--- rc=$RC1" >> "$LOG"
echo "--- vm mode (flag from run --help, if any)" >> "$LOG"
grep -iE 'vm|microvm|firecracker' "$LOG" | head -5
timeout 180 "$BIN" run docker.io/library/alpine:3.22 --backend=vm --kernel-image pullrun/kernel-asahi:6.19.14 >> "$LOG" 2>&1
RC2=$?
echo "--- rc=$RC2" >> "$LOG"
echo "--- decision: container rc=$RC1 vm rc=$RC2" >> "$LOG"
tail -25 "$LOG"
[ "$RC1" -eq 0 ] || [ "$RC2" -eq 0 ] && exit 0 || exit 1
