#!/usr/bin/env bash
# Question: does `run -t` refuse by name where /dev/ptmx is unusable?
#
# TODO/milestones.md T-1109, condition 1. `250-negative-tests.sh` clause 3
# takes the positive arm on every machine with a working pty and records
# the refusal arm as skipped, so the refusal wiring had no end-to-end
# drive. This script builds the missing machine: a private mount
# namespace where an empty directory covers the pty path (/dev/pts
# where /dev/ptmx is a symlink there, /dev/ptmx itself where it is
# a directory). It then drives the refusal through the shipped binary
# and checks the two things
# every 250 clause checks: the exit code from the process that produced
# it, and a message that names the reason.
#
# After the focused drive it runs 250 itself under the same cover, so
# the renewed `negative-tests.txt` carries the refusal arm measured
# rather than skipped. Where the census clause still skips, it runs
# the census once more and records the reason, so the remaining skip
# names its cause instead of standing alone.
#
#   ./251-tty-refusal-no-ptmx.sh
#
# Exit: 0 the refusal fired and named its reason, 1 the wiring is
#       broken, 2 it could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/tty-refusal-no-ptmx.txt"
SELF="$HERE/251-tty-refusal-no-ptmx.sh"

IMAGE="${PODBOX_NEG_IMAGE:-public.ecr.aws/docker/library/alpine:3.20}"

INNER=0
[ "${1:-}" = "--inner" ] && INNER=1

[ -x "$BIN" ] || {
	echo "SKIP: $BIN is not an executable. ./scripts/dev.sh build" >&2
	exit 2
}
command -v jq >/dev/null 2>&1 || {
	echo "SKIP: jq is not on PATH" >&2
	exit 2
}
command -v timeout >/dev/null 2>&1 || {
	echo "SKIP: timeout is not on PATH" >&2
	exit 2
}
# shellcheck source=../scripts/common/exit-codes.sh
. "$REPO/scripts/common/exit-codes.sh"
podbox_exit_codes "$BIN" || {
	echo "SKIP: cannot read podbox's exit-code table" >&2
	exit 2
}

WORK="${PODBOX_TTY_WORK:-$(mktemp -d)}"
export PODBOX_TTY_WORK="$WORK"
export WORK
export PODBOX_STORE="$WORK/store"
# Only the outer half owns the scratch. The inner half runs in a child
# process and shares the path; a trap there would clear the report
# before the outer half reads it.
[ "$INNER" -eq 0 ] && trap 'rm -rf "$WORK"' EXIT INT TERM

say() { printf '%s\n' "$*" >>"$WORK/report"; }

probe_usable() {
	# Bare `.ptmx.usable`: true, false, or null where the key is
	# absent. A `// "missing"` fallback must not stand here: jq's
	# `//` fires on false as well as null, so a covered probe
	# reporting false would read as missing (lane, 2026-09-25).
	timeout 120 "$BIN" probe --json 2>/dev/null | jq -r '.ptmx.usable'
}

finalize() {
	say ""
	say "== verdict"
	case "$1" in
	0) say "  the refusal fired where ptmx is unusable, and named its reason." ;;
	1) say "  the refusal wiring is broken where ptmx is unusable." ;;
	*) say "  the refusal could not be driven here." ;;
	esac
	cat "$WORK/report"
	mkdir -p "$(dirname "$OUT")"
	cp "$WORK/report" "$OUT"
	echo
	echo "written to ${OUT#"$REPO"/}"
	[ -x "$REPO/scripts/common/result-diff.sh" ] && "$REPO/scripts/common/result-diff.sh" "$OUT"
	exit "$1"
}

