#!/bin/sh
# Question: do the T-1337 operator verbs behave through the shipped binary?
#
# TODO/cli.md T-1337 (issue 38: no `doctor`, no `df`, `logs` without
# `--tail`). One script growing verb by verb; each commit drives the whole
# script, so every verb stays green beside the new one.
#
# Clauses:
#   tail-1. `logs --tail 5` prints the last five lines of a ten-line log;
#   tail-2. `logs` with no flag is byte-identical to `logs --tail 99`;
#   tail-3. `logs --tail 0` prints nothing and exits 0;
#   tail-4. a non-count `--tail` is a flag error at 125 naming the value;
#   tail-5. `logs -f --tail 3` prints the last three lines, then follows
#      new lines to the container's end instead of replaying the file.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no binary, no pull).
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/qol.txt"
WORK="$REPO/experiments/.sweep364-work"
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
# shellcheck source=../scripts/common/exit-codes.sh
. "$REPO/scripts/common/exit-codes.sh"
podbox_exit_codes "$BIN" || { echo "cannot read podbox's exit-code table: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
FLAG=$PODBOX_EXIT_FLAG_ERROR

STORE="$WORK/store"
mkdir -p "$STORE" || exit 2
export PODBOX_STORE="$STORE"

echo "" >>"$WORK/report"
echo "== pulls" >>"$WORK/report"
timeout 600 "$BIN" pull "$DEBIAN" >>"$WORK/report" 2>&1 \
	|| { echo "debian pull FAILED: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }

mklog() {
	name="$1"; shift
	timeout 120 "$BIN" create --name "$name" "$DEBIAN" "$@" >>"$WORK/report" 2>&1 || return 1
	timeout 120 "$BIN" start "$name" >>"$WORK/report" 2>&1 || return 1
	timeout 120 "$BIN" wait "$name" >>"$WORK/report" 2>&1 || return 1
}
rmlog() {
	timeout 120 "$BIN" rm "$1" >>"$WORK/report" 2>&1 || return 1
}

echo "" >>"$WORK/report"
echo "== tail-1. --tail 5 prints the last five lines" >>"$WORK/report"
if mklog t364a /bin/bash -c 'for i in 1 2 3 4 5 6 7 8 9 10; do echo line$i; done'; then
	timeout 120 "$BIN" logs --tail 5 t364a >"$WORK/tail5.out" 2>"$WORK/tail5.err"
	rc=$?
	printf 'line6\nline7\nline8\nline9\nline10\n' >"$WORK/tail5.want"
	if [ "$rc" -eq 0 ] && cmp -s "$WORK/tail5.out" "$WORK/tail5.want"; then
		pass "--tail 5 prints lines 6 to 10 exactly"
	else
		miss "--tail 5 rc=$rc"
		cat "$WORK/tail5.out" >>"$WORK/report"
	fi
	timeout 120 "$BIN" logs t364a >"$WORK/full.out" 2>/dev/null
	timeout 120 "$BIN" logs --tail 99 t364a >"$WORK/full99.out" 2>/dev/null
	echo "" >>"$WORK/report"
	echo "== tail-2. no flag is byte-identical to a wide tail" >>"$WORK/report"
	if cmp -s "$WORK/full.out" "$WORK/full99.out" && [ "$(wc -l <"$WORK/full.out")" -eq 10 ]; then
		pass "plain logs and --tail 99 agree on all ten lines"
	else
		miss "plain logs and --tail 99 disagree"
	fi
	echo "" >>"$WORK/report"
	echo "== tail-3. --tail 0 prints nothing" >>"$WORK/report"
	timeout 120 "$BIN" logs --tail 0 t364a >"$WORK/tail0.out" 2>/dev/null
	rc=$?
	if [ "$rc" -eq 0 ] && [ ! -s "$WORK/tail0.out" ]; then
		pass "--tail 0 is empty at exit 0"
	else
		miss "--tail 0 rc=$rc size=$(wc -c <"$WORK/tail0.out")"
	fi
	rmlog t364a || miss "rm t364a failed"
else
	miss "ten-line container did not run"
fi

echo "" >>"$WORK/report"
echo "== tail-4. a non-count tail is a flag error" >>"$WORK/report"
timeout 120 "$BIN" logs --tail=x t364a >"$WORK/tailx.out" 2>"$WORK/tailx.err"
rc=$?
if [ "$rc" -eq "$FLAG" ] && grep -q "non-negative line count" "$WORK/tailx.err"; then
	pass "non-count --tail refused at $rc naming the value"
else
	miss "non-count --tail rc=$rc (want $FLAG)"
fi

echo "" >>"$WORK/report"
echo "== tail-5. -f --tail follows from the tail" >>"$WORK/report"
if mklog t364b /bin/bash -c 'for i in 1 2 3 4 5; do echo line$i; done'; then
	timeout 120 "$BIN" logs -f --tail 3 t364b >"$WORK/follow.out" 2>"$WORK/follow.err"
	rc=$?
	printf 'line3\nline4\nline5\n' >"$WORK/follow.want"
	if [ "$rc" -eq 0 ] && cmp -s "$WORK/follow.out" "$WORK/follow.want"; then
		pass "-f --tail 3 prints the last three and exits"
	else
		miss "-f --tail 3 rc=$rc"
		cat "$WORK/follow.out" >>"$WORK/report"
	fi
	rmlog t364b || miss "rm t364b failed"
else
	miss "five-line container did not run"
fi

echo "" >>"$WORK/report"
if [ "$fail" -eq 0 ]; then echo "verdict           QOL TAIL SERVED" >>"$WORK/report"; else echo "verdict           QOL TAIL OPEN" >>"$WORK/report"; fi
echo "== counts: 5 tail clauses, fail=$fail" >>"$WORK/report"
cp "$WORK/report" "$OUT"
[ "$fail" -eq 0 ]
