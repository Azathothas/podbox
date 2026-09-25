#!/bin/sh
# Question: does each forced launch rung enter with its word in
# PODBOX_ACTIVE_MODE where its fixture holds, and refuse naming its
# reason where it does not?
#
# TODO/packaging.md T-1003 (rundir and cache, FUSE, tmpfs).
#
# Clauses (each flips from refusal to entry as its rung lands):
#   1. forced rundir over alpine enters with ACTIVE_MODE=rundir.
#   2. forced cache without PODBOX_CACHE refuses naming the opt-in;
#      with PODBOX_CACHE=1 over alpine enters with ACTIVE_MODE=cache.
#   3. forced tmpfs over alpine enters with ACTIVE_MODE=tmpfs where a
#      mount attaches, else refuses naming the mount.
#   4. forced fuse over alpine enters with ACTIVE_MODE=fuse where
#      /dev/fuse opens, else refuses naming the node.
#   5. forced memfd regression: static hello enters on memfd.
#   6. unknown word still refuses with the rung list.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no toolchain, no build, no pull).
set -u

# The job travels as /work/.podbox-job.sh: take the checkout from the
# working directory, never from $0 (see 353 for the measurement).
REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep358-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-358.txt"

ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "input             $ALPINE"
} >"$REPORT"

