#!/bin/sh
# Question: does `stop`, `rm -f` and the end of a foreground run leave no
# process of the payload's tree, on the chroot rung and on a host that
# denies chroot(2)?
#
# TODO/supervise.md T-1708. The launcher of a detached run is the child
# subreaper of the payload's tree (pod-supervise of podcrates v0.16.0, its
# T-162); a foreground run makes podvm's own process the subreaper. Each
# kills the descendants that stay at the end of the payload.
#
# The pinned input is the alpine digest of experiment 356. The fixture is
# experiments/deny-chroot.c under user and mount namespaces, as in
# experiment 356. Each payload marks its sleeps with a number that no other
# process uses, and the run looks for them with pgrep.
#
# Clauses, each on the capable host and on the fixture:
#   1. the M2 payload `sh -c 'sleep N & sleep N.5 & wait'` under `run -d`,
#      then `stop -t 1`, leaves no process of the tree within 60 s,
#      PODVM_429_ROUNDS times (20 by default); `inspect` reads the end
#      with `.End.Residual` 0.
#   2. a grandchild that calls `setsid` under `run -d`, then `rm -f`,
#      leaves no process.
#   3. a foreground `run` whose payload backgrounds two sleeps and that
#      gets SIGTERM leaves no process.
#   4. (capable host) under an outer subreaper, the launcher's parent is
#      that subreaper.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could not
# run (no build, no pull, no namespaces, no python3).
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep429-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-429.txt"
ROUNDS=${PODVM_429_ROUNDS:-20}

ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
# ⚠ PODVM_429_MIRROR=1 takes the same digest from mirror.gcr.io, as
# PODVM_356_MIRROR does, for a host where public.ecr.aws answers 429.
if [ "${PODVM_429_MIRROR:-0}" = 1 ]; then
	ALPINE='mirror.gcr.io/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
fi

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "input             $ALPINE"
echo "rounds            $ROUNDS"
} >"$REPORT"

