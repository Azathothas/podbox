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
#   3. forced tmpfs attempts entry: where a mount attaches it enters
#      with ACTIVE_MODE=tmpfs, payload bytes and exit 0; where the
#      attach verdict is down it refuses naming the mount. Either arm
#      leaves no staging behind.
#   4. forced fuse attempts entry: where /dev/fuse opens and a mount
#      attaches it enters with ACTIVE_MODE=fuse, payload bytes and
#      exit 0; elsewhere it refuses naming the node or the mount.
#      Either arm leaves no staging behind.
#   5. forced memfd regression: static hello enters on memfd.
#   6. unknown word still refuses with the rung list.
#   7. the packed rootfs without a registry: save writes one
#      OCI-layout tarball, load stages it into a fresh store hashing
#      every blob, a forced run over the loaded record enters with the
#      rung word and the payload bytes, and a flipped layer byte
#      refuses as a digest mismatch.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no toolchain, no build, no pull).
set -u

# The Windows wrapper runs the job at /in/job.sh with /work as the
# working directory. Take the checkout from that directory, not $0.
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

# Clause 3: forced tmpfs attempts entry. Where a mount attaches, the
# run enters with ACTIVE_MODE=tmpfs, the payload bytes on stdout, exit
# 0 and no staging left behind; where the attach verdict is down, the
# refusal names the mount and likewise leaves nothing. This lane
# denies the mount (mount(2) EPERM, even in a user namespace, measured
# 2026-09-30), so the refusal arm is the live one here and entry stays
# unproved until a mount-granting host runs this script.
out="$WORK/out-forced-tmpfs.txt"
timeout 120 env PODBOX_MODE=tmpfs "$PB" run --rm "$ALPINE" sh -c 'echo $PODBOX_ACTIVE_MODE; cat /etc/alpine-release' >"$out" 2>&1
rc=$?
echo "step forced-tmpfs exit $rc"
{
echo ""
echo "== forced-tmpfs"
echo "command           env PODBOX_MODE=tmpfs $PB run --rm $ALPINE sh -c 'echo \$PODBOX_ACTIVE_MODE; cat /etc/alpine-release'"
echo "exit              $rc"
echo "output:"
sed 's/^/  /' "$out"
} >>"$REPORT"
if [ "$rc" -eq 0 ]; then
	echo "tmpfs arm         ENTRY (a mount attaches on this machine)" >>"$REPORT"
	grep -q "^tmpfs$" "$out" || { echo "tmpfs entry did not report tmpfs" >>"$REPORT"; fail=1; }
	grep -q "3\.20" "$out" || { echo "tmpfs entry payload bytes missing" >>"$REPORT"; fail=1; }
else
	[ "$rc" -eq "$RUN_ERR" ] || { echo "tmpfs refusal exit $rc, wanted $RUN_ERR" >>"$REPORT"; fail=1; }
	echo "tmpfs arm         REFUSAL (no mount attaches on this machine)" >>"$REPORT"
	grep -qi "mount" "$out" || { echo "tmpfs refusal did not name the mount" >>"$REPORT"; fail=1; }
fi
if [ -d "$STORE/tmpfs" ] && [ -n "$(ls -A "$STORE/tmpfs" 2>/dev/null)" ]; then
	echo "tmpfs staging     LEAKED files" >>"$REPORT"; fail=1
else
	echo "tmpfs staging     cleaned" >>"$REPORT"
fi

# Clause 4: forced fuse attempts entry. Where /dev/fuse opens and a
# mount attaches, the run enters with ACTIVE_MODE=fuse, the payload
# bytes on stdout, exit 0 and no staging left behind; elsewhere the
# refusal names the node or the mount and likewise leaves nothing. No
# reachable machine holds the node (absent on the lane, uncreatable:
# mknod is EPERM even in a user namespace, measured 2026-09-30), so
# the refusal arm is the live one here and entry stays unproved until
# a host with the node and a granted mount runs this script.
out="$WORK/out-forced-fuse.txt"
timeout 120 env PODBOX_MODE=fuse "$PB" run --rm "$ALPINE" sh -c 'echo $PODBOX_ACTIVE_MODE; cat /etc/alpine-release' >"$out" 2>&1
rc=$?
echo "step forced-fuse exit $rc"
{
echo ""
echo "== forced-fuse"
echo "command           env PODBOX_MODE=fuse $PB run --rm $ALPINE sh -c 'echo \$PODBOX_ACTIVE_MODE; cat /etc/alpine-release'"
echo "exit              $rc"
echo "output:"
sed 's/^/  /' "$out"
} >>"$REPORT"
if [ "$rc" -eq 0 ]; then
	echo "fuse arm          ENTRY (the node opens and a mount attaches here)" >>"$REPORT"
	grep -q "^fuse$" "$out" || { echo "fuse entry did not report fuse" >>"$REPORT"; fail=1; }
	grep -q "3\.20" "$out" || { echo "fuse entry payload bytes missing" >>"$REPORT"; fail=1; }
