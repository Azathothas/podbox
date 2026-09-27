#!/bin/sh
# Question: why do early payload runs cost ~2 s and late ones ~0.08 s?
#
# TODO/gate.md T-1340. `experiments/360-perf-harness.sh` records three
# runs named by order on one store and one image (`run.first` ~1.8 s,
# `run.repeat` ~1.9 s, `run.late` ~0.08 s) with the cause unisolated.
# Three candidates before testing:
#   A. `run --rm` deletes the unreferenced rootfs (the `--rm` arm in
#      `crates/podbox-cli/src/run.rs`), so each keeper-less run re-pays
#      full extraction plus first-time fixups. Refutes to: every
#      keeper-less run stays slow, with the rootfs absent afterwards.
#   B. Kernel page-cache warming of the probe storm (one disposable
#      child per probe row) and completion reads. Refutes to: with the
#      rootfs pinned by a keeper, runs decay from slow to fast.
#   C. One-time fixup steps (openssl rehash over the CA dir, CA bundle
#      append) that skip once done (`if !linked`, HOST_CA_MARKER).
#      Refutes to: only the first kept run pays them; later kept runs
#      do not, even beside a slow keeper-less run.
# (A fourth, per-run HTTPS reachability cost, is bounded already:
# `run.late` at ~0.08 s caps every flat per-run cost below 0.1 s warm.)
#
# This drive times three keeper-less runs, one create, three kept runs,
# then rm plus two runs, and records rootfs presence after each step.
# Pinned input: the debian digest 360 uses.
#
#   sh experiments/368-run-decay.sh
#
# Exit: 0 every prediction held, 1 the drive ran and a prediction
# failed, 2 the lane could not run (no toolchain, no binary, no pull).
set -u

# The job travels inside the workspace as /work/.podbox-job.sh, so $0
# names the staging path, not experiments/. Take the checkout from the
# working directory instead (experiments/353-open-issue-triage.sh).
REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep368-work"
rm -rf "$WORK"; mkdir -p "$WORK/out" || exit 2
REPORT="$WORK/out/run-decay-368.txt"

DEBIAN='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "rustc             $(rustc --version 2>&1)"
echo "cargo             $(cargo --version 2>&1)"
echo "image input       $DEBIAN"
} >"$REPORT"

