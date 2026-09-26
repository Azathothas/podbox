#!/bin/sh
# Question: does the namespace rung isolate a mount the host cannot see?
#
# TODO/enter.md T-1339 (issue 59). The capable-host half: runs on a
# machine where unshare and mounts hold (the wsl-toolkit base, as
# root), with the lane-built binary granted in. The denied-host half
# (experiments/365-namespace.sh) runs in the lane.
#
# Clauses:
#   ns-1. `probe` selects namespace;
#   ns-2. a run enters it: banner mode=namespace reading mount-only
#      with host-shared network and pids and the never-claim, no
#      fallback line, and EnteredRung namespace;
#   ns-3. a file the payload writes to /tmp is visible to the payload
#      and absent from the host rootfs afterwards, with no leaked mount;
#   ns-4. an image without /tmp falls back: the banner names the
#      fallback and chroot, the payload still runs, exit 0;
#   ns-5. a detached container starts through the same gate and its
#      logs carry the payload output.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the base could
# not run (no binary, no pull, no unshare).
set -u

BIN="${PB_BIN:-/workspaces/pb365/podbox}"
WORK="${PB_WORK:-/tmp/pb365-work}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2

ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'

fail=0
pass() { echo "  ok: $1" >>"$WORK/report"; }
miss() { echo "  FAIL: $1" >>"$WORK/report"; fail=1; }

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "id                $(id -u):$(id -g)"
echo "podbox            $($BIN version 2>/dev/null || echo MISSING)"
echo "alpine            $ALPINE"
} >"$WORK/report"

[ -x "$BIN" ] || { echo "binary $BIN is not executable: COULD NOT RUN" >>"$WORK/report"; exit 2; }
command -v timeout >/dev/null 2>&1 || { echo "timeout is missing: COULD NOT RUN" >>"$WORK/report"; exit 2; }
unshare -m true >/dev/null 2>&1 || { echo "unshare is refused: COULD NOT RUN" >>"$WORK/report"; exit 2; }

STORE="$WORK/store"
mkdir -p "$STORE" || exit 2
export PODBOX_STORE="$STORE"

echo "" >>"$WORK/report"
echo "== pulls" >>"$WORK/report"
timeout 600 "$BIN" pull "$ALPINE" >>"$WORK/report" 2>&1 \
	|| { echo "alpine pull FAILED: COULD NOT RUN" >>"$WORK/report"; exit 2; }

echo "" >>"$WORK/report"
echo "== ns-1. the capable host selects namespace" >>"$WORK/report"
if timeout 120 "$BIN" probe 2>/dev/null | grep -q "^namespace"; then
	pass "probe selects namespace where unshare and mounts hold"
else
	miss "probe selects something else on a capable host"
	timeout 120 "$BIN" probe >>"$WORK/report" 2>&1
fi

echo "" >>"$WORK/report"
echo "== ns-2. a run enters the namespace rung" >>"$WORK/report"
timeout 120 "$BIN" run --rm --name ns365 "$ALPINE" /bin/echo base-hi >"$WORK/run.out" 2>"$WORK/run.err"
rc=$?
if [ "$rc" -eq 0 ] && grep -q "base-hi" "$WORK/run.out" \
	&& grep -q "mode=namespace" "$WORK/run.err" \
	&& grep -q "namespaces: mount-only" "$WORK/run.err" \
	&& grep -q "network: host-shared" "$WORK/run.err" \
	&& grep -q "this mode does NOT provide:" "$WORK/run.err" \
	&& ! grep -q "entered chroot instead" "$WORK/run.err"; then
	pass "run enters namespace with the honest banner and no fallback"
else
	miss "namespace run rc=$rc"
	cat "$WORK/run.out" "$WORK/run.err" >>"$WORK/report"
fi
timeout 120 "$BIN" rm ns365 >>"$WORK/report" 2>&1 || true
entered="$(timeout 120 "$BIN" system info --format '{{.EnteredRung}}' 2>/dev/null)"
if [ "$entered" = "namespace" ]; then
	pass "EnteredRung reads namespace after the run"
else
	miss "EnteredRung reads ${entered:-missing}"
fi

echo "" >>"$WORK/report"
echo "== ns-3. the payload's /tmp is invisible from the host" >>"$WORK/report"
timeout 120 "$BIN" run --name ns365b "$ALPINE" /bin/sh -c 'echo mark365 > /tmp/ns365mark; cat /tmp/ns365mark' >"$WORK/mark.out" 2>"$WORK/mark.err"
rc=$?
ROOTFS="$(timeout 120 "$BIN" inspect --format '{{.RootfsPath}}' "$ALPINE" 2>/dev/null)"
if [ "$rc" -eq 0 ] && grep -q "mark365" "$WORK/mark.out" && [ -n "$ROOTFS" ]; then
	pass "payload wrote and read its own /tmp file"
else
	miss "mark run rc=$rc rootfs=${ROOTFS:-missing}"
