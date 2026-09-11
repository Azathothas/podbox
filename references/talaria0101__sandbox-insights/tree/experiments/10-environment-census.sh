#!/usr/bin/env bash
# 10-environment-census.sh
#
# Question: what does this runtime actually permit, by mechanism —
# identity (N), syscall filter (F), path/provenance policy (M), and the
# unfiltered modern interfaces the filter forgot?
#
# Every claim in docs/environment.md and the paper depends on this
# census; it is number 10 because everything after it interprets it.
#
# Inputs: none (pure runtime probe). Tools: cc.
# Exit: 0 census taken (denials are results) · 2 could not build probe.
set -u
cd "$(dirname "$0")" || exit 2
. ../scripts/lib.sh

OUT="$SI_LOGS/10-environment-census.log"
{
  conditions environment-census
  echo "glibc: $(ldd --version 2>/dev/null | head -1)"
  echo
} > "$OUT"

cc -O2 -o "$SI_WORK/census" "$SI_SCRIPTS/census.c" 2>>"$OUT" || { echo "cannot build census" >> "$OUT"; exit 2; }
"$SI_WORK/census" 2>&1 | tee -a "$OUT"

echo
echo "log: $OUT"
# The census itself always succeeds: denials are its output.
exit 0