command -v cargo >/dev/null 2>&1 || { echo "cargo missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
echo "== bootstrap the lane toolchain (zig, cc, musl target)"
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
echo "podbox            $($PB version 2>&1)" >>"$REPORT"

STORE="$WORK/store"
mkdir -p "$STORE" || exit 2
export PODBOX_STORE="$STORE"

# rootfs_present: the store holds an extracted tree (run.rs `--rm` arm
# removes it where no container record references the digest).
rootfs_present() {
	[ -n "$(ls -A "$STORE/rootfs" 2>/dev/null)" ]
}

# t NAME -- CMD...: wall seconds of one run with its exit, payload
# check, rootfs presence after, and whether stderr kept the rootfs.
# The wall lands in $WORK/wall-NAME for the verdict comparisons.
t() {
	name="$1"; shift
	t0=$(date +%s%N)
	timeout 120 "$@" >"$WORK/out-$name.txt" 2>"$WORK/err-$name.txt"
	rc=$?
	t1=$(date +%s%N)
	sec=$(awk -v a="$t0" -v b="$t1" 'BEGIN { printf "%.3f", (b-a)/1e9 }')
	echo "$sec" >"$WORK/wall-$name"
	if rootfs_present; then tree="present"; else tree="absent"; fi
	echo "$tree" >"$WORK/tree-$name"
	if grep -aq "keeps the rootfs" "$WORK/err-$name.txt"; then kept="kept-msg"; else kept="no-kept-msg"; fi
	echo "$kept" >"$WORK/kept-$name"
	{
	echo ""
	echo "== $name"
	echo "command           $*"
	echo "exit              $rc"
	echo "wall              ${sec}s"
	echo "payload           $(head -1 "$WORK/out-$name.txt" 2>/dev/null)"
	echo "rootfs-after      $tree"
	echo "keeper-message    $kept"
	} >>"$REPORT"
}

echo "" >>"$REPORT"
echo "== cold pull and extract" >>"$REPORT"
t pull "$PB" pull "$DEBIAN"
t extract "$PB" extract "$DEBIAN"

echo "" >>"$REPORT"
echo "== phase A: three keeper-less runs (candidate A: all stay slow)" >>"$REPORT"
t a1 "$PB" run --rm "$DEBIAN" /bin/echo decay-hi
t a2 "$PB" run --rm "$DEBIAN" /bin/echo decay-hi
t a3 "$PB" run --rm "$DEBIAN" /bin/echo decay-hi

echo "" >>"$REPORT"
echo "== phase B: keeper pins the rootfs, then three kept runs" >>"$REPORT"
t create "$PB" create --name keeper368 "$DEBIAN" /bin/sleep 30
t b1 "$PB" run --rm "$DEBIAN" /bin/echo decay-hi
t b2 "$PB" run --rm "$DEBIAN" /bin/echo decay-hi
t b3 "$PB" run --rm "$DEBIAN" /bin/echo decay-hi

echo "" >>"$REPORT"
echo "== phase C: rm the keeper, one warm run, one re-extract run" >>"$REPORT"
t rmkeeper "$PB" rm keeper368
t c1 "$PB" run --rm "$DEBIAN" /bin/echo decay-hi
t c2 "$PB" run --rm "$DEBIAN" /bin/echo decay-hi

echo "" >>"$REPORT"
echo "== control: probe cost with no rootfs work" >>"$REPORT"
t probe "$PB" probe

# Verdict: each prediction names its refutation.
echo "" >>"$REPORT"
echo "== verdict against the three candidates" >>"$REPORT"
bad=0
check() {
	desc="$1"; shift
	if "$@" >/dev/null 2>&1; then
		echo "  hold: $desc" >>"$REPORT"
	else
		echo "  FAIL: $desc" >>"$REPORT"
		bad=$((bad + 1))
	fi
}
above() { awk -v v="$1" -v t="$2" 'BEGIN { exit !(v > t) }'; }
below() { awk -v v="$1" -v t="$2" 'BEGIN { exit !(v < t) }'; }
A1=$(cat "$WORK/wall-a1"); A3=$(cat "$WORK/wall-a3")
B1=$(cat "$WORK/wall-b1"); B3=$(cat "$WORK/wall-b3")
C1=$(cat "$WORK/wall-c1"); C2=$(cat "$WORK/wall-c2")
echo "walls: a1=$A1 a3=$A3 b1=$B1 b3=$B3 c1=$C1 c2=$C2" >>"$REPORT"
check "keeper-less runs stay slow (a1 above 1.0 s)" above "$A1" 1.0
check "keeper-less runs stay slow (a3 above 1.0 s)" above "$A3" 1.0
check "kept runs decay (b3 below 0.5 s)" below "$B3" 0.5
check "kept runs decay (b3 below b1)" \
	awk -v v="$B3" -v u="$B1" 'BEGIN { exit !(v < u) }'
check "post-rm warm run is fast (c1 below 0.5 s)" below "$C1" 0.5
check "post-rm second run re-pays extraction (c2 above 1.0 s)" above "$C2" 1.0
check "rootfs absent after keeper-less a3" \
	test "$(cat "$WORK/tree-a3")" = absent
check "rootfs present after kept b3" \
	test "$(cat "$WORK/tree-b3")" = present
check "keeper message on kept b3" \
	test "$(cat "$WORK/kept-b3")" = kept-msg
check "rootfs present after rm keeper" \
	test "$(cat "$WORK/tree-rmkeeper")" = present
check "rootfs absent after post-rm c1" \
	test "$(cat "$WORK/tree-c1")" = absent
echo "" >>"$REPORT"
if [ "$bad" -eq 0 ]; then echo "verdict           DECAY ISOLATED" >>"$REPORT"; else echo "verdict           DECAY OPEN ($bad failed)" >>"$REPORT"; fi

if [ -d /out ]; then cp "$WORK/out/run-decay-368.txt" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$bad" -eq 0 ]
