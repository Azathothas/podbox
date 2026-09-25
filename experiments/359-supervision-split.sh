#!/bin/sh
# Question: where syscall mediation is refused but pidfd and waitid hold,
# does supervision stay available with the banner stating mediation is
# off, and does the lifecycle loop still pass twenty of twenty?
#
# TODO/supervise.md T-0606 (fallback supervision).
#
# Clauses:
#   1. probe Prove (unchanged): tiers.supervise carries three legs, and
#      the refusal is null exactly where every leg is ok.
#   2. supervision available on this lane: tiers.supervision carries
#      two legs with a null refusal.
#   3. the two supervision rows ran: `probe --rows` names
#      pidfd_open(own pid) and waitid(P_PIDFD, child) as ok.
#   4. the banner stays silent where mediation holds: a run carries no
#      "no syscall mediation" line on this lane.
#   5. the lifecycle loop still passes twenty of twenty.
#
# The denied-notify shape (mediation refused beside held supervision,
# banner line on) is pinned by unit tests: the lane permits all three
# notify legs, so no lane run can show the fallback.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no toolchain, no build, no pull, no jq).
set -u

# The job travels as /work/.podbox-job.sh: take the checkout from the
# working directory, never from $0 (see 353 for the measurement).
REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep359-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-359.txt"

ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "input             $ALPINE"
} >"$REPORT"

command -v cargo >/dev/null 2>&1 || { echo "cargo missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
echo "== bootstrap the lane toolchain"
if timeout 1200 ./scripts/common/bootstrap-env.sh rust cc zig tools >>"$WORK/bootstrap.log" 2>&1; then
	echo "bootstrap         ok" >>"$REPORT"
else
	echo "bootstrap         FAILED" | tee -a "$REPORT"
	tail -15 "$WORK/bootstrap.log"
	exit 2
fi
# jq rides the bootstrap (its installed-tools line names it); the probe
# document clauses below need it, so its absence here is the lane, not
# the binary.
command -v jq >/dev/null 2>&1 || { echo "jq missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
echo "== interposer objects"
if timeout 1200 ./scripts/build-interpose.sh >>"$WORK/interpose.log" 2>&1; then
	echo "interpose         ok" >>"$REPORT"
else
	echo "interpose         FAILED" | tee -a "$REPORT"
	tail -15 "$WORK/interpose.log"
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
echo "podbox            $("$PB" version)" >>"$REPORT"

STORE="$WORK/store"
mkdir -p "$STORE" || exit 2
export PODBOX_STORE="$STORE"
fail=0

# Clause 1: the entry's Prove, unchanged by the split.
if timeout 120 "$PB" probe --json >"$WORK/probe.json" 2>"$WORK/probe-err.txt"; then
	echo "probe json        ok" >>"$REPORT"
else
	echo "probe --json FAILED" | tee -a "$REPORT"
	exit 1
fi
if jq -e '.tiers.supervise | (.legs | length == 3) and (([.legs[] | select(.verdict != "ok")] | length == 0) == (.refusal == null))' "$WORK/probe.json" >/dev/null; then
	echo "clause 1          mediation Prove holds" >>"$REPORT"
else
	echo "clause 1          MEDIATION PROVE FAILED" >>"$REPORT"; fail=1
fi

# Clause 2: supervision available where its pair holds.
if jq -e '.tiers.supervision | (.legs | length == 2) and (.refusal == null)' "$WORK/probe.json" >/dev/null; then
	echo "clause 2          supervision available" >>"$REPORT"
else
	echo "clause 2          SUPERVISION NOT AVAILABLE" >>"$REPORT"; fail=1
fi
jq -c '.tiers.supervision.legs[] | {name, verdict}' "$WORK/probe.json" >>"$REPORT"

# Clause 3: the two rows ran and answered ok.
if timeout 120 "$PB" probe --rows >"$WORK/rows.txt" 2>&1; then
	echo "probe rows        ok" >>"$REPORT"
else
	echo "probe --rows FAILED" | tee -a "$REPORT"
	exit 1
fi
for row in "pidfd_open(own pid)" "waitid(P_PIDFD, child)"; do
	if grep -q "$row" "$WORK/rows.txt"; then
		echo "clause 3          row present: $row" >>"$REPORT"
	else
		echo "clause 3          ROW MISSING: $row" >>"$REPORT"; fail=1
	fi
done

# Clause 4: the banner stays silent where mediation holds. A mediated
# run must never read as a degraded one.
if ! timeout 600 "$PB" pull "$ALPINE" >>"$REPORT" 2>&1; then
	echo "pull $ALPINE FAILED: COULD NOT RUN" | tee -a "$REPORT"
	exit 2
fi
if timeout 120 "$PB" run --rm "$ALPINE" true >"$WORK/run-out.txt" 2>"$WORK/run-err.txt"; then
	echo "run               ok" >>"$REPORT"
else
	echo "run $ALPINE FAILED" | tee -a "$REPORT"
	exit 1
fi
if grep -q "no syscall mediation" "$WORK/run-err.txt"; then
	echo "clause 4          BANNER STATES A FALLBACK THAT IS NOT RUNNING" >>"$REPORT"; fail=1
else
	echo "clause 4          banner silent where mediation holds" >>"$REPORT"
fi

# Clause 5: the lifecycle loop still passes twenty of twenty, on the
# just-built binary over the just-pulled image (230 pulls its own).
if PODBOX_BIN="$PB" PODBOX_RUN_IMAGE="$ALPINE" timeout 1500 ./experiments/230-lifecycle-loop.sh 20 >>"$WORK/loop.log" 2>&1; then
	echo "clause 5          loop 20 of 20" >>"$REPORT"
else
	echo "clause 5          LOOP FAILED" | tee -a "$REPORT"
	tail -10 "$WORK/loop.log"
	fail=1
fi

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           SUPERVISION SPLIT HOLDS"; else echo "verdict           SUPERVISION SPLIT OPEN"; fi
} >>"$REPORT"

cp "$REPORT" "$REPO/experiments/results/supervision-split.txt"
if [ -d /out ]; then cp "$REPORT" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
