#!/bin/sh
# Question: does each flag of sand-net's hardening line work on the rung
# that podvm enters, or refuse by name, on a host that denies chroot(2)?
#
# TODO/cli.md T-1655. The line (issue 167):
#   run --rm --cap-drop ALL --security-opt no-new-privileges --read-only
#     --tmpfs /tmp:rw,nosuid,nodev,size=N --tmpfs /dev/shm:rw,nosuid,nodev,size=N
#     --ulimit fsize=N -v PROJECT:/workspace -v STATE:/state
#     -w /workspace --network none --userns keep-id --entrypoint CMD IMAGE ARGS
#
# The fixture is experiments/deny-chroot.c under user and mount namespaces,
# as in experiment 356; podvm enters the userland loader family there. The
# pinned input is the alpine digest of experiment 356.
#
# Clauses:
#   1. each flag alone: it runs `true` at 0, or it refuses at 125 and the
#      message names the flag or its row ("status None", or its reason).
#   2. the line without the flags that refuse (--network, --userns): a
#      write to / fails with EROFS, /workspace and /tmp take writes, a
#      write past --ulimit fsize fails with EFBIG, and the payload's
#      NoNewPrivs is 1.
#   3. --user 993:990: `id -u` prints 993 and the payload writes the
#      project.
#   4. --memory 128m refuses by name; --ulimit nofile=64 shows in
#      `ulimit -n`.
#   5. --timeout 2 on `sh -c 'sleep N & wait'` ends the whole tree, each
#      child gone, within 60 s.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could not
# run (no build, no pull, no namespaces).
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep411-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-411.txt"
ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
# ⚠ PODVM_411_MIRROR=1 takes the same digest from mirror.gcr.io, as
# PODVM_356_MIRROR does.
if [ "${PODVM_411_MIRROR:-0}" = 1 ]; then
	ALPINE='mirror.gcr.io/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
fi
{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "input             $ALPINE"
} >"$REPORT"

