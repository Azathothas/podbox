#!/bin/sh
# Question: do the root-listing legs and the ptmx legs run and report
# their own errno?
#
# TODO/complete.md T-0414 (the root-listing clause and the `/dev/ptmx`
# clause). The proc-absence clause this filename promises is
# TODO/complete.md T-0413's and arrives with its ruling; this script
# asserts no proc shape until then.
#
# The binary is `$PODBOX_BIN`, a lane-built podbox (the legs are its
# code). A non-native lane stages it through the M5 debian driver, the
# experiments/270 pattern: the probe runs inside the driver container,
# and the conditions block says so.
#
#   ./155-proc-absence.sh
#
# Exit: 0 every clause held, 1 one did not, 2 a leg could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/proc-absence.txt"
WORK="$REPO/experiments/.sweep155-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
trap 'eng_cleanup 2>/dev/null; rm -rf "$WORK"' EXIT INT TERM

fail=0
leg_not_run() {
	echo "  $1: could not run ($2)" >>"$WORK/report"
	echo 2 >"$WORK/worst"
}

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
case "$(uname -s)" in
# The artifact is a Linux binary: executing it here would prove nothing
# about the binary and only that this host is not Linux. Readability is
# the check; the driver runs it below.
Linux) echo "podbox            $($BIN version 2>/dev/null || echo MISSING)" ;;
*) if [ -r "$BIN" ]; then echo "podbox            lane-built artifact, runs in the driver below"; else echo "podbox            MISSING ($BIN unreadable)"; fi ;;
esac
} >"$WORK/report"

command -v jq >/dev/null 2>&1 || { echo "jq is missing" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
# shellcheck source=lib/engine.sh
. "$HERE/lib/engine.sh"
export ENGINE_REPO ENGINE_WORK
ENGINE_REPO="$REPO"
ENGINE_WORK="$WORK"
if engine_pick; then HAVE_ENGINE=1; else HAVE_ENGINE=0; fi
[ "$HAVE_ENGINE" -eq 1 ] || { echo "engine            none answered" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }

# The driver hosts the staged binary on a non-native lane; natively the
# binary executes where it stands.
DRIVER='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
case "$(uname -s)" in
Linux) BIN_RUN="$BIN"; [ -x "$BIN_RUN" ] || { echo "binary $BIN is not executable" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; } ;;
*)
	[ -r "$BIN" ] || { echo "binary $BIN is not readable (set PODBOX_BIN to a guest build artifact)" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
	STAGE="$WORK/stage"
	eng_pull "$DRIVER" || exit 2
	mkdir -p "$STAGE" "$WORK/w" || exit 2
	cp "$BIN" "$STAGE/podbox" || exit 2
	eng_mount "$STAGE/podbox" /pb \
		&& eng_mount "$WORK/w" /w rw \
		&& eng_volmount "x155-$$" /v rw \
		|| { echo "driver inputs could not be staged" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
	eng_pb "$WORK/pb-shim" 120 "$DRIVER" || exit 2
	BIN_RUN="$WORK/pb-shim"
	;;
esac

# rows FILE: the three root-listing verdicts plus the ptmx pair, by name.
# FILE travels through cygpath: jq here is a native Windows program and
# reads an MSYS /c/... path as C:\c\... (measured 2026-09-22, the same
# mangling the 95 script met on its build context).
show_rows() {
	_f="$1"
	if command -v cygpath >/dev/null 2>&1; then
		_f="$(cygpath -w "$1")"
	fi
	jq -c -r '.probes[] | select(.name == "readdir(/)" or .name == "open(/bin, O_RDONLY) by name" or .name == "creat(/, O_CREAT|O_EXCL)" or .name == "stat(/dev/ptmx)" or .name == "open(/dev/ptmx, O_RDWR)") | "\(.name) \(.verdict) errno=\(.errno_name // "-")"' "$_f" 2>/dev/null
}

echo "" >>"$WORK/report"
echo "== 1. the root-listing legs run and report" >>"$WORK/report"
if ! timeout 120 $BIN_RUN probe --json >"$WORK/probe.json" 2>"$WORK/probe.err" || ! show_rows "$WORK/probe.json" >"$WORK/rows" || [ ! -s "$WORK/rows" ]; then
	leg_not_run "probe --json" "no rows; stderr tail:"
	tail -5 "$WORK/probe.err" >>"$WORK/report"
else
	cat "$WORK/rows" >>"$WORK/report"
	for leg in "readdir(/)" "open(/bin, O_RDONLY) by name" "creat(/, O_CREAT|O_EXCL)"; do
		grep -q -F "$leg" "$WORK/rows" || { echo "  MISSING leg: $leg" >>"$WORK/report"; fail=1; }
	done
fi

echo "" >>"$WORK/report"
echo "== 2. the ptmx legs run and report" >>"$WORK/report"
for leg in "stat(/dev/ptmx)" "open(/dev/ptmx, O_RDWR)"; do
	grep -q -F "$leg" "$WORK/rows" 2>/dev/null || { echo "  MISSING leg: $leg" >>"$WORK/report"; fail=1; }
done
echo "  both ptmx rows present above" >>"$WORK/report"

echo "" >>"$WORK/report"
echo "== 3. a denied arm, where the machine stages one" >>"$WORK/report"
# An unprivileged run shows creat refused with its errno on a machine
# that permits everything to root. setpriv may be absent either place;
# that is a note, not a failure: the legs above already ran.
case "$(uname -s)" in
Linux) LANE=native ;;
*) LANE=driver ;;
esac
PRIV=""
if [ "$LANE" = "native" ]; then
	if command -v setpriv >/dev/null 2>&1; then
		PRIV="setpriv --reuid=nobody --regid=nogroup --clear-groups"
	else
		echo "  no setpriv on this host: the denied arm is not staged here" >>"$WORK/report"
	fi
fi
if [ -n "$PRIV" ]; then
	# shellcheck disable=SC2086
	if timeout 120 $PRIV $BIN_RUN probe --json >"$WORK/unpriv.json" 2>/dev/null && show_rows "$WORK/unpriv.json" >"$WORK/urows" && [ -s "$WORK/urows" ]; then
		cat "$WORK/urows" >>"$WORK/report"
	else
		echo "  unprivileged run gave no rows" >>"$WORK/report"
	fi
elif [ "$LANE" = "driver" ]; then
	if eng_run 120 "$DRIVER" "" -- /bin/sh -c 'command -v setpriv >/dev/null 2>&1'; then
		if eng_run 120 "$DRIVER" "" -- /bin/sh -c 'exec setpriv --reuid=nobody --regid=nogroup --clear-groups /pb probe --json' >"$WORK/unpriv.json" 2>/dev/null && show_rows "$WORK/unpriv.json" >"$WORK/urows" && [ -s "$WORK/urows" ]; then
			cat "$WORK/urows" >>"$WORK/report"
		else
			echo "  unprivileged run gave no rows" >>"$WORK/report"
		fi
	else
		echo "  no setpriv in the driver: the denied arm is not staged here" >>"$WORK/report"
	fi
else
	echo "  no setpriv on this host: the denied arm is not staged here" >>"$WORK/report"
fi

cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo ""
echo "written to ${OUT#"$REPO"/}"
[ -s "$WORK/worst" ] && exit 2
[ "$fail" -eq 0 ] || exit 1
exit 0
