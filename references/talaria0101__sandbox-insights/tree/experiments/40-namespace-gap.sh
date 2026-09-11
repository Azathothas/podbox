#!/usr/bin/env bash
# 40-namespace-gap.sh
#
# Question: the denylist refuses the LEGACY namespace syscalls
# (unshare/setns) — does the clone family create namespaces anyway,
# and what do the resulting namespace-OWNER privileges buy, in-process?
#
# A number-denylist that predates clone3 blocks the old door and leaves
# the new one open. A process entering CLONE_NEWUSER|CLONE_NEWNET
# through clone3 OWNS those namespaces; owner-namespace capability
# checks pass for it. Demonstrated: raw-ICMP socket as netns owner,
# CLONE_NEWPID private process tree — all in-process, no helper binary.
#
# The boundary (measured, same run): uid_map stays EMPTY — /proc is a
# mount from the initial user namespace and map_write() checks against
# the MOUNT's userns — so an execve from the unmapped uid drops every
# granted capability. The gap buys in-process privileges, not a
# container runtime.
#
# THIS GAP IS OPERATOR-CLOSABLE. A closed gap is a committed negative
# result, not a failure: see the exit code convention.
#
# Inputs: none. Tools: cc.
# Exit: 0 gap open and demonstrated · 1 gap closed (negative result,
#       still evidence) · 2 could not build.
set -u
cd "$(dirname "$0")" || exit 2
. ../scripts/lib.sh

OUT="$SI_LOGS/40-namespace-gap.log"
{
  conditions namespace-gap
  echo
} > "$OUT"
cc -O2 -o "$SI_WORK/nspriv" "$SI_SCRIPTS/nspriv.c" 2>>"$OUT" || { echo "cannot build nspriv" >> "$OUT"; exit 2; }
"$SI_WORK/nspriv" 2>&1 | tee -a "$OUT"
rc=${PIPESTATUS[0]}
echo
echo "log: $OUT"
exit "$rc"