command -v cargo >/dev/null 2>&1 || { echo "cargo missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
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
	{ echo "fixture COMPILE FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
cat >"$WORK/wrap.sh" <<'EOF'
#!/bin/sh
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF
FX="unshare -Urm sh $WORK/wrap.sh $WORK"
PROJECT="$WORK/project"; STATE="$WORK/state"
mkdir -p "$PROJECT" "$STATE" || exit 2
fail=0
note() { echo "$*" >>"$REPORT"; }

step() {
	name="$1"; want="$2"; shift 2
	out="$WORK/out-$name.txt"
	timeout 120 "$@" >"$out" 2>&1
	rc=$?
	{
	echo ""
	echo "== $name"
	echo "exit              $rc"
	grep -v "Extracted\|cannot apply\|restore ownership\|no mapping\|not mapped\|intended metadata" "$out" | sed 's/^/  /'
	} >>"$REPORT"
	[ "$rc" -eq "$want" ] || { note "step $name: wanted exit $want, got $rc"; fail=1; }
}

# Clause 1: each flag alone.
for spec in "--cap-drop ALL" "--security-opt no-new-privileges" "--read-only" \
	"--tmpfs /tmp:rw,nosuid,nodev,size=64m" "--tmpfs /dev/shm:rw,nosuid,nodev,size=64m" \
	"--ulimit fsize=1048576" "-v $PROJECT:/workspace" "-v $STATE:/state" \
	"-w /tmp" "--network none" "--userns keep-id" "--entrypoint /bin/true"; do
	flag=${spec%% *}
	out="$WORK/flag.txt"
	# shellcheck disable=SC2086
	timeout 120 $FX "$PB" run --rm $spec "$ALPINE" true >"$out" 2>&1
	rc=$?
	if [ "$rc" -eq 0 ]; then
		note "flag $spec: works"
	elif [ "$rc" -eq 125 ] && grep -q -- "$flag" "$out"; then
		note "flag $spec: refused by name: $(grep -- "$flag" "$out" | head -1 | cut -c1-160)"
	else
		note "flag $spec: exit $rc, NOT named"; sed 's/^/  /' "$out" >>"$REPORT"; fail=1
	fi
done

# Clause 2: the line without the flags that refuse.
step line 0 $FX "$PB" run --rm --cap-drop ALL --security-opt no-new-privileges --read-only \
	--tmpfs /tmp:rw,nosuid,nodev,size=64m --tmpfs /dev/shm:rw,nosuid,nodev,size=64m \
	--ulimit fsize=65536 -v "$PROJECT:/workspace" -v "$STATE:/state" -w /workspace \
	--entrypoint sh "$ALPINE" -c 'touch /ro411 2>&1; echo p > /workspace/p411 && echo ws-ok; echo t > /tmp/t411 && echo tmp-ok; trap "" XFSZ; dd if=/dev/zero of=/tmp/big bs=4096 count=64 2>&1; echo done'
grep -q "Read-only file system" "$WORK/out-line.txt" || { note "line: a write to / did not fail with EROFS"; fail=1; }
grep -qx "ws-ok" "$WORK/out-line.txt" || { note "line: /workspace did not take a write"; fail=1; }
grep -qx "tmp-ok" "$WORK/out-line.txt" || { note "line: /tmp did not take a write"; fail=1; }
grep -q "File too large" "$WORK/out-line.txt" || { note "line: a write past fsize did not fail with EFBIG"; fail=1; }
grep -qx "p" "$PROJECT/p411" 2>/dev/null || { note "line: the project write did not land on the host"; fail=1; }
# NoNewPrivs of the payload, read from the host: the capable rung (the
# fixture sets the bit for its own filter, so it tells nothing there).
"$PB" run -d --name t411nnp --security-opt no-new-privileges "$ALPINE" sleep 4111 >/dev/null 2>&1
sleep 1
nnp=""
for p in $(pgrep -f "sleep 4111"); do
	[ "$(cat "/proc/$p/comm" 2>/dev/null)" = sleep ] && nnp=$(awk '/NoNewPrivs/ {print $2}' "/proc/$p/status")
done
"$PB" rm -f t411nnp >/dev/null 2>&1
note "capable NoNewPrivs $nnp"
[ "$nnp" = 1 ] || { note "the payload's NoNewPrivs is not 1"; fail=1; }

# Clause 3: --user 993:990.
step user 0 $FX "$PB" run --rm --user 993:990 -v "$PROJECT:/workspace" "$ALPINE" sh -c 'id -u; echo u > /workspace/u411 && echo wrote'
grep -qx "993" "$WORK/out-user.txt" || { note "user: id -u did not print 993"; fail=1; }
grep -qx "wrote" "$WORK/out-user.txt" || { note "user: the project write failed"; fail=1; }

# Clause 4: --memory refuses by name; --ulimit nofile shows.
step memory 125 $FX "$PB" run --rm --memory 128m "$ALPINE" true
grep -q -- "--memory\|-m, --memory" "$WORK/out-memory.txt" || { note "memory: not refused by name"; fail=1; }
step nofile 0 $FX "$PB" run --rm --ulimit nofile=64 "$ALPINE" sh -c 'ulimit -n'
grep -qx "64" "$WORK/out-nofile.txt" || { note "nofile: ulimit -n is not 64"; fail=1; }

# Clause 5: --timeout ends the tree.
m="5$$"
start=$(date +%s)
step timeout 137 $FX "$PB" run --rm --timeout 2 "$ALPINE" sh -c "sleep $m & wait"
took=$(( $(date +%s) - start ))
note "timeout took ${took} s"
i=0; while [ "$i" -lt 60 ] && pgrep -f "sleep $m" >/dev/null 2>&1; do sleep 1; i=$((i + 1)); done
if pgrep -f "sleep $m" >/dev/null 2>&1; then note "timeout: a child stayed"; pkill -9 -f "sleep $m"; fail=1; fi
[ "$took" -le 60 ] || { note "timeout: the run took over 60 s"; fail=1; }

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           THE LINE WORKS OR REFUSES BY NAME"; else echo "verdict           A CLAUSE DISAGREED"; fi
} >>"$REPORT"
cp "$REPORT" "$REPO/experiments/results/hardening-line.txt"
cat "$REPORT"
[ "$fail" -eq 0 ]