command -v cargo >/dev/null 2>&1 || { echo "cargo missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
command -v pgrep >/dev/null 2>&1 || { echo "pgrep missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "python3 missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
unshare -Urm true >/dev/null 2>&1 || { echo "user+mount namespaces refused: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
if timeout 1200 ./scripts/build-interpose.sh >"$WORK/interpose.log" 2>&1 &&
	timeout 1800 cargo build >"$WORK/build.log" 2>&1; then
	echo "build             ok" >>"$REPORT"
else
	echo "build             FAILED" | tee -a "$REPORT"; tail -15 "$WORK/build.log"; exit 2
fi
PB="$REPO/target/x86_64-unknown-linux-musl/debug/podvm"
echo "podvm             $("$PB" version)" >>"$REPORT"
export PODVM_STORE="$WORK/store"
timeout 600 "$PB" pull "$ALPINE" >>"$REPORT" 2>&1 || { echo "pull FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }

cc -O2 -Wall -Werror -static -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>>"$REPORT" ||
	cc -O2 -Wall -Werror -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>>"$REPORT" ||
	{ echo "fixture COMPILE FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
cat >"$WORK/wrap.sh" <<'EOF'
#!/bin/sh
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF
fail=0

# Wait up to 60 s until no process names the marker; 0 where none stays.
gone() {
	i=0
	while [ "$i" -lt 60 ]; do
		pgrep -f "sleep $1" >/dev/null 2>&1 || return 0
		sleep 1; i=$((i + 1))
	done
	return 1
}

note() { echo "$*" >>"$REPORT"; }

# One rung: $1 the label, the rest the prefix that runs podvm on it.
clauses() {
	label=$1; shift
	ok=0
	r=0
	while [ "$r" -lt "$ROUNDS" ]; do
		r=$((r + 1))
		m="42$r$$"
		name="t429-$label-$r"
		"$@" "$PB" run -d --name "$name" "$ALPINE" sh -c "sleep $m & sleep $m.5 & wait" >/dev/null 2>>"$WORK/err-$label.txt"
		sleep 1
		pgrep -f "sleep $m" >/dev/null 2>&1 || { note "$label round $r: the payload did not start"; continue; }
		"$@" "$PB" stop -t 1 "$name" >/dev/null 2>>"$WORK/err-$label.txt"
		if gone "$m"; then ok=$((ok + 1)); else note "$label round $r: a process of the tree stayed after stop"; pgrep -fa "sleep $m" >>"$REPORT"; pkill -9 -f "sleep $m"; fi
		[ "$r" -eq 1 ] && note "$label inspect     $("$@" "$PB" inspect --format '{{.State}} {{.End.Reason}} residual={{.End.Residual}}' "$name" 2>&1)"
		"$@" "$PB" rm "$name" >/dev/null 2>&1
	done
	note "$label stop        $ok of $ROUNDS left no process"
	[ "$ok" -eq "$ROUNDS" ] || fail=1
	"$@" "$PB" inspect --format '{{.End.Residual}}' "t429-$label-1" >/dev/null 2>&1 && { note "$label: rm left a record"; fail=1; }

	m="43$$"
	"$@" "$PB" run -d --name "t429-$label-setsid" "$ALPINE" sh -c "setsid sleep $m & wait" >/dev/null 2>>"$WORK/err-$label.txt"
	sleep 1
	"$@" "$PB" rm -f "t429-$label-setsid" >/dev/null 2>>"$WORK/err-$label.txt"
	if gone "$m"; then note "$label setsid rm -f  no process"; else note "$label setsid rm -f  a process STAYED"; pkill -9 -f "sleep $m"; fail=1; fi

	m="44$$"
	"$@" "$PB" run --rm "$ALPINE" sh -c "sleep $m & sleep $m.5 & wait" >/dev/null 2>>"$WORK/err-$label.txt" &
	fg=$!
	sleep 2
	# The podvm of the foreground run: the newest process with the marker
	# in its argv that is podvm itself.
	pv=$(pgrep -n -f "podvm run --rm .*sleep $m") || pv=""
	if [ -n "$pv" ]; then kill -TERM "$pv"; else note "$label foreground: podvm not found"; fail=1; fi
	wait "$fg" 2>/dev/null
	if gone "$m"; then note "$label foreground SIGTERM  no process"; else note "$label foreground SIGTERM  a process STAYED"; pkill -9 -f "sleep $m"; fail=1; fi
}

echo "== capable host"
"$PB" system info --format '{{.EnteredRung}}' >"$WORK/rung-capable.txt" 2>&1
note "capable rung      $(cat "$WORK/rung-capable.txt")"
clauses capable env

echo "== fixture"
FX="unshare -Urm sh $WORK/wrap.sh $WORK"
$FX "$PB" system info --format '{{.EnteredRung}}' >"$WORK/rung-fixture.txt" 2>&1
note "fixture rung      $(cat "$WORK/rung-fixture.txt")"
grep -qx userland "$WORK/rung-fixture.txt" || { note "the fixture does not deny chroot"; fail=1; }
clauses fixture $FX

echo "== outer subreaper"
cat >"$WORK/outer.py" <<'EOF'
import ctypes, json, os, subprocess, sys
pb, image = sys.argv[1], sys.argv[2]
libc = ctypes.CDLL(None, use_errno=True)
if libc.prctl(36, 1, 0, 0, 0) != 0:  # PR_SET_CHILD_SUBREAPER
    print("prctl refused"); sys.exit(2)
subprocess.run([pb, "run", "-d", "--name", "t429-outer", image, "sleep", "4545"], check=True, stdout=subprocess.DEVNULL)
lp = subprocess.run([pb, "inspect", "--format", "{{.LauncherPid}}", "t429-outer"], capture_output=True, text=True).stdout.strip()
with open(f"/proc/{lp}/stat") as f:
    ppid = int(f.read().rsplit(")", 1)[1].split()[1])
print(f"launcher {lp} parent {ppid} outer {os.getpid()}")
subprocess.run([pb, "rm", "-f", "t429-outer"], stdout=subprocess.DEVNULL)
sys.exit(0 if ppid == os.getpid() else 1)
EOF
if python3 -I "$WORK/outer.py" "$PB" "$ALPINE" >>"$REPORT" 2>&1; then
	note "outer subreaper   the launcher's parent is the outer subreaper"
else
	note "outer subreaper   DISAGREED"; fail=1
fi

{
echo ""
echo "== errors of the runs"
cat "$WORK"/err-*.txt 2>/dev/null | grep -v "Extracted\|cannot apply\|restore ownership\|no mapping\|not mapped\|intended" | head -40
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           THE TREE ENDS"; else echo "verdict           A PROCESS STAYED"; fi
} >>"$REPORT"

cp "$REPORT" "$REPO/experiments/results/tree-kill.txt"
cat "$REPORT"
[ "$fail" -eq 0 ]
