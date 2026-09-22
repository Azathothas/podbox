#!/bin/sh
# Question: does the CLI read PODBOX_MODE, feed the ladder, and drive the
# forced rung, while the default entry stays the chroot by path?
#
# TODO/packaging.md T-1003 (the CLI wiring and the Prove drive). The binary
# is `$PODBOX_BIN`, a lane-built podbox. A non-native lane stages it through
# the M5 debian driver, the experiments/240 pattern: one driver call runs
# all seven clauses against a container-local store, and the conditions
# block says so.
#
#   ./163-ladder-drive.sh
#
# Exit: 0 every clause held, 1 one did not, 2 a leg could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/ladder-drive.txt"
WORK="$REPO/experiments/.sweep163-work"
rm -rf "$WORK"; mkdir -p "$WORK" "$WORK/ladder" || exit 2
trap 'eng_cleanup 2>/dev/null; rm -rf "$WORK"' EXIT INT TERM

ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
DEBIAN='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
DRIVER='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'

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
Linux) echo "podbox            $($BIN version 2>/dev/null || echo MISSING)" ;;
*) if [ -r "$BIN" ]; then echo "podbox            lane-built artifact, runs in the driver below"; else echo "podbox            MISSING ($BIN unreadable)"; fi ;;
esac
echo "alpine            $ALPINE"
echo "debian            $DEBIAN"
} >"$WORK/report"

command -v jq >/dev/null 2>&1 || { echo "jq is missing" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
# shellcheck source=lib/engine.sh
. "$HERE/lib/engine.sh"
export ENGINE_REPO ENGINE_WORK
ENGINE_REPO="$REPO"
ENGINE_WORK="$WORK"
if engine_pick; then HAVE_ENGINE=1; else HAVE_ENGINE=0; fi
[ "$HAVE_ENGINE" -eq 1 ] || { echo "engine            none answered" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }

case "$(uname -s)" in
Linux) LANE=native ;;
*) LANE=driver ;;
esac

