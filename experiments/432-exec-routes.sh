#!/bin/sh
# Question: does a child that the payload starts through system(), popen()
# or fexecve() enter through the image's loader on a host that denies
# chroot(2)?
#
# TODO/interpose.md T-1703. glibc and musl start /bin/sh for system() and
# popen() through posix_spawn inside libc, and fexecve() is execveat()
# inside libc: both pass the preload, so the kernel read the loader on the
# host root. The interposer now exports the four names and routes each one.
#
# The fixture is experiments/deny-chroot.c under user and mount namespaces,
# as in experiment 356; podvm enters the userland loader family there. The
# probe experiments/exec-routes.c is a dynamic musl program, built with zig
# cc, that the payload runs from a volume. The host has no
# /lib/ld-musl-x86_64.so.1, so a child that the kernel starts on the host
# root fails there. The pinned input is the alpine digest of experiment 356.
#
# Clauses:
#   1. system() runs the image's /bin/sh: it prints sys-42 (the shell sees
#      /etc/alpine-release), and the status is 3.
#   2. popen() reads popen-42 from the image's shell, and pclose() gives 0.
#   3. fexecve() of a descriptor of /bin/busybox prints fexecve-ok.
#   4. fexecve() of a memfd copy of /bin/busybox prints memfd-ok.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could not
# run (no build, no pull, no namespaces, no zig).
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep432-work"
rm -rf "$WORK"; mkdir -p "$WORK/probe" || exit 2
REPORT="$WORK/out-432.txt"
ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
# ⚠ PODVM_432_MIRROR=1 takes the same digest from mirror.gcr.io, as
# PODVM_356_MIRROR does.
if [ "${PODVM_432_MIRROR:-0}" = 1 ]; then
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
command -v zig >/dev/null 2>&1 || { echo "zig missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
unshare -Urm true >/dev/null 2>&1 || { echo "user+mount namespaces refused: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
if timeout 1200 ./scripts/build-interpose.sh >"$WORK/interpose.log" 2>&1 &&
	timeout 1800 cargo build >"$WORK/build.log" 2>&1; then
	echo "build             ok" >>"$REPORT"
else
	echo "build             FAILED" | tee -a "$REPORT"; tail -15 "$WORK/build.log"; exit 2
fi
zig cc -target x86_64-linux-musl -dynamic -O2 -Wall -Werror -o "$WORK/probe/exec-routes" \
	"$REPO/experiments/exec-routes.c" 2>>"$REPORT" ||
	{ echo "probe COMPILE FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
PB="$REPO/target/x86_64-unknown-linux-musl/debug/podvm"
echo "podvm             $("$PB" version)" >>"$REPORT"
echo "host musl loader  $([ -e /lib/ld-musl-x86_64.so.1 ] && echo present || echo absent)" >>"$REPORT"
export PODVM_STORE="$WORK/store"
timeout 600 "$PB" pull "$ALPINE" >>"$REPORT" 2>&1 || { echo "pull FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
cc -O2 -Wall -Werror -static -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>>"$REPORT" ||
	cc -O2 -Wall -Werror -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>>"$REPORT" ||
	{ echo "fixture COMPILE FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
cat >"$WORK/wrap.sh" <<'EOS'
#!/bin/sh
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOS
FX="unshare -Urm sh $WORK/wrap.sh $WORK"
$FX "$PB" system info --format '{{.EnteredRung}}' >"$WORK/rung.txt" 2>&1
echo "fixture rung      $(cat "$WORK/rung.txt")" >>"$REPORT"
fail=0
grep -qx userland "$WORK/rung.txt" || { echo "the fixture does not deny chroot" >>"$REPORT"; fail=1; }
note() { echo "$*" >>"$REPORT"; }

# One clause: $1 the route, $2 the line it must print.
route() {
	out="$WORK/out-$1.txt"
	timeout 120 $FX "$PB" run --rm -v "$WORK/probe:/probe" "$ALPINE" /probe/exec-routes "$1" >"$out" 2>&1
	rc=$?
	{
	echo ""
	echo "== $1"
	echo "exit              $rc"
	grep -v "Extracted\|cannot apply\|restore ownership\|no mapping\|not mapped\|intended metadata" "$out" | sed 's/^/  /'
	} >>"$REPORT"
	[ "$rc" -eq 0 ] || { note "$1: wanted exit 0, got $rc"; fail=1; }
	grep -qx "$2" "$out" || { note "$1: no line $2"; fail=1; }
}

route sys "sys-42"
grep -qx "system-status 3" "$WORK/out-sys.txt" || { note "sys: the status is not 3"; fail=1; }
route popen "popen-42"
grep -qx "pclose-status 0" "$WORK/out-popen.txt" || { note "popen: pclose is not 0"; fail=1; }
route fexecve "fexecve-ok"
route memfd "memfd-ok"

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           EACH CHILD ENTERS THROUGH THE IMAGE"; else echo "verdict           A CLAUSE DISAGREED"; fi
} >>"$REPORT"
cp "$REPORT" "$REPO/experiments/results/exec-routes.txt"
cat "$REPORT"
[ "$fail" -eq 0 ]
