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
#
# Pinned input: IMAGE, PODVM_437_IMAGE (busybox:musl of public.ecr.aws by
# default). Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane
# could not run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep437-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
OUT="$REPO/experiments/results/payload-dies-with-launcher.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
IMAGE="${PODVM_437_IMAGE:-public.ecr.aws/docker/library/busybox:musl}"
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

if [ "$fail" -eq 0 ]; then
	note "verdict: THE PAYLOAD ENDS WITH ITS LAUNCHER"
else
	note "verdict: A CLAUSE DISAGREED"
fi
rm -rf "$WORK"
exit "$fail"
