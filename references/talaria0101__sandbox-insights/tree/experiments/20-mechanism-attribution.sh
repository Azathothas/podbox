#!/usr/bin/env bash
# 20-mechanism-attribution.sh
#
# Question: for each denial, WHICH mechanism is responsible — the
# syscall filter (F) or a kernel-internal check (capability scope,
# path policy, procfs provenance)?
#
# Method: the bogus-argument discriminator. A seccomp filter sees the
# syscall number and six registers; it cannot dereference a pointer
# and it runs before the syscall body. A syscall called with an
# argument the kernel would reject inside its own body answers:
#   path/pid-shaped errno (ENOENT/EBADF/ESRCH)  -> executed
#   EPERM for the same argument                 -> refused pre-entry
# Controls (pidfd_getfd, kcmp) prove the prober can still see the
# difference.
#
# Inputs: none. Tools: cc.
# Exit: 0 taken · 2 could not build.
set -u
cd "$(dirname "$0")" || exit 2
. ../scripts/lib.sh

OUT="$SI_LOGS/20-mechanism-attribution.log"
{
  conditions mechanism-attribution
  echo
} > "$OUT"
cc -O2 -o "$SI_WORK/attribute" "$SI_SCRIPTS/attribute.c" 2>>"$OUT" || { echo "cannot build attribute" >> "$OUT"; exit 2; }
"$SI_WORK/attribute" 2>&1 | tee -a "$OUT"
echo
echo "log: $OUT"
exit 0
