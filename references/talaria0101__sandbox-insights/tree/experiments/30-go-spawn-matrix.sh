#!/usr/bin/env bash
# 30-go-spawn-matrix.sh
#
# Question: which os/exec SysProcAttr shapes make Go call setgroups in
# the child, on a runtime where the user namespace denies setgroups?
#
# Why it matters: a non-nil Credential fails the spawn with a bare
# "operation not permitted" — the identical string a refused clone
# produces. Tools have been "fixed" by dropping Credential when the
# one-field NoSetGroups:true is the correct, no-op-elsewhere fix.
#
# Inputs: none. Tools: go.
# Exit: 0 matrix taken · 2 could not run.
set -u
cd "$(dirname "$0")" || exit 2
. ../scripts/lib.sh

OUT="$SI_LOGS/30-go-spawn-matrix.log"
{
  conditions go-spawn-matrix
  echo "go: $(go version)"
  echo
} > "$OUT"
cd "$SI_WORK" || exit 2
if go run "$SI_SCRIPTS/spawn_matrix.go" >>"$OUT" 2>&1; then
  tail -16 "$OUT"
  echo
  echo "log: $OUT"
  exit 0
else
  echo "go run failed" >> "$OUT"
  tail -5 "$OUT"
  exit 2
fi
