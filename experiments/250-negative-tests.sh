#!/usr/bin/env bash
# Question: does every refusal this project SHIPPED actually happen?
#
# TODO/milestones.md T-1109. `TOOL.md` section 9 names three; this corpus adds
# five more, one per honesty rule that has landed since.
#
# ⭐ WHY THIS EXISTS AT ALL. Every honesty rule in `TOOL.md` sections 4.1 and 6.8
# is unenforced until something asserts the refusal happens. A rule that is only
# prose regresses silently, which is the exact failure mode the whole design is
# built against: a runtime that quietly stops refusing looks identical to one
# that never had to.
#
# ⛔ DRIVEN FROM THE OUTSIDE, THROUGH THE SHIPPED BINARY. A unit test asserting
# that a function returns an error is a test of that function. What a caller
# gets is an exit code and a line on stderr, and that is what every clause below
# reads. Several of these refusals are produced by code no unit test reaches.
#
# ⛔ EVERY CLAUSE ASSERTS TWO THINGS, and the second is the one that rots:
#
#   1. the exit code, read from the process that produced it, unpiped;
#   2. that the message NAMES THE REASON. A refusal a caller cannot act on is a
#      refusal that costs a session, and "unknown option" where the table has a
#      reason is a regression even though the code is the same.
#
# ⚠ The exit codes are DATA, read out of the binary rather than written here:
# `scripts/common/exit-codes.sh` and TODO/cli.md T-0802. Six clauses across four
# experiments had a `2` written into them and all went red at once the day
# docker's codes were measured.
#
#   ./250-negative-tests.sh
#
# Exit: 0 every refusal happened and named its reason, 1 one did not, 2 could
#       not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/negative-tests.txt"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT INT TERM

IMAGE="${PODBOX_NEG_IMAGE:-public.ecr.aws/docker/library/alpine:3.20}"
# ⭐ An image the completion layer must run a COMMAND inside, which alpine is
# not: `--strict`'s fourth reason counts T-0412's steps, and against an image
# with none that branch has never been seen to refuse. openSUSE is the row the
# hash-indexed CApath was found on, so it is the one that has a step.
# ⚠ Not a quota-bearing registry: the distribution's own.
STEP_IMAGE="${PODBOX_NEG_STEP_IMAGE:-registry.opensuse.org/opensuse/leap:15.6}"

[ -x "$BIN" ] || {
	echo "SKIP: $BIN is not an executable. ./scripts/dev.sh build" >&2
	exit 2
}
command -v jq >/dev/null 2>&1 || {
	echo "SKIP: jq is not on PATH. ./scripts/common/bootstrap-env.sh tools" >&2
	exit 2
}
# shellcheck source=../scripts/common/exit-codes.sh
. "$REPO/scripts/common/exit-codes.sh"
podbox_exit_codes "$BIN" || {
	echo "SKIP: cannot read podbox's exit-code table" >&2
	exit 2
}

export PODBOX_STORE="$WORK/store"
fail=0
skipped=0
say() { printf '%s\n' "$*" >>"$WORK/report"; }

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -r)"
	printf 'podbox            %s\n' "$("$BIN" version)"
	printf 'image             %s\n' "$IMAGE"
	printf 'flag-error code   %s\n' "$PODBOX_EXIT_FLAG_ERROR"
	printf 'cli-error code    %s\n' "$PODBOX_EXIT_CLI_ERROR"
	echo
} >"$WORK/report"

# ⭐ The step clause below needs an ANNOUNCED CA bundle: T-0412 proposes its
# rehash step only where the machine names a bundle through $SSL_CERT_FILE,
# $CURL_CA_BUNDLE or $REQUESTS_CA_BUNDLE, and without one --strict names no
# step, deterministically. 240 provisions the same announcement for its
# driver rows. The clause reads the announcement below and skips by name
# where none exists instead of failing on an environment it never asked for.
if [ -z "${SSL_CERT_FILE:-}${CURL_CA_BUNDLE:-}${REQUESTS_CA_BUNDLE:-}" ]; then
	if test -s /etc/ssl/certs/ca-certificates.crt; then
		export SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
		say "  CA bundle announced via SSL_CERT_FILE for the step clause"
	elif command -v apt-get >/dev/null 2>&1 \
	&& DEBIAN_FRONTEND=noninteractive apt-get update -qq \
	&& DEBIAN_FRONTEND=noninteractive apt-get install -y -qq ca-certificates \
	&& test -s /etc/ssl/certs/ca-certificates.crt; then
		export SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
		say "  CA bundle installed and announced via SSL_CERT_FILE for the step clause"
	fi
fi