fi
if [ -n "$ROOTFS" ] && [ -d "$ROOTFS/tmp" ] && [ ! -e "$ROOTFS/tmp/ns365mark" ]; then
	pass "the host rootfs holds no ns365mark after the run"
else
	miss "ns365mark leaked into the host rootfs (or the rootfs is absent)"
	ls "$ROOTFS/tmp/" >>"$WORK/report" 2>&1
fi
if grep -Fq "$ROOTFS/tmp" /proc/self/mounts 2>/dev/null; then
	miss "a payload mount leaked into the host mount table"
	grep -F "$ROOTFS/tmp" /proc/self/mounts >>"$WORK/report" 2>&1
else
	pass "no payload mount in the host mount table"
fi
timeout 120 "$BIN" rm ns365b >>"$WORK/report" 2>&1 || true

echo "" >>"$WORK/report"
echo "== ns-4. an image without /tmp falls back to chroot" >>"$WORK/report"
NOTMP="$WORK/notmp"
rm -rf "$NOTMP"; mkdir -p "$NOTMP" || exit 2
timeout 120 "$BIN" extract "$ALPINE" >>"$WORK/report" 2>&1 \
	|| { echo "extract FAILED: COULD NOT RUN" >>"$WORK/report"; exit 2; }
if [ -z "$ROOTFS" ] || [ ! -d "$ROOTFS/bin" ]; then
	miss "no extracted rootfs to copy for the no-tmp image"
else
	cp -a --no-preserve=ownership "$ROOTFS/bin" "$ROOTFS/lib" "$NOTMP/" >>"$WORK/report" 2>&1 \
		|| miss "copy of bin and lib for the no-tmp image failed"
	rm -rf "$NOTMP/tmp"
	ls -la "$NOTMP/bin" 2>/dev/null | head -5 >>"$WORK/report" 2>&1
fi
if [ -f "$NOTMP/bin/sh" ] || [ -L "$NOTMP/bin/sh" ]; then
# ^ Either shape passes: the alpine applets are absolute links
# (`sh -> /bin/busybox`), which dangle outside the staged tree and
# read as missing to `-f`, then resolve inside the rootfs on import.
tar -cf "$WORK/notmp.tar" -C "$NOTMP" bin lib >>"$WORK/report" 2>&1 \
	|| miss "tar of the no-tmp image failed"
fi
if { [ -f "$NOTMP/bin/sh" ] || [ -L "$NOTMP/bin/sh" ]; } && [ -f "$WORK/notmp.tar" ]; then
timeout 120 "$BIN" import "$WORK/notmp.tar" notmp365:test >>"$WORK/report" 2>&1 \
	|| miss "no-tmp import failed"
timeout 120 "$BIN" run --rm --name ns365c notmp365:test /bin/sh -c 'echo fallback-hi' >"$WORK/fb.out" 2>"$WORK/fb.err"
rc=$?
if [ "$rc" -eq 0 ] && grep -q "fallback-hi" "$WORK/fb.out" \
	&& grep -q "entered chroot instead" "$WORK/fb.err" \
	&& grep -q "no /tmp mount point" "$WORK/fb.err"; then
	pass "no-tmp image falls back to chroot with the fallback named"
else
	miss "fallback run rc=$rc"
	cat "$WORK/fb.out" "$WORK/fb.err" >>"$WORK/report"
fi
else
	miss "no-tmp image holds no shell or its tar is missing; import and run skipped"
fi
timeout 120 "$BIN" rm ns365c >>"$WORK/report" 2>&1 || true
timeout 120 "$BIN" rmi notmp365:test >>"$WORK/report" 2>&1 || true

echo "" >>"$WORK/report"
echo "== ns-5. a detached container starts through the same gate" >>"$WORK/report"
timeout 120 "$BIN" create --name ns365d "$ALPINE" /bin/echo detached-hi >>"$WORK/report" 2>&1 \
	|| miss "create failed"
timeout 120 "$BIN" start ns365d >>"$WORK/report" 2>&1 || miss "start failed"
timeout 120 "$BIN" wait ns365d >>"$WORK/report" 2>&1 || miss "wait failed"
if timeout 120 "$BIN" logs ns365d 2>/dev/null | grep -q "detached-hi"; then
	pass "detached container ran and its logs carry the output"
else
	miss "detached logs carry no output"
	timeout 120 "$BIN" logs ns365d >>"$WORK/report" 2>&1
fi
timeout 120 "$BIN" rm ns365d >>"$WORK/report" 2>&1 || true

echo "" >>"$WORK/report"
echo "== cleanup" >>"$WORK/report"
timeout 120 "$BIN" rmi "$ALPINE" >>"$WORK/report" 2>&1 || true
rm -rf "$STORE"

echo "" >>"$WORK/report"
if [ "$fail" -eq 0 ]; then echo "verdict           NAMESPACE BASE SERVED" >>"$WORK/report"; else echo "verdict           NAMESPACE BASE OPEN" >>"$WORK/report"; fi
echo "== counts: 5 base clauses, fail=$fail" >>"$WORK/report"
cat "$WORK/report"
[ "$fail" -eq 0 ]
