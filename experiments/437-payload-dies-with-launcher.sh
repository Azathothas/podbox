#!/bin/sh
# Question: does a detached payload end when its launcher is killed?
#
# TODO/enter.md T-1732. podcrates v0.22.0 (COORD-046, PodUtils/podcrates#71)
# gives pod-enter's `Plan.die_with_parent`: the child sets
# PR_SET_PDEATHSIG(SIGKILL) first after the fork. The launcher of
# `podvm run -d` sets it, so a payload never outlives a killed supervisor
# (podrunner's test 26 found it alive before).
#
# Clauses:
#   1. `run -d IMAGE sleep N`: the payload is a child of the launcher.
#   2. SIGKILL to the launcher: within 5 s the payload is gone or a zombie
#      (state Z); a live payload (state S or R) disagrees.
#   3. `ps -a` names the container Dead, not Up.
#   4. T-1748 (PodUtils/podcrates#124, V-4): SIGKILL to the `podvm` of a
#      foreground `run IMAGE sleep N` on the rung that the host selects:
#      within 5 s the payload is gone or a zombie.
#   5. The same on the loader family of the userland rung: a dynamic image
#      (DYNAMIC) on the chroot-denied fixture of experiment 356 (user and
#      mount namespaces, deny-chroot.c).
#
# Pinned input: IMAGE, PODVM_437_IMAGE (busybox:musl of public.ecr.aws by
# default), and DYNAMIC, PODVM_437_DYNAMIC (debian bookworm-slim). Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane
# could not run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep437-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
OUT="$REPO/experiments/results/payload-dies-with-launcher.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
IMAGE="${PODVM_437_IMAGE:-public.ecr.aws/docker/library/busybox:musl}"
# Clause 5: a dynamic glibc image, so the loader family runs it (the pin of
# experiment 356).
DYNAMIC="${PODVM_437_DYNAMIC:-public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a}"
export PODVM_STORE="$WORK/store"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }
[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "image             $IMAGE"
note ""

# A duration of this run only: the payload's argv is then exactly
# `sleep N`, which the argv of the launcher (`podvm run -d ...`) is not.
n=$((600 + $$ % 1000))
id=$(timeout 300 "$PB" run -d "$IMAGE" sleep "$n" 2>"$WORK/run.err" | tail -1)
[ -n "$id" ] || { sed 's/^/  /' "$WORK/run.err" | tail -3; echo "run -d failed: COULD NOT RUN" >&2; exit 2; }
pp=""
for _ in 1 2 3 4 5 6 7 8 9 10; do
	pp=$(pgrep -x -f "sleep $n" | head -1)
	[ -n "$pp" ] && break
	sleep 0.5
done
[ -n "$pp" ] || { echo "no payload process: COULD NOT RUN" >&2; "$PB" rm -f "$id" >/dev/null 2>&1; exit 2; }
lp=$(ps -o ppid= -p "$pp" | tr -d ' ')

note "== clause 1  the payload is a child of the launcher"
note "  payload $pp, its parent $lp: $(ps -o args= -p "$lp" | sed "s|$REPO|\$REPO|" | cut -c1-60)"
case "$(ps -o args= -p "$lp")" in
*podvm*) ;;
*) bad "the parent of the payload is not podvm's launcher" ;;
esac
note ""

note "== clause 2  SIGKILL to the launcher"
# Never a signal to a process that is not podvm's launcher.
case "$lp:$(ps -o args= -p "$lp")" in
1:* | :*) note "  COULD NOT RUN: the parent is not a launcher"; "$PB" rm -f "$id" >/dev/null 2>&1; exit 2 ;;
*podvm*) kill -9 "$lp" ;;
*) note "  COULD NOT RUN: the parent is not a launcher"; "$PB" rm -f "$id" >/dev/null 2>&1; exit 2 ;;
esac
state=""
for _ in 1 2 3 4 5 6 7 8 9 10; do
	state=$(ps -o stat= -p "$pp" 2>/dev/null | cut -c1)
	case "$state" in "" | Z) break ;; esac
	sleep 0.5
