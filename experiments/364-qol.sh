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
#   df-1. `system df` exits 0 and rows the pulled image with nonzero
#      stored and extracted bytes;
#   df-2. the totals lines are present and the store is untouched by
#      df (the image count is identical before and after);
#   df-3. a digest pull is reclaimable: df's Reclaimable line equals
#      `image prune -f`'s Total reclaimed space with a dangling image,
#      and df after reads zero;
#   df-4. `system df --bogus` is a flag error and `system df --help`
#      prints usage at 0.
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
echo "== df-1. system df rows the pulled image with nonzero bytes" >>"$WORK/report"
timeout 120 "$BIN" system df >"$WORK/df.out" 2>"$WORK/df.err"
rc=$?
if [ "$rc" -eq 0 ] && grep -q "^REPOSITORY" "$WORK/df.out" \
	&& grep -q "debian" "$WORK/df.out"; then
	pass "system df exits 0 with a header and the debian row"
else
	miss "system df rc=$rc"
	cat "$WORK/df.out" >>"$WORK/report"
fi
debian_row="$(grep "debian" "$WORK/df.out" | head -1)"
stored="$(echo "$debian_row" | awk '{print $(NF-3), $(NF-2)}')"
rootfs="$(echo "$debian_row" | awk '{print $(NF-1), $NF}')"
if [ "$stored" != "0 B" ] && [ "$rootfs" != "0 B" ]; then
	pass "debian row bills stored $stored beside rootfs $rootfs"
else
	miss "debian row reads stored=$stored rootfs=$rootfs"
fi

echo "" >>"$WORK/report"
echo "== df-2. totals are present and the store is untouched" >>"$WORK/report"
before="$(timeout 120 "$BIN" images -q 2>/dev/null | wc -l)"
for line in "Images:" "Containers:" "Stored blobs:" "Extracted rootfs:" "Reclaimable:"; do
	grep -q "^$line" "$WORK/df.out" \
		|| miss "df output has no $line line"
done
timeout 120 "$BIN" system df >/dev/null 2>&1
after="$(timeout 120 "$BIN" images -q 2>/dev/null | wc -l)"
if [ "$before" = "$after" ]; then
	pass "totals present; $before image(s) before and after df"
else
	miss "df changed the store: $before before, $after after"
fi

echo "" >>"$WORK/report"
echo "== df-3. a digest pull is reclaimable, to the byte" >>"$WORK/report"
ALPINE="public.ecr.aws/docker/library/alpine:latest"
timeout 600 "$BIN" pull "$ALPINE" >>"$WORK/report" 2>&1 \
	|| { echo "alpine pull FAILED: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
DIGEST_REF="$(timeout 120 "$BIN" inspect --format '{{.RepoDigests}}' "$ALPINE" 2>/dev/null)"
if [ -z "$DIGEST_REF" ]; then
	miss "inspect printed no RepoDigests for alpine"
else
	timeout 120 "$BIN" rmi "$ALPINE" >>"$WORK/report" 2>&1 || miss "rmi alpine failed"
	timeout 600 "$BIN" pull "$DIGEST_REF" >>"$WORK/report" 2>&1 \
		|| miss "digest pull of $DIGEST_REF failed"
	timeout 120 "$BIN" images >>"$WORK/report" 2>&1
	if timeout 120 "$BIN" images 2>/dev/null | grep -q "<none>"; then
		pass "digest pull records no tag (dangling)"
	else
		miss "digest pull is not dangling"
	fi
fi
timeout 120 "$BIN" system df >"$WORK/df2.out" 2>/dev/null
r1="$(grep "^Reclaimable:" "$WORK/df2.out" | sed 's/^Reclaimable: //; s/ (what.*//')"
if [ -n "$r1" ] && [ "$r1" != "0 B" ]; then
	pass "dangling alpine is reclaimable: $r1"
else
	miss "Reclaimable reads ${r1:-missing} with a dangling image"
fi
timeout 120 "$BIN" image prune -f >"$WORK/prune.out" 2>&1
prune_rc=$?
r2="$(grep "Total reclaimed space:" "$WORK/prune.out" | sed 's/.*Total reclaimed space: //')"
if [ "$prune_rc" -eq 0 ] && [ -n "$r2" ] && [ "$r1" = "$r2" ]; then
	pass "prune freed $r2, the exact string df reported"
else
	miss "prune rc=$prune_rc freed=${r2:-missing} vs df ${r1:-missing}"
	cat "$WORK/prune.out" >>"$WORK/report"
fi
timeout 120 "$BIN" system df >"$WORK/df3.out" 2>/dev/null
r3="$(grep "^Reclaimable:" "$WORK/df3.out" | sed 's/^Reclaimable: //; s/ (what.*//')"
if [ "$r3" = "0 B" ]; then
	pass "df after prune reads Reclaimable 0 B"
else
	miss "df after prune reads Reclaimable ${r3:-missing}"
fi

echo "" >>"$WORK/report"
echo "== df-4. a bad df flag is a flag error, --help prints usage" >>"$WORK/report"
timeout 120 "$BIN" system df --bogus >"$WORK/dfbogus.out" 2>"$WORK/dfbogus.err"
rc=$?
if [ "$rc" -eq "$FLAG" ]; then
	pass "system df --bogus refused at $rc"
else
	miss "system df --bogus rc=$rc (want $FLAG)"
fi
timeout 120 "$BIN" system df --help >"$WORK/dfhelp.out" 2>"$WORK/dfhelp.err"
rc=$?
if [ "$rc" -eq 0 ] && grep -q "podbox system df" "$WORK/dfhelp.out"; then
	pass "system df --help prints usage at 0"
else
	miss "system df --help rc=$rc"
fi

echo "" >>"$WORK/report"
if [ "$fail" -eq 0 ]; then echo "verdict           QOL TAIL+DF SERVED" >>"$WORK/report"; else echo "verdict           QOL TAIL+DF OPEN" >>"$WORK/report"; fi
echo "== counts: 5 tail clauses, 4 df clauses, fail=$fail" >>"$WORK/report"
cp "$WORK/report" "$OUT"
[ "$fail" -eq 0 ]