# Runs covered: hides the pty path where the probe reports it usable,
# drives the focused refusal, then runs 250 under the same cover.
# The cover follows the layout: /dev/ptmx is usually a symlink into
# /dev/pts, and a directory binds onto a directory only. Binding over
# the symlink answers "not a directory" (lane, 2026-09-25), so the
# cover lands on /dev/pts instead and leaves the symlink dangling,
# which makes open answer ENOENT.
inner() {
	now="$(probe_usable)"
	if [ "$now" != "true" ] && [ "$now" != "false" ]; then
		say "== cover"
		say "  SKIP: the probe answered [$now], neither true nor false."
		printf '2' >"$WORK/focused"
		return 2
	fi
	if [ "$now" = "false" ]; then
		say "== cover"
		say "  no cover: this machine already reports ptmx unusable."
	else
		if [ -L /dev/ptmx ]; then
			link="$(readlink /dev/ptmx)"
			say "== cover"
			say "  /dev/ptmx is a symlink to $link."
			case "$link" in
			*pts*) COVER=/dev/pts ;;
			*)
				say "  SKIP: the cover needs a symlink into /dev/pts or a"
				say "    directory, and this target is neither."
				printf '2' >"$WORK/focused"
				return 2
				;;
			esac
		elif [ -d /dev/ptmx ]; then
			say "== cover"
			say "  /dev/ptmx is a directory."
			COVER=/dev/ptmx
		else
			say "== cover"
			say "  SKIP: /dev/ptmx is neither a symlink nor a directory,"
			say "    so the cover has no target here."
			printf '2' >"$WORK/focused"
			return 2
		fi
		mkdir -p "$WORK/empty" || {
			say "  SKIP: no empty directory for the cover."
			printf '2' >"$WORK/focused"
			return 2
		}
		before="$(grep -c " $COVER " /proc/mounts 2>/dev/null || true)"
		if mount --bind "$WORK/empty" "$COVER" 2>"$WORK/mount.err"; then
			say "  an empty directory covers $COVER (mount --bind)."
		else
			say "  SKIP: mount --bind over $COVER failed here:"
			say "    $(head -1 "$WORK/mount.err")"
			printf '2' >"$WORK/focused"
			return 2
		fi
		after="$(grep -c " $COVER " /proc/mounts 2>/dev/null || true)"
		if [ "$after" -gt "$before" ] 2>/dev/null; then
			say "  the mount table carries the cover ($before -> $after lines)."
		else
			say "  SKIP: the cover left no trace in the mount table."
			printf '2' >"$WORK/focused"
			return 2
		fi
	fi
	# Read the covered probe from the artefact, not from a pipe: where
	# the answer is not true or false the bytes and the exit code are
	# the evidence, so they are captured here instead of discarded.
	timeout 120 "$BIN" probe --json >"$WORK/probe-covered.json" 2>"$WORK/probe-covered.err"
	prc=$?
	pbytes="$(wc -c <"$WORK/probe-covered.json" 2>/dev/null || printf '?')"
	covered="$(jq -r '.ptmx.usable' "$WORK/probe-covered.json" 2>/dev/null)"
	say "  probe --json under the cover: exit $prc, $pbytes bytes."
	say "  ptmx usable under the cover: ${covered:-<empty>}."
	if [ "$covered" != "true" ] && [ "$covered" != "false" ]; then
		say "  the covered report carries no usable answer. Head of stdout:"
		head -8 "$WORK/probe-covered.json" | sed 's/^/    /' | cut -c1-118 >>"$WORK/report"
		say "  tail of stderr:"
		tail -5 "$WORK/probe-covered.err" | sed 's/^/    /' | cut -c1-118 >>"$WORK/report"
	fi
	if [ "$covered" = "true" ]; then
		say "  FAIL: the probe still reports a usable pty under the cover,"
		say "    so the predicate reads from somewhere else."
		printf '1' >"$WORK/focused"
		return 1
	fi
	if [ "$covered" != "false" ]; then
		say "  SKIP: the probe answered [$covered], neither true nor false."
		printf '2' >"$WORK/focused"
		return 2
	fi
	say ""
	say "== focused refusal: run -t where ptmx is unusable"
	if ! timeout 600 "$BIN" pull "$IMAGE" >"$WORK/pull.log" 2>&1; then
		say "  SKIP: could not pull $IMAGE."
		tail -3 "$WORK/pull.log" | sed 's/^/    /' >>"$WORK/report"
		printf '2' >"$WORK/focused"
		return 2
	fi
	out="$(timeout 300 "$BIN" run -t --rm "$IMAGE" true 2>&1 >/dev/null)"
	rc=$?
	say "  run -t --rm $IMAGE true   rc=$rc (want $PODBOX_EXIT_RUNTIME_ERROR)"
	if [ "$rc" != "$PODBOX_EXIT_RUNTIME_ERROR" ]; then
		say "  FAIL: wrong code."
		say "    GOT: $(printf '%s' "$out" | head -2 | tr '\n' ' ' | cut -c1-118)"
		printf '1' >"$WORK/focused"
		return 1
	fi
	if ! printf '%s' "$out" | grep -qF -- "cannot allocate a pty"; then
		say "  FAIL: the code is right and the reason is not named."
		say "    GOT: $(printf '%s' "$out" | head -2 | tr '\n' ' ' | cut -c1-118)"
		printf '1' >"$WORK/focused"
		return 1
	fi
	say "  ok: refused and named:"
	say "    $(printf '%s' "$out" | grep -F -- "cannot allocate a pty" | head -1 | cut -c1-114)"
	printf '0' >"$WORK/focused"
	say ""
	say "== full 250 under the same cover"
	timeout 2400 "$HERE/250-negative-tests.sh" >"$WORK/250.log" 2>&1
	t250=$?
	say "  250 exit: $t250 (0 every refusal measured, 2 a clause could not run)."
	grep -F "run -t" "$WORK/250.log" | sed 's/^/    /' | cut -c1-120 >>"$WORK/report" || true
	grep -F "30-attribution-census.sh" "$WORK/250.log" | sed 's/^/    /' | cut -c1-120 >>"$WORK/report" || true
	census_rc="$(grep -F '30-attribution-census.sh' "$WORK/250.log" 2>/dev/null | grep -o 'rc=[0-9][0-9]*' | head -1 | cut -d= -f2 || true)"
	if [ "$census_rc" != "0" ]; then
		say ""
		say "== why the census clause is not green (one more run, as the control)"
		timeout 900 "$HERE/30-attribution-census.sh" >"$WORK/census.log" 2>&1
		ccrc=$?
		say "  census exit on the repeat: $ccrc."
		tail -20 "$WORK/census.log" | sed 's/^/    /' >>"$WORK/report"
	fi
	return "$(cat "$WORK/focused")"
}

