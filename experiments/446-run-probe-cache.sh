#!/bin/sh
# Question: does `run` take the probe from the cache of T-0111, and how much
# of the probe's cost does a warm cache save a run?
#
# TODO/probe.md T-1746 (ask V-2 of podswarm, PodUtils/podcrates#124). The
# store is the store of experiment 443 (PODVM_STORE), which holds alpine
# 3.20; the experiment keeps its probe.json and puts it back at the end.
#
# Pinned inputs:
#   OCI     alpine 3.20 of the store
#   podvm   PODVM_446_BIN (default the release build of the workspace)
#   RUNS    PODVM_446_RUNS (default 10)
#
# Clauses:
#   1. with PODVM_DEBUG, a run with no cache says that the probe ran now,
#      and the next run says that it came from the cache.
#   2. a cache whose `podvm` field names another build is not served: the
#      run says that another build took the answer.
#   3. the median of RUNS runs of `run --rm IMAGE true` with a warm cache is
#      lower than the median with no cache by at least 80 percent of the
#      median of RUNS runs of `probe --json`.
#
# Exit: 0 each clause matched, 1 one or more failed, 2 the lane could not
# run.
set -u

REPO="$(cd "$(dirname "$0")/.." && pwd)"
PB="${PODVM_446_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
export PODVM_STORE="${PODVM_STORE:-$REPO/.work/qa/store}"
RUNS="${PODVM_446_RUNS:-10}"
A=mirror.gcr.io/library/alpine:3.20
C="$PODVM_STORE/probe.json"
W="$(mktemp -d)"
[ -f "$C" ] && cp "$C" "$W/saved.json"
trap 'if [ -f "$W/saved.json" ]; then cp "$W/saved.json" "$C"; else rm -f "$C"; fi; rm -rf "$W"' EXIT INT TERM

[ -x "$PB" ] || { echo "no podvm binary at $PB: COULD NOT RUN" >&2; exit 2; }
"$PB" image inspect "$A" >/dev/null 2>&1 || { echo "no alpine 3.20 in $PODVM_STORE: COULD NOT RUN" >&2; exit 2; }
"$PB" run --rm "$A" true >/dev/null 2>&1 || { echo "a run fails here: COULD NOT RUN" >&2; exit 2; }

FAIL=0
verdict() { # clause, test, what
	if eval "$2"; then echo "PASS $1: $3"; else echo "FAIL $1: $3"; FAIL=1; fi
}
ms() { echo $(( $(date +%s%N) / 1000000 )); }
median() { tr ' ' '\n' | grep . | sort -n | awk '{a[NR]=$1} END {print (NR ? a[int((NR+1)/2)] : "-")}'; }

rm -f "$C"
cold=$(PODVM_DEBUG=1 "$PB" run --rm "$A" true 2>&1 | grep 'the probe')
warm=$(PODVM_DEBUG=1 "$PB" run --rm "$A" true 2>&1 | grep 'the probe')
echo "  1| $cold"
echo "  1| $warm"
verdict 1 'case "$cold" in *"ran now"*) ;; *) false ;; esac && case "$warm" in *"came from the cache"*) ;; *) false ;; esac' \
	"no cache: the probe runs; then the run takes the cache"

sed 's/"podvm":"[^"]*"/"podvm":"0.0.0-other"/' "$C" >"$W/other.json" && cp "$W/other.json" "$C"
other=$(PODVM_DEBUG=1 "$PB" run --rm "$A" true 2>&1 | grep 'the probe')
echo "  2| $other"
verdict 2 'case "$other" in *"another build of podvm took the cached answer (0.0.0-other)"*) ;; *) false ;; esac' \
	"a cache of another build is not served"

c=""; w=""; p=""
i=0
while [ $i -lt "$RUNS" ]; do
	rm -f "$C"
	t0=$(ms); "$PB" run --rm "$A" true >/dev/null 2>&1; t1=$(ms); c="$c $((t1 - t0))"
	t0=$(ms); "$PB" run --rm "$A" true >/dev/null 2>&1; t1=$(ms); w="$w $((t1 - t0))"
	t0=$(ms); "$PB" probe --json >/dev/null 2>&1; t1=$(ms); p="$p $((t1 - t0))"
	i=$((i + 1))
done
mc=$(echo "$c" | median); mw=$(echo "$w" | median); mp=$(echo "$p" | median)
echo "  3| no cache:$c (median $mc ms)"
echo "  3| warm:$w (median $mw ms)"
echo "  3| probe --json:$p (median $mp ms)"
verdict 3 '[ $((mc - mw)) -ge $((mp * 8 / 10)) ]' \
	"a warm cache saves $((mc - mw)) ms of a run; the probe costs $mp ms (80 percent: $((mp * 8 / 10)) ms)"

if [ $FAIL = 0 ]; then echo "RUN TAKES THE PROBE FROM THE CACHE"; fi
exit $FAIL
