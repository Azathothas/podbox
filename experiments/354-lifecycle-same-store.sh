#!/bin/sh
# Question: does the full lifecycle run green in one store on a
# chroot-capable lane, where 353's per-clause stores could not show it?
#
# Corrective re-run for `experiments/353-open-issue-triage.sh`, whose
# one-store-per-clause shape made every cross-clause verb (`start`,
# `exec`, `logs`, `stop`, `rm`) answer `no such container`: each clause
# ran in a fresh store holding no record. This drive holds one store
# for the whole loop. Pinned input is the alpine digest below (the
# same bytes 353 and 163 use).
#
#   sh experiments/354-lifecycle-same-store.sh
#
# Exit: 0 the loop ran end to end, 1 a verb failed, 2 the lane could
# not run (no build, no network).
set -u

# The job travels as /work/.podbox-job.sh: take the checkout from the
# working directory, never from $0 (see 353 for the measurement).
REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep354-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-354.txt"

ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "image input       $ALPINE"
} >"$REPORT"

command -v cargo >/dev/null 2>&1 || { echo "cargo missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
echo "== bootstrap the lane toolchain"
if timeout 1200 ./scripts/common/bootstrap-env.sh rust cc zig tools >>"$WORK/bootstrap.log" 2>&1; then
	echo "bootstrap         ok" >>"$REPORT"
else
	echo "bootstrap         FAILED" | tee -a "$REPORT"
	tail -15 "$WORK/bootstrap.log"
	exit 2
fi
echo "== build the debug binary"
if timeout 1800 cargo build >>"$WORK/build.log" 2>&1; then
	echo "build             ok" >>"$REPORT"
else
	echo "build             FAILED" | tee -a "$REPORT"
	tail -15 "$WORK/build.log"
	exit 1
fi
PB="$REPO/target/x86_64-unknown-linux-musl/debug/podbox"
[ -x "$PB" ] || { echo "binary missing after build: FAILED" | tee -a "$REPORT"; exit 1; }

STORE="$WORK/store"
mkdir -p "$STORE" || exit 2
export PODBOX_STORE="$STORE"
fail=0
step() {
	name="$1"; want="$2"; shift 2
	out="$WORK/out-$name.txt"
	timeout 120 "$@" >"$out" 2>&1
	rc=$?
	echo "step $name exit $rc"
	{
	echo ""
	echo "== $name"
	echo "command           $*"
	echo "exit              $rc"
	echo "output:"
	sed 's/^/  /' "$out"
	} >>"$REPORT"
	[ "$rc" -eq "$want" ] || { echo "step $name: wanted exit $want, got $rc" >>"$REPORT"; fail=1; }
}

step pull 0 "$PB" pull "$ALPINE"
step run-rm 0 "$PB" run --rm "$ALPINE" echo same-store-hi
step create 0 "$PB" create --name lc354 "$ALPINE" sleep 60
step start 0 "$PB" start lc354
step exec 0 "$PB" exec lc354 echo exec-hi
step ps 0 "$PB" ps
step logs 0 "$PB" logs lc354
step stop 0 "$PB" stop lc354
step wait-stopped 0 "$PB" wait lc354
step rm 0 "$PB" rm lc354
step ps-empty 0 "$PB" ps -a

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           LOOP COMPLETE"; else echo "verdict           LOOP FAILED"; fi
} >>"$REPORT"

if [ -d /out ]; then cp "$REPORT" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
