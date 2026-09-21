#!/usr/bin/env bash
# Question: is every non-goal a refusal the code makes, naming its leg and
# its errno, with a leg the host permits turning the refusal off?
#
# TODO/podvm.md T-1306 (the non-goals; the mechanisms come from the spec
# sections the entry cites, E-75 through E-34).
#
# Clauses over the shipped binary itself, read out of `podbox probe --json`
# with the stderr evidence beside it:
#   0. the document carries all six non-goal rows with a stance each, or
#      nothing below can be driven;
#   1. every refused row names an errno in its detail;
#   2. every open row carries no refusal language;
#   3. every unestablished row names why it cannot say;
#   4. kvm acceleration is refused on this lane naming ENOENT, which is
#      T-1301's lane measurement re-read live: a future lane with /dev/kvm
#      fails this clause out loud rather than drifting;
#   5. the stderr evidence carries the non-goals block, so the code path
#      ran rather than only the JSON.
#
# A clause asserts the SHAPE, never which rows take it: whether this
# machine's bind, UTS or ptrace rows are denied or open is the machine's
# answer, and hard-coding it would be the constant the entry forbids.
#
# Runs on Linux, native or in a job container: the binary executes
# directly, and nothing here pulls an image. On a Windows host run it
# inside the base with PODBOX_BIN set to a guest build artifact.
#
# ⚠ `set -u` and no `pipefail`: this script also runs under `sh` in a
# Linux job container, where `sh` is dash. No `read -t`, no coproc, no
# arrays: bounded waits are `timeout`.
#
# Exit: 0 every clause green, 1 a clause failed, 2 a tool, the binary or the
# document could not run.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/podvm-non-goals.txt"
WORK="$REPO/experiments/.sweep149-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

STORE="$WORK/store"
export PODBOX_STORE="$STORE"

fail=0
fails=0
driven=0
say() { printf '%s\n' "$*" >>"$WORK/report"; }
pass() { driven=$((driven + 1)); say "  ok: $1"; }
miss() { fail=1; fails=$((fails + 1)); say "  FAIL: $1"; }

[ -x "$BIN" ] || {
	echo "SKIP: $BIN is not an executable. Build it:" >&2
	echo "      cargo build --release --target x86_64-unknown-linux-musl" >&2
	exit 2
}
command -v jq >/dev/null 2>&1 || { echo "SKIP: jq is not on PATH" >&2; exit 2; }

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -r)"
	printf 'podbox            %s\n' "$("$BIN" version)"
	printf 'jq                %s\n' "$(jq --version)"
	echo
} >"$WORK/report"

# The document, read out of the binary. Every clause starts here, so a
# section the binary does not publish is a section this script cannot drive.
# The bare run beside it captured the stderr evidence: `--json` keeps
# stdout pure JSON and prints no evidence, so the block is read there.
timeout 120 "$BIN" probe --json >"$WORK/doc.json" 2>/dev/null
[ "$?" -eq 0 ] || { echo "SKIP: podbox probe --json did not run" >&2; exit 2; }
timeout 120 "$BIN" probe >"$WORK/rung.txt" 2>"$WORK/evidence.txt"
jq -e '.non_goals' "$WORK/doc.json" >/dev/null 2>&1 || {
	echo "SKIP: the document carries no non_goals section" >&2; exit 2;
}

# One stance per row, carriage returns stripped: a jq that ends lines with
# CRLF would otherwise poison every comparison but the last.
stance() {
	jq -r --arg n "$1" '.non_goals[] | select(.name == $n) | .stance' \
		"$WORK/doc.json" | tr -d '\r'
}
detail() {
	jq -r --arg n "$1" '.non_goals[] | select(.name == $n) | .detail' \
		"$WORK/doc.json" | tr -d '\r'
}

NAMES="tcp listener
kvm acceleration
runc-style oci
user-mode linux guest
uid_map identity switch
guest file over the ceiling"

say "== 0. six rows with a stance each"
while IFS= read -r n; do
	[ -n "$n" ] || continue
	s="$(stance "$n")"
	case "$s" in
		refused|open|unestablished) pass "row $n carries stance $s" ;;
		*) miss "row $n carries no stance (got: $s)" ;;
	esac
done <<NAMES
$NAMES
NAMES

say ""
say "== 1. every refusal names its errno"
while IFS= read -r n; do
	[ -n "$n" ] || continue
	[ "$(stance "$n")" = "refused" ] || continue
	d="$(detail "$n")"
	if printf '%s' "$d" | grep -Eq '=[A-Z][A-Z0-9]*'; then
		pass "refused $n names an errno"
	else
		miss "refused $n names no errno: $d"
	fi
done <<NAMES
$NAMES
NAMES

say ""
say "== 2. every open row carries no refusal language"
while IFS= read -r n; do
	[ -n "$n" ] || continue
	[ "$(stance "$n")" = "open" ] || continue
	d="$(detail "$n")"
	if printf '%s' "$d" | grep -q "refused"; then
		miss "open $n reads as a refusal: $d"
	else
		pass "open $n carries no refusal language"
	fi
done <<NAMES
$NAMES
NAMES

say ""
say "== 3. every unestablished row names why it cannot say"
while IFS= read -r n; do
	[ -n "$n" ] || continue
	[ "$(stance "$n")" = "unestablished" ] || continue
	d="$(detail "$n")"
	if printf '%s' "$d" | grep -Eq 'unestablished: .+'; then
		pass "unestablished $n names its reason"
	else
		miss "unestablished $n names no reason: $d"
	fi
done <<NAMES
$NAMES
NAMES

say ""
say "== 4. kvm acceleration is refused on this lane naming ENOENT"
if [ "$(stance "kvm acceleration")" = "refused" ] \
	&& detail "kvm acceleration" | grep -q "open(/dev/kvm, O_RDWR)=ENOENT"; then
	pass "kvm acceleration refused naming open(/dev/kvm, O_RDWR)=ENOENT"
else
	miss "kvm acceleration (stance: $(stance "kvm acceleration")): $(detail "kvm acceleration")"
fi

say ""
say "== 5. the stderr evidence carries the block"
if grep -q "non-goals, one stance per blocked design" "$WORK/evidence.txt"; then
	pass "the evidence carries the non-goals block"
else
	miss "the evidence carries no non-goals block"
fi

say ""
say "== counts: $driven driven, $fails mismatches"
cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"

[ "$fail" -eq 0 ] || exit 1
exit 0
