#!/usr/bin/env bash
# 45-senotif-primitives.sh
#
# Question: can a syscall TRACER exist where the ptrace syscall class
# is denied outright? The tracee can always seccomp ITSELF, so the
# question reduces to six primitives, each proven by this script:
#
#   P1  a process can install SECCOMP_FILTER_FLAG_NEW_LISTENER on itself
#   P2  the listener fd crosses to the tracer over a unix socket (SCM_RIGHTS)
#   P3  NOTIF_RECV/SEND with USER_NOTIF_FLAG_CONTINUE shepherd every syscall
#   P4  a forged return value (SKIP + sequenced retval) is accepted
#   P5  SECCOMP_IOCTL_NOTIF_ADDFD injects a REAL fd into the tracee
#   P6  /proc/<pid>/mem serves memory arguments while the tracee is
#       blocked in the notification (stable, no TOCTOU window)
#
# Negative results are results: on a host where user-notification is
# unavailable this exits 2 and says which primitive failed.
#
# Inputs: none. Tools: cc.
# Exit: 0 primitives proven · 1 a primitive failed · 2 unavailable.
set -u
cd "$(dirname "$0")" || exit 2
. ../scripts/lib.sh

OUT="$SI_LOGS/45-senotif-primitives.log"
{
  conditions senotif-primitives
  echo
} > "$OUT"
cc -O2 -o "$SI_WORK/senotif_probe" "$SI_SCRIPTS/senotif_probe.c" 2>>"$OUT" || { echo "cannot build senotif_probe" >> "$OUT"; exit 2; }
"$SI_WORK/senotif_probe" 2>&1 | tee -a "$OUT"
rc=${PIPESTATUS[0]}
echo
echo "log: $OUT"
exit "$rc"