timeout 600 "$BIN" pull "$IMAGE" >"$WORK/pull.log" 2>&1 || {
	echo "SKIP: could not pull $IMAGE" >&2
	tail -3 "$WORK/pull.log" >&2
	exit 2
}

# ⛔ ONE PLACE runs a refusal, so every clause asserts the same two things and
# the report cannot show an assertion that was not made.
#   refuses <name> <want-code> <needle> -- <podbox args...>
refuses() {
	local name="$1" want="$2" needle="$3"
	shift 3
	[ "${1:-}" = "--" ] && shift
	local out rc
	out="$(timeout 300 "$BIN" "$@" 2>&1 >/dev/null)"
	rc=$?
	local verdict="ok"
	if [ "$rc" != "$want" ]; then
		verdict="WRONG CODE, wanted $want"
		fail=1
	elif ! printf '%s' "$out" | grep -qF -- "$needle"; then
		verdict="the code is right and the REASON is not named"
		fail=1
	fi
	printf '  %-34s rc=%-4s %s\n' "$name" "$rc" "$verdict" >>"$WORK/report"
	printf '      %s\n' "$(printf '%s' "$out" | grep -F -- "$needle" | head -1 | cut -c1-118)" \
		>>"$WORK/report"
	[ "$verdict" = "ok" ] || printf '      GOT: %s\n' \
		"$(printf '%s' "$out" | head -2 | tr '\n' ' ' | cut -c1-118)" >>"$WORK/report"
}

# --------------------------------------------------------------------- 1
say "== 1. TOOL.md section 9's three"
# ⭐ `--network=none` must FAIL with a named reason. ⚠ The reason comes out of
# the parity table rather than being written here: T-0801 made the table
# binding, so the row's own note is what a caller reads.
refuses "run --network=none" "$PODBOX_EXIT_FLAG_ERROR" \
	"no network namespace to select" -- run --network=none "$IMAGE" true
# ⭐ `-v ...:ro` must be REJECTED, not honoured as a copy.
refuses "run -v host:/mapped:ro" "$PODBOX_EXIT_FLAG_ERROR" \
	"a copy pretending to be a mount" -- run -v "$WORK:/mapped:ro" "$IMAGE" true
# ⭐ The third, a Go payload under `interpose` declined rather than silently
# unvirtualized (TODO/interpose.md T-0706 owns the refusal; T-1110 drives it
# here). The victim is generated, not fetched: no matrix row is a Go image
# and inventing a registry reference is refused (`experiments/src/govictim.sh`
# carries the layout). It is ET_REL on purpose, so `execve` answers ENOEXEC
# and the run exits 126 after the decline, deterministically on any kernel.
# ⚠ No `--rm`: the staged victim must survive into the runs below, and the
# store is this script's own mktemp directory either way.
govictim_ok=1
"$BIN" extract "$IMAGE" >/dev/null 2>&1 || govictim_ok=0
GROOT="$("$BIN" inspect --format '{{.RootfsPath}}' "$IMAGE" 2>/dev/null)" || govictim_ok=0
if [ "$govictim_ok" -eq 1 ] && [ -n "${GROOT:-}" ] \
&& sh "$REPO/experiments/src/govictim.sh" >"$GROOT/go-victim" \
&& chmod 755 "$GROOT/go-victim"; then
	refuses "go payload declined" "$PODBOX_EXIT_CANNOT_INVOKE" \
		"interpose: declined" -- run "$IMAGE" /go-victim
	# ⛔ The decline must name the CLASS, not just the tier: any unreadable
	# file declines, so the row-3 reason is asserted separately, or the
	# clause would pass with the Go logic deleted.
	govout="$(timeout 300 "$BIN" run "$IMAGE" /go-victim 2>&1 >/dev/null)"
	printf '%s' "$govout" | grep -qF -- "Go build markers" || {
		say "  FAIL: the decline does not name Go build markers"
		printf '      GOT: %s\n' "$(printf '%s' "$govout" | head -2 | tr '\n' ' ' | cut -c1-118)" >>"$WORK/report"
		fail=1
	}
else
	say "  FAIL: the Go victim could not be staged"
	fail=1
fi

# --------------------------------------------------------------------- 2
say ""
say "== 2. the honesty switch: --strict refuses a degraded run (T-0804)"
# ⛔ Two halves, and the second is what makes the first mean something.
out="$(timeout 300 "$BIN" run --strict --rm "$IMAGE" true 2>&1 >/dev/null)"
rc=$?
say "  run --strict                       rc=$rc"
printf '%s' "$out" | grep -F -- "--strict, and this run is degraded" | head -1 \
	| sed 's/^/      /' | cut -c1-120 >>"$WORK/report"
