#!/bin/sh
# Question: does the namespace rung change nothing where its legs fail?
#
# TODO/enter.md T-1339 (issue 59: `run` never entered `namespace`). This
# is the denied-host half: the lane refuses unshare and mounts, so the
# probe selects chroot and every run must enter exactly as before, with
# the entered rung reading chroot. The capable-host half
# (experiments/366-namespace-base.sh) runs where unshare holds.
#
# Clauses:
#   ns-lane-1. `probe` selects below namespace on the lane;
#   ns-lane-2. a run enters chroot: the banner reads mode=chroot and
#      names no namespace fallback;
#   ns-lane-3. `system info --format '{{.EnteredRung}}'` reads chroot.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no binary, no pull).
set -u

# The job travels inside the workspace as /work/.podbox-job.sh, so $0
# names the staging path, not experiments/. Take the checkout from the
# working directory instead (experiments/353-open-issue-triage.sh).
REPO="$(pwd)"
cd "$REPO" || exit 2
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/namespace.txt"
WORK="$REPO/experiments/.sweep365-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2

DEBIAN='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'

fail=0
pass() { echo "  ok: $1" >>"$WORK/report"; }
miss() { echo "  FAIL: $1" >>"$WORK/report"; fail=1; }

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "podbox            $($BIN version 2>/dev/null || echo MISSING)"
echo "debian            $DEBIAN"
} >"$WORK/report"

[ -x "$BIN" ] || { echo "binary $BIN is not executable: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }

STORE="$WORK/store"
mkdir -p "$STORE" || exit 2
export PODBOX_STORE="$STORE"

echo "" >>"$WORK/report"
echo "== pulls" >>"$WORK/report"
timeout 600 "$BIN" pull "$DEBIAN" >>"$WORK/report" 2>&1 \
	|| { echo "debian pull FAILED: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }

echo "" >>"$WORK/report"
echo "== ns-lane-1. the lane selects below namespace" >>"$WORK/report"
lane_rung="$(timeout 120 "$BIN" probe 2>/dev/null | head -1)"
if [ -n "$lane_rung" ] && [ "$lane_rung" != "namespace" ]; then
	pass "probe selects $lane_rung where unshare is refused"
else
	miss "probe selects ${lane_rung:-nothing} on the lane"
	timeout 120 "$BIN" probe >>"$WORK/report" 2>&1
fi

echo "" >>"$WORK/report"
echo "== ns-lane-2. a run enters chroot with no fallback line" >>"$WORK/report"
timeout 120 "$BIN" run --rm --name ns365 "$DEBIAN" /bin/echo lane-hi >"$WORK/run.out" 2>"$WORK/run.err"
rc=$?
if [ "$rc" -eq 0 ] && grep -q "lane-hi" "$WORK/run.out" \
	&& grep -q "mode=chroot" "$WORK/run.err" \
	&& ! grep -q "entered chroot instead" "$WORK/run.err"; then
	pass "run enters chroot, banner mode=chroot, no fallback named"
else
	miss "lane run rc=$rc"
	cat "$WORK/run.out" "$WORK/run.err" >>"$WORK/report"
fi
timeout 120 "$BIN" rm ns365 >>"$WORK/report" 2>&1 || true

echo "" >>"$WORK/report"
echo "== ns-lane-3. EnteredRung reads chroot" >>"$WORK/report"
entered="$(timeout 120 "$BIN" system info --format '{{.EnteredRung}}' 2>/dev/null)"
if [ "$entered" = "chroot" ]; then
	pass "EnteredRung reads chroot on the lane"
else
	miss "EnteredRung reads ${entered:-missing}"
fi

echo "" >>"$WORK/report"
if [ "$fail" -eq 0 ]; then echo "verdict           NAMESPACE LANE SERVED" >>"$WORK/report"; else echo "verdict           NAMESPACE LANE OPEN" >>"$WORK/report"; fi
echo "== counts: 3 lane clauses, fail=$fail" >>"$WORK/report"
cp "$WORK/report" "$OUT"
if [ -d /out ]; then cp "$OUT" /out/ 2>/dev/null || true; fi
[ "$fail" -eq 0 ]
