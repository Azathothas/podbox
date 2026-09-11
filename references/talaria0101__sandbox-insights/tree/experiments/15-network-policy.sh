#!/usr/bin/env bash
# 15-network-policy.sh
#
# Question: what does the network envelope permit, and what mechanism
# do the denials imply?
#
# A seccomp filter cannot dereference the sockaddr pointer, so denials
# that are address-, port- and family-aware belong to a layer below
# the syscall boundary (cgroup-BPF SOCK_ADDR class). This census
# produces the rows that argument rests on.
#
# Inputs: none. Tools: python3.
# Exit: 0 census taken.
set -u
cd "$(dirname "$0")" || exit 2
. ../scripts/lib.sh

OUT="$SI_LOGS/15-network-policy.log"
{
  conditions network-policy
  echo
} > "$OUT"
python3 "$SI_SCRIPTS/netpolicy.py" 2>&1 | tee -a "$OUT"
echo
echo "log: $OUT"
exit 0
