#!/bin/sh
# Question: does a pull refuse past the ceiling instead of dying past it?
#
# TODO/image.md T-1342 (issue 65, second half: the `windows fetch` half
# shipped with the T-1112 landing). Two live clauses on one image with
# two file-size ceilings, set with `ulimit -f` (512-byte blocks) around
# the pull alone, after the build: a 10 MiB ceiling refuses the debian
# blobs up front naming the object bytes, the ceiling and the room, with
# no partial left; an unlimited ceiling pulls the same bytes clean.
# The mid-stream cap against a lying origin is unit-pinned in
# `crates/podbox-image/src/registry.rs` (an infinite reader served
# over loopback), because no honest registry lies on demand.
#
# Pinned inputs: the debian digest 360 pulls, the 10 MiB ceiling.
#
#   sh experiments/372-pull-ceiling.sh
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no toolchain, no build, no pull).
set -u

# The Windows wrapper runs the job at /in/job.sh with /work as the
# working directory. Take the checkout from that directory, not $0.
REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep372-work"
rm -rf "$WORK"; mkdir -p "$WORK/out" || exit 2
REPORT="$WORK/out/pull-ceiling-372.txt"

DEBIAN='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
# 10 MiB in 512-byte blocks: under every debian layer, over the manifests.
CEILING_BLOCKS=20480

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "rustc             $(rustc --version 2>&1)"
echo "cargo             $(cargo --version 2>&1)"
echo "image input       $DEBIAN"
echo "ceiling blocks    $CEILING_BLOCKS (10 MiB)"
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
fail=0

echo "" >>"$REPORT"
echo "== clause 1: a 10 MiB ceiling refuses the debian blobs up front" >>"$REPORT"
# The limit applies inside the subshell alone: the build above already
# wrote past it, and the report stays outside it. Each stage reports its
# own code so a failure names the stage, not just the pull.
(
	ulimit -S -f "$CEILING_BLOCKS" || { echo "ULIMIT_SOFT rc=$?"; exit 11; }
	echo "fsize soft        $(ulimit -Sf)"
	echo "fsize hard        $(ulimit -Hf)"
	timeout 300 "$PB" pull "$DEBIAN" >"$WORK/pull-small.txt" 2>"$WORK/pull-small.err"
	echo "PULL rc=$?"
) >"$WORK/clause1.txt" 2>&1
sed 's/^/  /' "$WORK/clause1.txt" >>"$REPORT"
RC=$(grep -o "PULL rc=[0-9]*" "$WORK/clause1.txt" | awk -F= '{print $2}')
RC=${RC:-missing}
echo "exit              $RC" >>"$REPORT"
sed -n '1,4p' "$WORK/pull-small.err" | sed 's/^/  /' >>"$REPORT"
[ "$RC" -eq 125 ] || { echo "FAIL: refusal did not exit 125" >>"$REPORT"; fail=1; }
grep -q "declares .* byte(s)" "$WORK/pull-small.err" || { echo "FAIL: refusal did not name the object bytes" >>"$REPORT"; fail=1; }
grep -q "file-size ceiling" "$WORK/pull-small.err" || { echo "FAIL: refusal did not name the ceiling" >>"$REPORT"; fail=1; }
grep -q "free at" "$WORK/pull-small.err" || { echo "FAIL: refusal did not name the room" >>"$REPORT"; fail=1; }
if find "$STORE" -name "*.partial" | grep -q .; then
	echo "FAIL: a partial survived the refusal" >>"$REPORT"; fail=1
else
	echo "partials           none" >>"$REPORT"
fi
echo "clause 1          $([ "$fail" -eq 0 ] && echo HOLDS || echo OPEN)" >>"$REPORT"

echo "" >>"$REPORT"
echo "== clause 2: the same bytes pull clean with no ceiling" >>"$REPORT"
if timeout 600 "$PB" pull "$DEBIAN" >"$WORK/pull-open.txt" 2>"$WORK/pull-open.err"; then
	echo "exit              0" >>"$REPORT"
	echo "clause 2          HOLDS" >>"$REPORT"
else
	echo "exit              $?" >>"$REPORT"
	echo "FAIL: the control pull did not land" >>"$REPORT"
	sed -n '1,4p' "$WORK/pull-open.err" | sed 's/^/  /' >>"$REPORT"
	fail=1
	echo "clause 2          OPEN" >>"$REPORT"
fi

echo "" >>"$REPORT"
if [ "$fail" -eq 0 ]; then echo "verdict           PULL CEILING HOLDS" >>"$REPORT"; else echo "verdict           PULL CEILING OPEN" >>"$REPORT"; fi

if [ -d /out ]; then cp "$WORK/out/pull-ceiling-372.txt" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