command -v cargo >/dev/null 2>&1 || { echo "cargo missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
echo "== bootstrap the lane toolchain"
if timeout 1200 ./scripts/common/bootstrap-env.sh rust cc zig tools >>"$WORK/bootstrap.log" 2>&1; then
	echo "bootstrap         ok" >>"$REPORT"
else
	echo "bootstrap         FAILED" | tee -a "$REPORT"
	tail -15 "$WORK/bootstrap.log"
	exit 2
fi
echo "== interposer objects"
if timeout 1200 ./scripts/build-interpose.sh >>"$WORK/interpose.log" 2>&1; then
	echo "interpose         ok" >>"$REPORT"
else
	echo "interpose         FAILED" | tee -a "$REPORT"
	tail -15 "$WORK/interpose.log"
	exit 2
fi
echo "== build the debug binary"
if timeout 1800 cargo build >>"$WORK/build.log" 2>&1; then
	echo "build             ok" >>"$REPORT"
else
	echo "build             FAILED" | tee -a "$REPORT"
	tail -15 "$WORK/build.log"
	exit 1
fi
PB="$REPO/target/x86_64-unknown-linux-musl/debug/podbox"
[ -x "$PB" ] || { echo "binary missing after build: FAILED" | tee -a "$REPORT"; exit 1; }
echo "podbox            $("$PB" version)" >>"$REPORT"

# shellcheck source=../scripts/common/exit-codes.sh
. "$REPO/scripts/common/exit-codes.sh"
podbox_exit_codes "$PB" || { echo "cannot read podbox's exit-code table: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
RUN_ERR=$PODBOX_EXIT_RUNTIME_ERROR

STORE="$WORK/store"
mkdir -p "$STORE" || exit 2
export PODBOX_STORE="$STORE"
fail=0

step() {
	name="$1"; want="$2"; shift 2
	out="$WORK/out-$name.txt"
	timeout 120 "$@" >"$out" 2>&1
	rc=$?
	echo "step $name exit $rc"
	{
	echo ""
	echo "== $name"
	echo "command           $*"
	echo "exit              $rc"
	echo "output:"
	sed 's/^/  /' "$out"
	} >>"$REPORT"
	[ "$rc" -eq "$want" ] || { echo "step $name: wanted exit $want, got $rc" >>"$REPORT"; fail=1; }
}

if ! timeout 600 "$PB" pull "$ALPINE" >>"$REPORT" 2>&1; then
	echo "pull $ALPINE FAILED: COULD NOT RUN" | tee -a "$REPORT"
	exit 2
fi

# Clause 1: forced rundir enters with the rung word.
step forced-rundir 0 env PODBOX_MODE=rundir "$PB" run --rm "$ALPINE" sh -c 'echo $PODBOX_ACTIVE_MODE'
grep -q "^rundir$" "$WORK/out-forced-rundir.txt" || { echo "rundir run did not report rundir" >>"$REPORT"; fail=1; }
if ls "$STORE/runs" >/dev/null 2>&1 && [ -n "$(ls -A "$STORE/runs" 2>/dev/null)" ]; then
	echo "rundir staging    LEAKED files" >>"$REPORT"; fail=1
else
	echo "rundir staging    cleaned" >>"$REPORT"
fi

# Clause 2: cache needs the opt-in; with it, it enters.
step forced-cache-noopt "$RUN_ERR" env PODBOX_MODE=cache "$PB" run --rm "$ALPINE" true
grep -q "PODBOX_CACHE=1" "$WORK/out-forced-cache-noopt.txt" || { echo "cache refusal did not name the opt-in" >>"$REPORT"; fail=1; }
step forced-cache 0 env PODBOX_MODE=cache PODBOX_CACHE=1 "$PB" run --rm "$ALPINE" sh -c 'echo $PODBOX_ACTIVE_MODE'
grep -q "^cache$" "$WORK/out-forced-cache.txt" || { echo "cache run did not report cache" >>"$REPORT"; fail=1; }
if [ -d "$STORE/cache" ] && [ -n "$(ls -A "$STORE/cache" 2>/dev/null)" ]; then
	echo "cache staging    persistent" >>"$REPORT"
else
	echo "cache staging    MISSING after entry" >>"$REPORT"; fail=1
fi

# Clause 3: tmpfs enters where a mount attaches; where the attach
# verdict is down, the refusal names the mount. The lane denies the
# mount (EPERM), so this clause drives the refusal arm; the entry arm
# is wired beside it and runs where a mount holds.
step forced-tmpfs "$RUN_ERR" env PODBOX_MODE=tmpfs "$PB" run --rm "$ALPINE" sh -c 'echo $PODBOX_ACTIVE_MODE'
grep -qi "mount" "$WORK/out-forced-tmpfs.txt" || { echo "tmpfs refusal did not name the mount" >>"$REPORT"; fail=1; }

# Clause 4: fuse enters where /dev/fuse opens (until wired, the node
# refusal names it).
step forced-fuse "$RUN_ERR" env PODBOX_MODE=fuse "$PB" run --rm "$ALPINE" sh -c 'echo $PODBOX_ACTIVE_MODE'
grep -q "dev/fuse" "$WORK/out-forced-fuse.txt" || { echo "fuse refusal did not name the node" >>"$REPORT"; fail=1; }

# Clause 5: memfd regression on the static hello.
cat >"$WORK/hello.c" <<'EOF'
#include <stdio.h>
int main(void) { puts("static-hi"); return 0; }
EOF
mkdir -p "$WORK/static-root" || exit 2
if cc -O2 -Wall -Werror -static-pie -o "$WORK/static-root/hello" "$WORK/hello.c" 2>>"$REPORT"; then
	echo "static hello      static-pie" >>"$REPORT"
else
	echo "static hello      COMPILE FAILED" | tee -a "$REPORT"
	exit 2
fi
(cd "$WORK/static-root" && tar -cf "$WORK/static-hello.tar" hello) 2>>"$REPORT" \
	|| { echo "static tar FAILED" | tee -a "$REPORT"; exit 2; }
if timeout 120 "$PB" import "$WORK/static-hello.tar" static-hello:1 >>"$REPORT" 2>&1; then
	echo "static import     ok" >>"$REPORT"
else
	echo "static import     FAILED" | tee -a "$REPORT"
	exit 1
fi
step forced-memfd 0 env PODBOX_MODE=memfd "$PB" run --rm static-hello:1 /hello
grep -q "static-hi" "$WORK/out-forced-memfd.txt" || { echo "forced memfd payload stdout missing" >>"$REPORT"; fail=1; }

# Clause 6: an unknown word refuses with the rung list.
step bogus-word "$RUN_ERR" env PODBOX_MODE=memfd2 "$PB" run --rm "$ALPINE" true
grep -q "memfd, fuse, tmpfs, rundir, cache" "$WORK/out-bogus-word.txt" || { echo "unknown word did not list the rungs" >>"$REPORT"; fail=1; }

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           LADDER RUNGS HOLD"; else echo "verdict           LADDER RUNGS OPEN"; fi
} >>"$REPORT"

cp "$REPORT" "$REPO/experiments/results/ladder-rungs.txt"
if [ -d /out ]; then cp "$REPORT" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
