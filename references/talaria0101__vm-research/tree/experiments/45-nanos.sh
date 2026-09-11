#!/usr/bin/env bash
# Question: nanovms/nanos (via ops 0.1.46 CLI) — a unikernel: one app, one
# kernel, no guest userspace. Can ops fetch the nanos release, build an image
# from a real static ELF, and boot it under qemu here (TCG)?
# Exit codes: 0 guest ran and printed its proof line, 1 ran but failed, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/45-nanos.log"
conditions nanos > "$LOG"

OPS="$VR_WORK/ops"
[ -x "$OPS" ] || { echo "ops missing" >> "$LOG"; exit 2; }
"$OPS" version >> "$LOG" 2>&1

# the payload: a real static ELF that prints a proof line
HELLO="$VR_WORK/hello-nanos"
if [ ! -x "$HELLO" ]; then
  cat > "$VR_WORK/hello.c" <<'EOF'
#include <stdio.h>
int main(void){ printf("VMR-NANOS-OK pid=%d\n", getpid()); return 0; }
EOF
  grep -q unistd "$VR_WORK/hello.c" || sed -i '1i #include <unistd.h>' "$VR_WORK/hello.c"
  gcc -static -O2 -o "$HELLO" "$VR_WORK/hello.c" || exit 2
fi
file "$HELLO" >> "$LOG"

START=$(date +%s)
( cd "$VR_WORK" && timeout 300 "$OPS" run -m 256M "$HELLO" ) >> "$LOG" 2>&1
RC=$?
END=$(date +%s)
echo "--- rc=$RC wall=$((END-START))s" >> "$LOG"
tail -14 "$LOG"
grep -q 'VMR-NANOS-OK' "$LOG" && exit 0 || exit 1