else
	[ "$rc" -eq "$RUN_ERR" ] || { echo "fuse refusal exit $rc, wanted $RUN_ERR" >>"$REPORT"; fail=1; }
	echo "fuse arm          REFUSAL (the node or the mount is denied here)" >>"$REPORT"
	grep -q "dev/fuse" "$out" || { echo "fuse refusal did not name the node" >>"$REPORT"; fail=1; }
fi
if [ -d "$STORE/fuse" ] && [ -n "$(ls -A "$STORE/fuse" 2>/dev/null)" ]; then
	echo "fuse staging      LEAKED files" >>"$REPORT"; fail=1
else
	echo "fuse staging      cleaned" >>"$REPORT"
fi

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

# Clause 7: the packed rootfs without a registry. `save` writes the
# image as one OCI-layout tarball (the embedded input format:
# oci-layout, index.json and content-addressed blobs, each verified on
# the way out); `load` stages it into a fresh store with every blob
# hashed on the way in; a forced run over the loaded record enters
# with the rung word and the payload bytes; a flipped layer byte
# refuses as a digest mismatch with the bytes discarded.
STORE2="$WORK/store2"
mkdir -p "$STORE2" || exit 2
step pack-save 0 "$PB" save -o "$WORK/pack.tar" "$ALPINE"
[ -s "$WORK/pack.tar" ] || { echo "pack.tar empty or missing" >>"$REPORT"; fail=1; }
step pack-load 0 env PODBOX_STORE="$STORE2" "$PB" load -i "$WORK/pack.tar"
name=$(sed -n 's/^Loaded image: //p' "$WORK/out-pack-load.txt" | head -1)
[ -n "$name" ] || { echo "load printed no image name" >>"$REPORT"; fail=1; }
echo "loaded as         $name" >>"$REPORT"
out="$WORK/out-pack-run.txt"
timeout 120 env PODBOX_STORE="$STORE2" PODBOX_MODE=rundir "$PB" run --rm "$name" sh -c 'echo $PODBOX_ACTIVE_MODE; cat /etc/alpine-release' >"$out" 2>&1
rc=$?
echo "step pack-run exit $rc"
{
echo ""
echo "== pack-run"
echo "command           env PODBOX_STORE=<fresh> PODBOX_MODE=rundir $PB run --rm $name sh -c 'echo \$PODBOX_ACTIVE_MODE; cat /etc/alpine-release'"
echo "exit              $rc"
echo "output:"
sed 's/^/  /' "$out"
} >>"$REPORT"
[ "$rc" -eq 0 ] || { echo "pack run exit $rc, wanted 0" >>"$REPORT"; fail=1; }
grep -q "^rundir$" "$out" || { echo "pack run did not report rundir" >>"$REPORT"; fail=1; }
grep -q "3\.20" "$out" || { echo "pack run payload bytes missing" >>"$REPORT"; fail=1; }
if [ -d "$STORE2/runs" ] && [ -n "$(ls -A "$STORE2/runs" 2>/dev/null)" ]; then
	echo "pack staging      LEAKED files" >>"$REPORT"; fail=1
else
	echo "pack staging      cleaned" >>"$REPORT"
fi
# The bounded failure: one flipped byte in the largest blob (the
# layer) repacked beside the untouched index. The loader must refuse
# naming the digest mismatch, and the bytes stay out of the store.
rm -rf "$WORK/corrupt"; mkdir -p "$WORK/corrupt" || exit 2
tar -xf "$WORK/pack.tar" -C "$WORK/corrupt" || { echo "pack unpack FAILED" >>"$REPORT"; fail=1; }
layer=$(ls -S "$WORK/corrupt"/blobs/sha256/* | head -1)
[ -n "$layer" ] || { echo "no blobs in the pack" >>"$REPORT"; fail=1; }
printf '\377' | dd of="$layer" bs=1 count=1 conv=notrunc 2>/dev/null || { echo "byte flip FAILED" >>"$REPORT"; fail=1; }
(cd "$WORK/corrupt" && tar -cf "$WORK/pack-corrupt.tar" oci-layout index.json blobs/sha256/*) || { echo "corrupt repack FAILED" >>"$REPORT"; fail=1; }
out="$WORK/out-pack-corrupt.txt"
timeout 120 env PODBOX_STORE="$STORE2" "$PB" load -i "$WORK/pack-corrupt.tar" >"$out" 2>&1
rc=$?
echo "step pack-corrupt exit $rc"
{
echo ""
echo "== pack-corrupt"
echo "command           env PODBOX_STORE=<fresh> $PB load -i <flipped pack>"
echo "exit              $rc"
echo "output:"
sed 's/^/  /' "$out"
} >>"$REPORT"
[ "$rc" -ne 0 ] || { echo "corrupt pack LOADED: must refuse" >>"$REPORT"; fail=1; }
grep -qi "digest mismatch" "$out" || { echo "corrupt refusal did not name the digest mismatch" >>"$REPORT"; fail=1; }

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           LADDER RUNGS HOLD"; else echo "verdict           LADDER RUNGS OPEN"; fi
} >>"$REPORT"

cp "$REPORT" "$REPO/experiments/results/ladder-rungs.txt"
if [ -d /out ]; then cp "$REPORT" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
