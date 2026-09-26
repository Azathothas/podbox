#!/bin/sh
# Question: does an off-arch payload run end to end where qemu-user holds it?
#
# TODO/interpose.md T-1327 (issue 58's coverage axis, issue 26's object
# series): the pair stays x86_64-only by decision, so an aarch64 payload
# must take the honest machine-mismatch decline and still run. Runs on
# a machine with binfmt_misc registered for aarch64 and a user-mode
# emulator behind it (the wsl-toolkit base), with the lane-built
# x86_64 binary granted in.
#
# Clauses:
#   qemu-1. binfmt_misc carries aarch64 with an interpreter present;
#   qemu-2. the pinned aarch64 alpine manifest pulls;
#   qemu-3. the extracted /bin/echo reads e_machine 0xb7 (foreignness proved);
#   qemu-4. `run` exits 0 with the payload output, and stderr names the
#      machine-mismatch decline (0xb7 payload against the 0x3e object)
#      rather than serving a wrong-arch object;
#   qemu-5. `system info --format '{{.EnteredRung}}'` reads the entered rung.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the base could
# not run (no binary, no pull, no binfmt, no emulator).
set -u

BIN="${PB_BIN:-/workspaces/pb365/podbox}"
WORK="${PB_WORK:-/tmp/pb367-work}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2

ALPINE_AA64_TAG='public.ecr.aws/docker/library/alpine:3.20'
ALPINE_AA64='public.ecr.aws/docker/library/alpine:3.20@sha256:45e09956dc667c5eff3583c9d94830261fb1ca0be10a0a7db36266edf5de9e1d'

fail=0
pass() { echo "  ok: $1" >>"$WORK/report"; }
miss() { echo "  FAIL: $1" >>"$WORK/report"; fail=1; }

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "id                $(id -u):$(id -g)"
echo "podbox            $($BIN version 2>/dev/null || echo MISSING)"
echo "alpineaa64        $ALPINE_AA64 (pulled by tag with --platform linux/arm64)"
echo "binfmt            $(cat /proc/sys/fs/binfmt_misc/qemu-aarch64 2>/dev/null | tr '\n' ';' || echo absent)"
} >"$WORK/report"

[ -x "$BIN" ] || { echo "binary $BIN is not executable: COULD NOT RUN" >>"$WORK/report"; exit 2; }
command -v timeout >/dev/null 2>&1 || { echo "timeout is missing: COULD NOT RUN" >>"$WORK/report"; exit 2; }
[ -e /proc/sys/fs/binfmt_misc/qemu-aarch64 ] || { echo "no aarch64 binfmt registration: COULD NOT RUN" >>"$WORK/report"; exit 2; }

STORE="$WORK/store"
mkdir -p "$STORE" || exit 2
export PODBOX_STORE="$STORE"

echo "" >>"$WORK/report"
echo "== qemu-1. binfmt carries aarch64 with an interpreter" >>"$WORK/report"
INTERP="$(grep -a "^interpreter" /proc/sys/fs/binfmt_misc/qemu-aarch64 2>/dev/null | awk '{print $2}')"
if [ -n "$INTERP" ] && [ -x "$INTERP" ]; then
	pass "aarch64 interpreter $INTERP present and executable"
else
	miss "aarch64 interpreter ${INTERP:-missing} not executable"
fi

echo "" >>"$WORK/report"
echo "== qemu-2. the aarch64 platform pulls by tag" >>"$WORK/report"
timeout 600 "$BIN" pull --platform linux/arm64 "$ALPINE_AA64_TAG" >>"$WORK/report" 2>&1 \
	|| { echo "aarch64 pull FAILED: COULD NOT RUN" >>"$WORK/report"; cat "$WORK/report"; exit 2; }
pass "aarch64 alpine pulls by tag with --platform linux/arm64"

echo "" >>"$WORK/report"
echo "== qemu-3. the extracted payload is foreign" >>"$WORK/report"
ROOTFS="$(timeout 600 "$BIN" extract --platform linux/arm64 "$ALPINE_AA64_TAG" 2>"$WORK/extract.err")"
# ^ The alpine applets are absolute links (`echo -> /bin/busybox`),
# which dangle outside the extracted tree and read as missing to
# `-f`, then resolve inside the rootfs on entry (as 366 ns-4).
if [ -n "$ROOTFS" ] && { [ -f "$ROOTFS/bin/echo" ] || [ -L "$ROOTFS/bin/echo" ]; } \
	&& command -v readelf >/dev/null 2>&1; then
	# readelf follows links, and the applet link is absolute (it
	# dangles off the rootfs), so resolve it inside the tree first.
	ECHO_LINK="$(readlink "$ROOTFS/bin/echo" 2>/dev/null || echo bin/echo)"
	case "$ECHO_LINK" in
		/*) ECHO_BIN="$ROOTFS$ECHO_LINK" ;;
		*) ECHO_BIN="$ROOTFS/bin/$ECHO_LINK" ;;
	esac
	MACH="$(readelf -h "$ECHO_BIN" 2>/dev/null | awk '/Machine:/ {print $NF}')"
	echo "payload machine     $MACH" >>"$WORK/report"
	case "$MACH" in
		*AArch64*) pass "extracted /bin/echo is AArch64" ;;
		*) miss "extracted /bin/echo reads [$MACH], not AArch64" ;;
	esac
else
	miss "no extracted rootfs with /bin/echo (or no readelf)"
fi

echo "" >>"$WORK/report"
echo "== qemu-4. the foreign payload runs with the decline named" >>"$WORK/report"
timeout 300 "$BIN" run --platform linux/arm64 --rm --name qemu367 "$ALPINE_AA64_TAG" /bin/echo foreign-hi >"$WORK/run.out" 2>"$WORK/run.err"
rc=$?
if [ "$rc" -eq 0 ] && grep -q "foreign-hi" "$WORK/run.out" \
	&& grep -q "is ELF machine 0xb7" "$WORK/run.err" \
	&& grep -q "the musl object is 0x3e" "$WORK/run.err"; then
	pass "aarch64 payload runs; stderr names the 0xb7 against 0x3e decline"
else
	miss "foreign run rc=$rc"
	cat "$WORK/run.out" "$WORK/run.err" >>"$WORK/report"
fi
timeout 120 "$BIN" rm qemu367 >>"$WORK/report" 2>&1 || true

echo "" >>"$WORK/report"
echo "== qemu-5. the entered rung reads" >>"$WORK/report"
entered="$(timeout 120 "$BIN" system info --format '{{.EnteredRung}}' 2>/dev/null)"
echo "entered rung        ${entered:-missing}" >>"$WORK/report"
if [ -n "$entered" ]; then
	pass "EnteredRung reads $entered after the foreign run"
else
	miss "EnteredRung reads nothing"
fi

echo "" >>"$WORK/report"
echo "== cleanup" >>"$WORK/report"
timeout 120 "$BIN" rmi "$ALPINE_AA64_TAG" >>"$WORK/report" 2>&1 || true
rm -rf "$STORE"

echo "" >>"$WORK/report"
if [ "$fail" -eq 0 ]; then echo "verdict           QEMU-USER AARCH64 SERVED" >>"$WORK/report"; else echo "verdict           QEMU-USER AARCH64 OPEN" >>"$WORK/report"; fi
echo "== counts: 5 clauses, fail=$fail" >>"$WORK/report"
cat "$WORK/report"
[ "$fail" -eq 0 ]