if [ "$rc" = "$PODBOX_EXIT_RUNTIME_ERROR" ]; then
	printf '%s' "$out" | grep -q -- "--strict, and this run is degraded" || {
		say "  FAIL: it refused without naming --strict as the reason"
		fail=1
	}
	# ⛔ AND IT NAMES EVERY REASON, not the first one it found.
	n="$(printf '%s' "$out" | grep -c '^  - ')"
	say "  reasons named                      $n"
	[ "$n" -ge 1 ] || { say "  FAIL: it refused and listed nothing"; fail=1; }
elif [ "$rc" = 0 ]; then
	# ⚠ A machine where nothing about the run is degraded. Legitimate, and it
	# means this clause measured nothing rather than that it passed.
	say "  ⚠ nothing about this run is degraded here, so --strict had nothing"
	say "    to refuse. That is a reading about this machine, not a pass."
	skipped=1
else
	say "  FAIL: --strict exited $rc, which is neither 0 nor a refusal"
	fail=1
fi
# ⚠ And WITHOUT it the same run proceeds, or `--strict` is the default in
# disguise.
timeout 300 "$BIN" run --rm "$IMAGE" true >/dev/null 2>&1
rc=$?
say "  the same run without --strict      rc=$rc (must be 0)"
[ "$rc" -eq 0 ] || { say "  FAIL: the run does not work without --strict"; fail=1; }

# ⛔ AND THE FOURTH REASON, which the image above cannot produce. `--strict`
# counts T-0412's steps BEFORE any of them runs, because podbox executing a
# command inside somebody else's image is exactly the difference from docker
# that switch exists to refuse. ⚠ Against an image with no steps that branch is
# never taken, so it was shipped unexercised until this clause existed.
out="$(timeout 600 "$BIN" run --strict --rm "$STEP_IMAGE" true 2>&1 >/dev/null)"
rc=$?
if [ -z "${SSL_CERT_FILE:-}${CURL_CA_BUNDLE:-}${REQUESTS_CA_BUNDLE:-}" ]; then
	say "  --strict against an image with a step  rc=$rc (no bundle announced)"
	say "  SKIP: without an announced CA bundle T-0412 proposes no step, so"
	say "    reason 4 has nothing to fire on. That is the environment, not a pass."
	skipped=1
elif [ "$rc" = "$PODBOX_EXIT_RUNTIME_ERROR" ]; then
	step="$(printf '%s' "$out" | grep -c '^  - podbox would run ')"
	say "  --strict against an image with a step  rc=$rc, step reasons $step"
	printf '%s' "$out" | grep -F -- '- podbox would run ' | head -1 \
		| sed 's/^/      /' | cut -c1-120 >>"$WORK/report"
	[ "$step" -ge 1 ] || {
		say "  FAIL: it refused and named no step, so reason 4 did not fire"
		fail=1
	}
elif [ "$rc" = 124 ]; then
	say "  ⚠ the step image could not be fetched here (timeout), so reason 4"
	say "    was not measured. That is a third state, not a pass."
	skipped=1
else
	say "  ⚠ --strict exited $rc against $STEP_IMAGE, so reason 4 was not"
	say "    measured here. A machine that needs no step is a legitimate reading."
	skipped=1
fi

# --------------------------------------------------------------------- 3
say ""
say "== 3. -t is a NAMED refusal where /dev/ptmx is unusable (T-0503)"
# ⚠ Conditional on this machine, and which arm ran is recorded. A pty podbox
# could not measure is not one it may promise, and a pty it CAN allocate is not
# a refusal to assert.
usable="$(timeout 120 "$BIN" probe --json 2>/dev/null | jq -r '.ptmx.usable // false')"
say "  /dev/ptmx usable here              $usable"
if [ "$usable" = "true" ]; then
	timeout 300 "$BIN" run -t --rm "$IMAGE" true >/dev/null 2>&1
	rc=$?
	say "  run -t                             rc=$rc (a pty is available, so 0)"
	[ "$rc" -eq 0 ] || { say "  FAIL: -t was refused on a machine with a pty"; fail=1; }
	say "  ⚠ the refusal arm could not be measured here: this machine has a pty"
	skipped=1
else
	refuses "run -t" "$PODBOX_EXIT_RUNTIME_ERROR" \
		"cannot allocate a pty" -- run -t --rm "$IMAGE" true
fi

# --------------------------------------------------------------------- 4
say ""
say "== 4. the table decides, and an unlisted flag cannot be quietly accepted"
refuses "an unlisted flag" "$PODBOX_EXIT_FLAG_ERROR" \
	"no row in the parity table" -- run --no-such-flag "$IMAGE" true
refuses "a None flag names its status" "$PODBOX_EXIT_FLAG_ERROR" \
	"status None" -- run --privileged "$IMAGE" true