done
note "  the state of the payload after the kill: ${state:-gone}"
case "$state" in
"" | Z) ;;
*) bad "the payload is alive (state $state) after its launcher died"; kill -9 "$pp" 2>/dev/null ;;
esac
note ""

note "== clause 3  ps names the container Dead"
st=$("$PB" ps -a 2>/dev/null | grep "^${id%"${id#????????????}"}" | cut -c1-140)
note "  $st"
case "$st" in
*Dead*) ;;
*) bad "ps -a does not name the container Dead" ;;
esac
"$PB" rm -f "$id" >/dev/null 2>&1
note ""

# Clauses 4 and 5: a foreground run, killed. The pid of the background job
# is podvm itself: the fixture's unshare, sh and deny-chroot each exec.
foreground() {
	clause="$1"; shift
	m=$((2000 + $$ % 1000 + clause))
	"$@" "$PB" run --rm "${FG_IMAGE:-$IMAGE}" sleep "$m" >"$WORK/fg$clause.out" 2>&1 &
	pv=$!
	fp=""
	for _ in $(seq 1 60); do
		# The loader family runs `ld.so --argv0 sleep PATH N`: match the
		# last argument, and never podvm itself.
		fp=""
		for q in $(pgrep -f " $m\$"); do
			case "$q:$(ps -o args= -p "$q")" in
			"$pv":* | *"podvm run"* | *unshare*) ;;
			*) fp=$q; break ;;
			esac
		done
		[ -n "$fp" ] && break
		sleep 0.5
	done
	if [ -z "$fp" ]; then
		note "  COULD NOT RUN: no payload process"; sed 's/^/  /' "$WORK/fg$clause.out" | tail -3 | tee -a "$OUT"
		kill -9 "$pv" 2>/dev/null; return 2
	fi
	case "$(ps -o args= -p "$fp")" in
	*--argv0*) note "  the payload runs through the loader family (--argv0)" ;;
	*) note "  the payload runs by a direct exec" ;;
	esac
	case "$(ps -o args= -p "$pv")" in
	*podvm*) kill -9 "$pv" ;;
	*) note "  COULD NOT RUN: $pv is not podvm"; kill -9 "$fp" 2>/dev/null; return 2 ;;
	esac
	state=""
	for _ in 1 2 3 4 5 6 7 8 9 10; do
		state=$(ps -o stat= -p "$fp" 2>/dev/null | cut -c1)
		case "$state" in "" | Z) break ;; esac
		sleep 0.5
	done
	note "  the payload $fp after SIGKILL to podvm $pv: ${state:-gone}"
	case "$state" in
	"" | Z) ;;
	*) bad "the payload is alive (state $state) after its podvm died"; kill -9 "$fp" 2>/dev/null ;;
	esac
	wait "$pv" 2>/dev/null
	return 0
}

note "== clause 4  SIGKILL to the podvm of a foreground run"
foreground 4 || fail=1
note ""

note "== clause 5  the same on the userland rung (the fixture of 356)"
if cc -O2 -Wall -Werror -static -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>/dev/null &&
	unshare -Urm true 2>/dev/null; then
	printf '#!/bin/sh\nMNT="$1"; shift\nexec "$MNT/deny-chroot" "$@"\n' >"$WORK/wrap.sh"
	timeout 300 "$PB" pull "$DYNAMIC" >/dev/null 2>&1 || { note "  COULD NOT RUN: the pull of $DYNAMIC"; fail=1; }
	FG_IMAGE="$DYNAMIC" foreground 5 unshare -Urm sh "$WORK/wrap.sh" "$WORK" || fail=1
	grep -q "loader family" "$OUT" || bad "clause 5 did not run through the loader family"
else
	note "  COULD NOT RUN: no static cc or no user namespaces"; fail=1
fi
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: THE PAYLOAD ENDS WITH ITS LAUNCHER"
else
	note "verdict: A CLAUSE DISAGREED"
fi
rm -rf "$WORK"
exit "$fail"