if [ "$INNER" -eq 1 ]; then
	inner
	exit "$?"
fi

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -r)"
	printf 'podbox            %s\n' "$("$BIN" version)"
	printf 'image             %s\n' "$IMAGE"
	printf 'flag-error code   %s\n' "$PODBOX_EXIT_FLAG_ERROR"
	printf 'cli-error code    %s\n' "$PODBOX_EXIT_CLI_ERROR"
	printf 'runtime-error     %s\n' "$PODBOX_EXIT_RUNTIME_ERROR"
	pre="$(probe_usable)"
	printf 'ptmx usable here  %s\n' "$pre"
	echo
} >"$WORK/report"

case "$pre" in
true)
	command -v unshare >/dev/null 2>&1 || {
		say "SKIP: unshare is not on PATH, so no private mount namespace."
		finalize 2
	}
	command -v mount >/dev/null 2>&1 || {
		say "SKIP: mount is not on PATH, so nothing can cover /dev/ptmx."
		finalize 2
	}
	if ! unshare --user --map-root-user --mount true 2>"$WORK/unshare.err"; then
		say "SKIP: unshare --user --map-root-user --mount fails here:"
		say "  $(head -1 "$WORK/unshare.err")"
		finalize 2
	fi
	say "namespaces answer: the drive re-runs under unshare --user --map-root-user --mount."
	timeout 2700 unshare --user --map-root-user --mount "$SELF" --inner
	rc=$?
	if [ "$rc" -eq 124 ]; then
		say "the covered drive timed out (124): it ran but never answered."
		rc=2
	fi
	;;
false)
	say "this machine already reports ptmx unusable: driving with no cover."
	"$SELF" --inner
	rc=$?
	;;
*)
	say "SKIP: the probe answered [$pre]."
	finalize 2
	;;
esac
finalize "$rc"