# Seven clauses, one runner per lane. Each clause writes out/err/rc; the
# assertions below read those, so both lanes share them.
if [ "$LANE" = "native" ]; then
	[ -x "$BIN" ] || { echo "binary $BIN is not executable" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
	export PODBOX_STORE="$WORK/store163"
	rm -rf "$PODBOX_STORE"
	RUN="$BIN"
	if ! timeout 300 "$RUN" pull "$ALPINE" >"$WORK/ladder/pull-alpine.log" 2>&1 \
	|| ! timeout 300 "$RUN" pull "$DEBIAN" >"$WORK/ladder/pull-debian.log" 2>&1; then
		leg_not_run "pull" "see $WORK/ladder/pull-*.log"
	fi
	# The static payload the memfd rung needs; see the driver block below
	# for why alpine's own busybox does not qualify.
	if [ ! -s "$WORK/worst" ]; then
		timeout 600 "$RUN" run --rm "$DEBIAN" /bin/sh -c 'apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq busybox-static' >"$WORK/ladder/setup.log" 2>&1 \
			|| leg_not_run "busybox-static setup" "see $WORK/ladder/setup.log"
	fi
	c() {
		n="$1"; shift
		timeout 300 "$RUN" "$@" >"$WORK/ladder/$n.out" 2>"$WORK/ladder/$n.err"
		echo "$?" >"$WORK/ladder/$n.rc"
	}
	[ -s "$WORK/worst" ] || PODBOX_MODE= c c1 run --rm "$ALPINE" /bin/sh -c 'echo $PODBOX_ACTIVE_MODE'
	[ -s "$WORK/worst" ] || PODBOX_MODE=memfd c c1m run --rm "$DEBIAN" /bin/busybox-static sh -c 'echo $PODBOX_ACTIVE_MODE'
	[ -s "$WORK/worst" ] || PODBOX_MODE= c c2 run --rm "$ALPINE" /bin/sh -c 'test -z "$PODBOX_MODE" && test -n "$PODBOX_ACTIVE_MODE" && echo $PODBOX_ACTIVE_MODE'
	[ -s "$WORK/worst" ] || PODBOX_MODE= c c3 run --rm -e PODBOX_MODE=memfd "$ALPINE" /bin/sh -c 'test -z "$PODBOX_MODE" && echo $PODBOX_ACTIVE_MODE'
	[ -s "$WORK/worst" ] || PODBOX_MODE=fuse c c4 run --rm "$ALPINE" /bin/sh -c 'echo unreachable'
	[ -s "$WORK/worst" ] || PODBOX_MODE=memfd2 c c5 run --rm "$ALPINE" /bin/sh -c 'echo unreachable'
	[ -s "$WORK/worst" ] || PODBOX_MODE=memfd c c6 exec "$ALPINE" /bin/sh -c 'echo unreachable'
	[ -s "$WORK/worst" ] || PODBOX_MODE=memfd c c7 run --rm "$DEBIAN" /bin/true
else
	[ -r "$BIN" ] || { echo "binary $BIN is not readable (set PODBOX_BIN to a guest build artifact)" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
	STAGE="$WORK/stage"
	eng_pull "$DRIVER" || exit 2
	mkdir -p "$STAGE" "$WORK/w" || exit 2
	cp "$BIN" "$STAGE/podbox" || exit 2
	eng_mount "$STAGE/podbox" /pb \
		&& eng_mount "$WORK/w" /w rw \
		&& eng_volmount "x163-$$" /v rw \
		|| { echo "driver inputs could not be staged" >>"$WORK/report"; echo "verdict           COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
	if [ ! -f "$WORK/w/cacert.pem" ]; then
		eng_run 600 "$DRIVER" "" -- /bin/sh -c 'DEBIAN_FRONTEND=noninteractive apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq ca-certificates && cp /etc/ssl/certs/ca-certificates.crt /w/cacert.pem && test -s /w/cacert.pem' \
			>/dev/null 2>"$WORK/ca-install163.log" || leg_not_run "CA provision" "see $WORK/ca-install163.log"
	fi
	cat >"$WORK/row163.sh" <<'ROW163_EOF'
#!/bin/sh
# All seven ladder clauses in one driver call, against one container-local
# store. Stdout carries only stage failures; every reading lands in /w/lad/.
set -u
export PODBOX_STORE=/tmp/ostore163
rm -rf /tmp/ostore163
if [ -f /w/cacert.pem ]; then
	export SSL_CERT_FILE=/w/cacert.pem
fi
d=/w/lad
mkdir -p "$d" || exit 6
rm -f "$d"/*.out "$d"/*.err "$d"/*.rc
ALPINE="$1"
DEBIAN="$2"
/pb pull "$ALPINE" >"$d/pull-alpine.log" 2>&1 || exit 3
/pb pull "$DEBIAN" >"$d/pull-debian.log" 2>&1 || exit 3
# The static payload the memfd rung needs. Alpine's own busybox is
# dynamically linked (PT_INTERP /lib/ld-musl-x86_64.so.1, measured
# 2026-09-22), so the rung correctly refuses it; debian's busybox-static
# is the static one, installed here once for the clauses below.
timeout 600 /pb run --rm "$DEBIAN" /bin/sh -c 'apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq busybox-static' >"$d/setup.log" 2>&1 || exit 4
c() {
	n="$1"; shift
	timeout 300 /pb "$@" >"$d/$n.out" 2>"$d/$n.err"
	echo "$?" >"$d/$n.rc"
}
PODBOX_MODE= c c1 run --rm "$ALPINE" /bin/sh -c 'echo $PODBOX_ACTIVE_MODE'
PODBOX_MODE=memfd c c1m run --rm "$DEBIAN" /bin/busybox-static sh -c 'echo $PODBOX_ACTIVE_MODE'
PODBOX_MODE= c c2 run --rm "$ALPINE" /bin/sh -c 'test -z "$PODBOX_MODE" && test -n "$PODBOX_ACTIVE_MODE" && echo $PODBOX_ACTIVE_MODE'
PODBOX_MODE= c c3 run --rm -e PODBOX_MODE=memfd "$ALPINE" /bin/sh -c 'test -z "$PODBOX_MODE" && echo $PODBOX_ACTIVE_MODE'
PODBOX_MODE=fuse c c4 run --rm "$ALPINE" /bin/sh -c 'echo unreachable'
PODBOX_MODE=memfd2 c c5 run --rm "$ALPINE" /bin/sh -c 'echo unreachable'
PODBOX_MODE=memfd c c6 exec "$ALPINE" /bin/sh -c 'echo unreachable'
PODBOX_MODE=memfd c c7 run --rm "$DEBIAN" /bin/true
exit 0
ROW163_EOF
	if [ ! -s "$WORK/worst" ]; then
		eng_mount "$WORK/row163.sh" /drv/row163.sh || leg_not_run "stage row163.sh" "mount refused"
	fi
	if [ ! -s "$WORK/worst" ]; then
		if eng_run 900 "$DRIVER" "" -- /bin/sh /drv/row163.sh "$ALPINE" "$DEBIAN" 2>"$WORK/row163.stage"; then
			for n in c1 c1m c2 c3 c4 c5 c6 c7; do
				cp "$WORK/w/lad/$n.out" "$WORK/ladder/$n.out" && cp "$WORK/w/lad/$n.err" "$WORK/ladder/$n.err" && cp "$WORK/w/lad/$n.rc" "$WORK/ladder/$n.rc" || leg_not_run "read back $n" "copy failed"
			done
		else
			leg_not_run "row163 driver" "see $WORK/row163.stage"
		fi
	fi
fi

# The assertions, shared by both lanes.
rc_of() { cat "$WORK/ladder/$1.rc"; }
check() {
	name="$1"; want_rc="$2"; want_out="$3"; want_err="$4"
	if [ ! -f "$WORK/ladder/$name.rc" ]; then
		echo "  $name: MISSING (driver gave no readings)" >>"$WORK/report"; fail=1; return
	fi
	got_rc="$(rc_of "$name")"
	if [ "$got_rc" = "$want_rc" ]; then
		echo "  $name: rc=$got_rc" >>"$WORK/report"
	else
		echo "  $name: FAIL rc=$got_rc, want $want_rc" >>"$WORK/report"; fail=1
	fi
	if [ -n "$want_out" ]; then
		if grep -q -F "$want_out" "$WORK/ladder/$name.out" 2>/dev/null; then
			echo "  $name: stdout names $(printf '%s' "$want_out" | head -c 48)" >>"$WORK/report"
		else
			echo "  $name: FAIL stdout names no $want_out" >>"$WORK/report"; fail=1
		fi
	fi
	if [ -n "$want_err" ]; then
		if grep -q -F "$want_err" "$WORK/ladder/$name.err" 2>/dev/null; then
			echo "  $name: stderr names $(printf '%s' "$want_err" | head -c 48)" >>"$WORK/report"
		else
			echo "  $name: FAIL stderr names no $want_err" >>"$WORK/report"; fail=1
		fi
	fi
}

echo "" >>"$WORK/report"
echo "== 1. a forced memfd over a static payload enters on the memfd rung" >>"$WORK/report"
echo "   (busybox-static on debian: alpine's own busybox is dynamically" >>"$WORK/report"
echo "   linked and the rung correctly refuses it; clause 7 drives that arm)" >>"$WORK/report"
[ -s "$WORK/worst" ] || check c1m 0 "memfd" "entering on the memfd rung"

echo "" >>"$WORK/report"
echo "== 2. the default entry reports chroot and hands no directive down" >>"$WORK/report"
[ -s "$WORK/worst" ] || check c2 0 "chroot" ""

echo "" >>"$WORK/report"
echo "== 3. a declared PODBOX_MODE is scrubbed before the payload" >>"$WORK/report"
[ -s "$WORK/worst" ] || check c3 0 "chroot" ""

echo "" >>"$WORK/report"
echo "== 4. a forced rung this runtime lacks refuses naming it" >>"$WORK/report"
[ -s "$WORK/worst" ] || check c4 125 "" "/dev/fuse"

echo "" >>"$WORK/report"
echo "== 5. an unknown word refuses with the rung list" >>"$WORK/report"
[ -s "$WORK/worst" ] || check c5 125 "" "names no launch rung"

echo "" >>"$WORK/report"
echo "== 6. exec never reaches the ladder and refuses the force" >>"$WORK/report"
[ -s "$WORK/worst" ] || check c6 125 "" "does not drive"

echo "" >>"$WORK/report"
echo "== 7. a forced memfd over a dynamic payload refuses naming the loader" >>"$WORK/report"
[ -s "$WORK/worst" ] || check c7 125 "" "dynamically linked"

echo "" >>"$WORK/report"
echo "== 0. the default entry without any force (control)" >>"$WORK/report"
[ -s "$WORK/worst" ] || check c1 0 "chroot" ""

cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
# The raw clause outputs, one file per clause, so a red clause names its
# refusal without a re-drive: the container is removed when it exits.
mkdir -p "$REPO/experiments/results/sweep163"
rm -f "$REPO/experiments/results/sweep163"/c*.out "$REPO/experiments/results/sweep163"/c*.err "$REPO/experiments/results/sweep163"/c*.rc
cp "$WORK/ladder"/c*.out "$WORK/ladder"/c*.err "$WORK/ladder"/c*.rc "$REPO/experiments/results/sweep163/" 2>/dev/null || true
echo ""
echo "written to ${OUT#"$REPO"/}"
[ -s "$WORK/worst" ] && exit 2
[ "$fail" -eq 0 ] || exit 1
exit 0