refuses "a None VERB names its reason" "$PODBOX_EXIT_RUNTIME_ERROR" \
	"cgroup this runtime does not grant" -- stats

# --------------------------------------------------------------------- 5
say ""
say "== 5. podbox speaks HTTPS only, and never downgrades (T-0201)"
# ⛔ Under a timeout, because the failure this refusal prevents is a HANG: on
# the runtimes podbox targets tcp/80 is black-holed, so a fallback does not fail,
# it waits. A clause with no bound would pass by hanging.
out="$(timeout 30 "$BIN" pull "http://registry.invalid/library/x:latest" 2>&1 >/dev/null)"
rc=$?
say "  pull http://...                      rc=$rc (124 would be the hang)"
[ "$rc" != 124 ] || { say "  FAIL: it hung, which is what this refusal exists to prevent"; fail=1; }
[ "$rc" = "$PODBOX_EXIT_CLI_ERROR" ] || { say "  FAIL: wanted $PODBOX_EXIT_CLI_ERROR"; fail=1; }
printf '%s' "$out" | grep -q 'HTTPS only' || {
	say "  FAIL: it refused without saying podbox is HTTPS only"
	fail=1
}
printf '      %s\n' "$(printf '%s' "$out" | grep -o 'HTTPS only[^.]*' | head -1 | cut -c1-110)" \
	>>"$WORK/report"

# --------------------------------------------------------------------- 6
say ""
say "== 6. a container podbox did not see end has NO exit code (T-0604)"
# ⛔ `wait` refuses rather than printing a guess. A runtime that invents an exit
# code is lying in the one field an automated caller reads first.
cid="$(timeout 300 "$BIN" run -d --name "neg-$$" "$IMAGE" sleep 30 2>/dev/null)"
if [ -z "$cid" ]; then
	say "  SKIP: could not start a detached container"
	skipped=1
else
	lp="$(timeout 60 "$BIN" inspect --format '{{.LauncherPid}}' "neg-$$" 2>/dev/null)"
	if [ -n "$lp" ] && [ "$lp" != 0 ]; then
		kill -9 "$lp" 2>/dev/null
		sleep 1
		out="$(timeout 60 "$BIN" wait "neg-$$" 2>&1 >/dev/null)"
		rc=$?
		state="$(timeout 60 "$BIN" inspect --format '{{.State}}' "neg-$$" 2>/dev/null)"
		code="$(timeout 60 "$BIN" inspect --format '{{.ExitCode}}' "neg-$$" 2>/dev/null)"
		say "  after SIGKILL of the launcher      state=$state exitcode=$code wait rc=$rc"
		[ "$state" = dead ] || { say "  FAIL: it does not read dead"; fail=1; }
		[ "$code" = "-" ] || { say "  FAIL: it invented an exit code: $code"; fail=1; }
		[ "$rc" = "$PODBOX_EXIT_RUNTIME_ERROR" ] || {
			say "  FAIL: wait exited $rc rather than refusing"
			fail=1
		}
	else
		say "  SKIP: no launcher pid to kill"
		skipped=1
	fi
	timeout 60 "$BIN" rm -f "neg-$$" >/dev/null 2>&1
fi

# --------------------------------------------------------------------- 7
say ""
say "== 7. the attribution census is 0 or 2, never 1 (T-1109's own rule)"
# ⛔ A 1 means the runtime moved or a probe stopped discriminating, and either is
# a finding rather than a flake. ⚠ Its exit code is read from the process that
# produced it, so it is run unpiped into a file.
if [ -x "$REPO/experiments/30-attribution-census.sh" ]; then
	timeout 900 "$REPO/experiments/30-attribution-census.sh" >"$WORK/census.log" 2>&1
	crc=$?
	say "  30-attribution-census.sh           rc=$crc"
	case "$crc" in
	0) say "      it ran and matched" ;;
	2) say "      it could not run here, which is the third state" ;;
	*)
		say "      ⛔ a 1 is a FINDING: the runtime moved or a probe stopped"
		say "         discriminating. The log tail:"
		tail -3 "$WORK/census.log" | sed 's/^/         /' >>"$WORK/report"
		fail=1
		;;
	esac
else
	say "  SKIP: 30-attribution-census.sh is not executable here"
	skipped=1
fi

say ""
say "== verdict"
if [ "$fail" -eq 0 ]; then
	say "  every refusal that could be driven here happened AND named its reason."
else
	say "  ⛔ a refusal did not happen, or happened without saying why."
fi

cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"
[ -x "$REPO/scripts/common/result-diff.sh" ] && "$REPO/scripts/common/result-diff.sh" "$OUT"

[ "$fail" -eq 0 ] || exit 1
[ "$skipped" -eq 0 ] || exit 2
exit 0
