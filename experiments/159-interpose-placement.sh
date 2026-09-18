#!/usr/bin/env bash
# Question: does the interpose tier reach a dynamic payload and decline a
# static one by name?
#
# TODO/interpose.md T-0702 (placement) and T-0706 (classification).
#
# Three clauses:
#   A. T-0702's Prove, adapted: that entry greps /proc/self/environ, and
#      the chroot has no /proc. The object is asserted as a file and as the
#      LD_PRELOAD the payload's own `env` reports instead.
#   B. T-0706 row 2, adapted: that entry's Prove stages its static binary
#      with `run -v`, and `run -v` is a refused None row (parity table), so
#      the binary is staged with `extract` plus a host copy plus `run`
#      instead. The decline line must name the static payload, and the
#      payload must still run: the decline is the tier's, never the run's.
#   C. Control: a dynamic payload is NOT declined and IS preloaded.
#
# Inputs pinned: public.ecr.aws/docker/library/alpine:3.20 at the digest
# DISTRO_ROWS_M5 records, and the podbox binary under test (which is itself
# the static payload clause B drives: a musl static binary with no
# PT_INTERP).
#
# Exit: 0 every clause held, 1 one did not, 2 could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
IMAGE="public.ecr.aws/docker/library/alpine@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc"
OUT="$REPO/experiments/results/interpose-placement.txt"
WORK="$(mktemp -d)"

cleanup() {
	rm -rf "$WORK"
}
trap cleanup EXIT INT TERM

[ -x "$BIN" ] || {
	echo "SKIP: $BIN is not an executable. Build it:" >&2
	echo "      cargo build --release --target x86_64-unknown-linux-musl" >&2
	exit 2
}

export PODBOX_STORE="$WORK/store"
unset PODBOX_DEFAULT_PLATFORM DOCKER_DEFAULT_PLATFORM

fail=0
{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -r)"
	printf 'host arch         %s\n' "$(uname -m)"
	printf 'podbox            %s\n' "$("$BIN" version 2>/dev/null || echo unversioned)"
	printf 'image             %s\n' "$IMAGE"
	printf 'payload B         %s (the binary under test)\n' "$BIN"
	file "$BIN" | sed 's/^/payload B file      /'

	echo "== A. T-0702: the object is inside the rootfs and in environ"
	# ⛔ No /proc read: the chroot has no /proc, so /proc/self/environ does
	# not exist there. The environ is read through `env` instead.
	if "$BIN" run --rm "$IMAGE" sh -c 'test -f /.podbox/interpose.so && env | grep -q "^LD_PRELOAD=/\.podbox/interpose\.so"' 2>"$WORK/a.err"; then
		echo "ok   A: /.podbox/interpose.so is a file and names LD_PRELOAD"
	else
		echo "FAIL A: the object is not placed or not preloaded"
		sed 's/^/  stderr: /' "$WORK/a.err"
		fail=1
	fi

	echo "== B. T-0706 row 2: a static payload is declined by name, and runs"
	"$BIN" extract "$IMAGE" >/dev/null 2>"$WORK/extract.err" || {
		echo "FAIL B: extract failed"
		sed 's/^/  /' "$WORK/extract.err"
		fail=1
	}
	ROOT="$("$BIN" inspect --format '{{.RootfsPath}}' "$IMAGE" 2>"$WORK/inspect.err")" || {
		echo "FAIL B: inspect --format {{.RootfsPath}} failed"
		sed 's/^/  /' "$WORK/inspect.err"
		fail=1
	}
	cp "$BIN" "$ROOT/podbox" || {
		echo "FAIL B: staging the static payload failed"
		fail=1
	}
	out="$("$BIN" run --rm "$IMAGE" /podbox --version 2>&1)"
	rc=$?
	printf '%s\n' "$out" | sed 's/^/  run output: /'
	if printf '%s\n' "$out" | grep -q 'interpose: declined'; then
		echo "ok   B: the decline names the static payload"
	else
		echo "FAIL B: no 'interpose: declined' line"
		fail=1
	fi
	if [ "$rc" -eq 0 ]; then
		echo "ok   B2: the declined payload still ran (exit 0)"
	else
		echo "FAIL B2: the run exited $rc; a decline must not fail the run"
		fail=1
	fi

	echo "== C. control: a dynamic payload is preloaded, not declined"
	out="$("$BIN" run --rm "$IMAGE" sh -c 'echo reached' 2>&1)"
	if printf '%s\n' "$out" | grep -q 'interpose: declined'; then
		echo "FAIL C: a dynamic payload was declined"
		fail=1
	else
		echo "ok   C: no decline for a dynamic payload"
	fi
	if printf '%s\n' "$out" | grep -q 'interpose: /.podbox/interpose.so'; then
		echo "ok   C2: the banner names the placed object"
	else
		echo "FAIL C2: the banner names no placed object"
		fail=1
	fi
	if printf '%s\n' "$out" | grep -qx 'reached'; then
		echo "ok   C3: the payload ran"
	else
		echo "FAIL C3: the payload output is missing"
		fail=1
	fi

	echo "== verdict"
	if [ "$fail" -eq 0 ]; then echo "ok"; else echo "FAILED"; fi
} | tee "$OUT"
exit "$fail"
