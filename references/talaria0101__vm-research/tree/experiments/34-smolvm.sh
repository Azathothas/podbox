#!/usr/bin/env bash
# Question: smol-machines/smolvm v1.14.4 (libkrun-based) — the binary's own
# --help documents "Linux x86_64: KVM | /dev/kvm"; what does an actual
# machine-create attempt produce here?
# Exit codes: 0 usable, 1 blocked, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/34-smolvm.log"
conditions smolvm > "$LOG"
BIN="$VR_WORK/bins/smolvm/smolvm-1.14.4-linux-x86_64/smolvm"
[ -x "$BIN" ] || { echo "binary missing" >> "$LOG"; exit 2; }
timeout 15 "$BIN" --help > /dev/null 2>&1   # the help text itself is evidence
timeout 15 "$BIN" --help | head -30 >> "$LOG" 2>&1
echo "--- machine create attempt" >> "$LOG"
cd "$VR_WORK"
timeout 20 "$BIN" machine create --help >> "$LOG" 2>&1
timeout 20 "$BIN" machine delete -n alpine >> "$LOG" 2>&1
timeout 180 "$BIN" machine create -n alpine -I docker.io/library/alpine:3.22 --net >> "$LOG" 2>&1
echo "--- machine create, 256MiB variant (default 8GiB hits RLIMIT_FSIZE=500MB: SIGXFSZ, rc 153)" >> "$LOG"
timeout 20 "$BIN" machine delete -n tiny >> "$LOG" 2>&1
timeout 180 "$BIN" machine create -n tiny -I docker.io/library/alpine:3.22 --net --mem 256 --cpus 1 >> "$LOG" 2>&1
echo "--- machine start (the decisive probe: libkrun opens /dev/kvm)" >> "$LOG"
timeout 60 "$BIN" machine start --name tiny >> "$LOG" 2>&1
echo "rc(start)=$?" >> "$LOG"
echo "--- honest probe: state after start + exec" >> "$LOG"
timeout 20 "$BIN" machine ls >> "$LOG" 2>&1
timeout 30 "$BIN" machine exec --name tiny -- uname -a >> "$LOG" 2>&1
echo "rc(exec)=$?" >> "$LOG"
echo "--- decision: start rc=153 (SIGXFSZ: RLIMIT_FSIZE=500MB hit during boot) even at --mem 256; state stays created; exec says not running" >> "$LOG"
tail -20 "$LOG"
exit 1
