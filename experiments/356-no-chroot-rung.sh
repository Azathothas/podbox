#!/bin/sh
# Question: on a host denying chroot(2) with ptmx, kvm and tun absent,
# does one no-chroot family run with the banner naming the rung, and
# does the -t refusal name ptmx beside the chroot sentence?
#
# TODO/enter.md T-0503 (ordering) and T-1317 (no-chroot rung).
#
# The fixture is two denials composed: a mount namespace hiding
# /dev/ptmx behind a directory (open answers EISDIR, the
# stat-Ok-open-denied arm T-0503 refuses) and experiments/deny-chroot.c
# denying chroot(2) with EPERM. kvm and tun are absent in the lane job
# container the way they are on the target. Pinned inputs are the two
# digests below: alpine (dynamic musl busybox: the loader family on the
# musl loader) and debian bookworm-slim (dynamic glibc dash: the loader
# family on the glibc loader). The memfd family runs a static hello
# built locally (cc -static-pie): no registry payload is static on the
# lane, and the build proves the family on a pinned toolchain rather
# than on a registry digest.
#
# Clauses:
#   1. capable-host sanity: run, run -t, EnteredRung chroot.
#   2. fixture probe: chroot denied, ptmx unusable.
#   3. T-0503: run -t names ptmx beside the chroot sentence at 125.
#   4. T-1317 loader family (glibc): debian runs at 0 with the banner
#      naming the rung and PODBOX_ACTIVE_MODE userland on payload stdout.
#   4b. T-1317 loader family (musl): alpine runs at 0 the same way,
#      through `sh` keeping the invoked name (busybox applet dispatch).
#   5. T-1317 memfd family: the static hello runs at 0 the same way.
#   6. strict arm: --strict refuses the userland run at 125.
#   7. refusal arm: an unresolvable payload refuses at 125 naming
#      chroot and the family, with the rootfs byte-identical after.
#   8. forced ladder: PODBOX_MODE=memfd enters the static hello without
#      chroot at 0; PODBOX_MODE=memfd over dynamic alpine refuses
#      naming the unforced loader family; PODBOX_MODE=fuse refuses
#      naming the force.
#   9. lifecycle scope: run -d and create refuse naming their reason.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no toolchain, no build, no pull, no namespaces).
set -u

# The job travels as /work/.podbox-job.sh: take the checkout from the
# working directory, never from $0 (see 353 for the measurement).
REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep356-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-356.txt"

ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
DEBIAN='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "static input      $ALPINE"
echo "dynamic input     $DEBIAN"
} >"$REPORT"

command -v cargo >/dev/null 2>&1 || { echo "cargo missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
unshare -Urm true >/dev/null 2>&1 || { echo "user+mount namespaces refused: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
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
FLAG=$PODBOX_EXIT_FLAG_ERROR
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

# The pulls happen outside the fixture. Either failing is the lane,
# not the binary.
for img in "$ALPINE" "$DEBIAN"; do
	if ! timeout 600 "$PB" pull "$img" >>"$REPORT" 2>&1; then
		echo "pull $img FAILED: COULD NOT RUN" | tee -a "$REPORT"
		exit 2
	fi
done

# The static hello for the memfd family. Built locally: no registry
# payload on the lane is static, and a local build pins the family to
# the toolchain rather than to a digest.
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
if (cd "$WORK/static-root" && tar -cf "$WORK/static-hello.tar" hello) 2>>"$REPORT"; then
	echo "static tar        ok" >>"$REPORT"
else
	echo "static tar        FAILED" | tee -a "$REPORT"
	exit 2
fi
if timeout 120 "$PB" import "$WORK/static-hello.tar" static-hello:1 >>"$REPORT" 2>&1; then
	echo "static import     ok" >>"$REPORT"
else
	echo "static import     FAILED" | tee -a "$REPORT"
	exit 1
fi
HELLO='static-hello:1'

# Clause 1: capable-host sanity, unfiltered.
step sane-run 0 "$PB" run --rm "$ALPINE" echo capable-hi
grep -q "capable-hi" "$WORK/out-sane-run.txt" || { echo "capable payload stdout missing" >>"$REPORT"; fail=1; }
step sane-tty 0 "$PB" run --rm -t "$ALPINE" true
"$PB" system info --format '{{.EnteredRung}}' >"$WORK/entered.txt" 2>&1
grep -q "^chroot$" "$WORK/entered.txt" || { echo "EnteredRung is not chroot here:" >>"$REPORT"; cat "$WORK/entered.txt" >>"$REPORT"; fail=1; }

# Clause 2: the fixture. A directory over /dev/ptmx inside a mount
# namespace, and the seccomp launcher denying chroot(2).
echo "== build the fixture"
if cc -O2 -Wall -Werror -static -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>>"$REPORT"; then
	echo "fixture           static" >>"$REPORT"
elif cc -O2 -Wall -Werror -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>>"$REPORT"; then
	echo "fixture           dynamic" >>"$REPORT"
else
	echo "fixture           COMPILE FAILED" | tee -a "$REPORT"
	exit 2
fi
mkdir -p "$WORK/emptydir" || exit 2
cat >"$WORK/wrap.sh" <<'EOF'
#!/bin/sh
# Inside user+mount namespaces: hide the pty multiplexer, deny chroot,
# run podbox. /dev/ptmx is a symlink, so the hide covers /dev/pts
# itself: open answers ENOENT there.
mount -t tmpfs tmpfs /dev/pts || exit 2
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF
FX="unshare -Urm sh $WORK/wrap.sh $WORK"
export FX
echo "fixture           $FX \$PB ..." >>"$REPORT"

# The fixture proves itself before anything asserts through it: a bare
# true through namespaces, mount and filter at exit 0.
step fx-smoke 0 $FX true
step fx-probe 0 $FX "$PB" probe --rows
grep -qi "eperm\|denied" "$WORK/out-fx-probe.txt" \
	|| { echo "fixture probe shows no denial" >>"$REPORT"; fail=1; }

# Clause 3 (T-0503): -t on the both-denied fixture names ptmx beside
# the chroot sentence, at 125.
step fx-tty "$RUN_ERR" $FX "$PB" run --rm -t "$ALPINE" true
grep -q "ptmx" "$WORK/out-fx-tty.txt" || { echo "-t refusal did not name ptmx" >>"$REPORT"; fail=1; }
grep -q "chroot(2) is denied" "$WORK/out-fx-tty.txt" || { echo "-t refusal masked the chroot denial" >>"$REPORT"; fail=1; }

# Clause 4 (T-1317 loader, glibc): debian runs without chroot at 0.
step fx-debian 0 $FX "$PB" run --rm "$DEBIAN" sh -c 'echo $PODBOX_ACTIVE_MODE'
grep -q "^userland$" "$WORK/out-fx-debian.txt" || { echo "loader run did not report userland" >>"$REPORT"; fail=1; }
grep -q "loader family" "$WORK/out-fx-debian.txt" || { echo "loader banner missing the family" >>"$REPORT"; fail=1; }

# Clause 4b (T-1317 loader, musl): alpine runs without chroot at 0
# through the invoked name. Busybox dispatches on argv[0]'s basename,
# so `sh` must stay `sh` and not become the resolved `busybox` file.
step fx-alpine 0 $FX "$PB" run --rm "$ALPINE" sh -c 'echo $PODBOX_ACTIVE_MODE'
grep -q "^userland$" "$WORK/out-fx-alpine.txt" || { echo "musl loader run did not report userland" >>"$REPORT"; fail=1; }
grep -q "loader family" "$WORK/out-fx-alpine.txt" || { echo "musl loader banner missing the family" >>"$REPORT"; fail=1; }

# Clause 5 (T-1317 memfd): the static hello runs without chroot at 0.
step fx-hello 0 $FX "$PB" run --rm "$HELLO" /hello
grep -q "static-hi" "$WORK/out-fx-hello.txt" || { echo "memfd payload stdout missing" >>"$REPORT"; fail=1; }
grep -q "memfd family" "$WORK/out-fx-hello.txt" || { echo "memfd banner missing the family" >>"$REPORT"; fail=1; }

# Clause 6 (strict arm): --strict refuses the weaker rung at 125.
step fx-strict "$RUN_ERR" $FX "$PB" run --rm --strict "$ALPINE" true
grep -qi "strict" "$WORK/out-fx-strict.txt" || { echo "--strict refusal unnamed" >>"$REPORT"; fail=1; }

# Clause 7 (refusal arm): nothing resolvable refuses naming chroot and
# the family, and the rootfs is byte-identical after.
ROOTFS=$("$PB" inspect --format '{{.RootfsPath}}' "$ALPINE" 2>/dev/null) || ROOTFS=""
if [ -n "$ROOTFS" ] && [ -d "$ROOTFS" ]; then
	find "$ROOTFS" -type f -exec sha256sum {} + | sort >"$WORK/rootfs-before.txt" 2>/dev/null
	echo "rootfs files      $(wc -l <"$WORK/rootfs-before.txt")" >>"$REPORT"
else
	echo "rootfs not extracted: FAILED" >>"$REPORT"; fail=1
fi
step fx-refuse "$RUN_ERR" $FX "$PB" run --rm "$ALPINE" /nonexistent-probe-target
grep -q "chroot(2) is denied" "$WORK/out-fx-refuse.txt" || { echo "refusal did not name chroot" >>"$REPORT"; fail=1; }
grep -qi "no-chroot family" "$WORK/out-fx-refuse.txt" || { echo "refusal did not name the family" >>"$REPORT"; fail=1; }
if [ -f "$WORK/rootfs-before.txt" ]; then
	find "$ROOTFS" -type f -exec sha256sum {} + | sort >"$WORK/rootfs-after.txt" 2>/dev/null
	if cmp -s "$WORK/rootfs-before.txt" "$WORK/rootfs-after.txt"; then
		echo "refusal mutation   none" >>"$REPORT"
	else
		echo "refusal MUTATED the rootfs" >>"$REPORT"; fail=1
	fi
fi

# Clause 8 (forced ladder): memfd enters the static payload without
# chroot; memfd forced over a dynamic payload refuses naming the
# unforced loader family; fuse refuses naming the force.
step fx-forced-hello 0 env PODBOX_MODE=memfd $FX "$PB" run --rm "$HELLO" /hello
grep -q "static-hi" "$WORK/out-fx-forced-hello.txt" || { echo "forced memfd payload stdout missing" >>"$REPORT"; fail=1; }
step fx-forced-memfd "$RUN_ERR" env PODBOX_MODE=memfd $FX "$PB" run --rm "$ALPINE" echo forced-hi
grep -q "without PODBOX_MODE" "$WORK/out-fx-forced-memfd.txt" || { echo "forced memfd over dynamic did not name the unforced family" >>"$REPORT"; fail=1; }
step fx-forced-fuse "$RUN_ERR" env PODBOX_MODE=fuse $FX "$PB" run --rm "$ALPINE" true
grep -q "PODBOX_MODE=fuse" "$WORK/out-fx-forced-fuse.txt" || { echo "forced fuse refusal unnamed" >>"$REPORT"; fail=1; }

# Clause 9 (lifecycle scope): detached and record paths stay chroot-gated.
step fx-detached "$RUN_ERR" $FX "$PB" run -d --name ul356 "$ALPINE" sleep 60
grep -q "run foreground" "$WORK/out-fx-detached.txt" || { echo "-d refusal did not name the fallback" >>"$REPORT"; fail=1; }
step fx-create "$RUN_ERR" $FX "$PB" create --name ul356c "$ALPINE" true
grep -q "chroot(2) is denied" "$WORK/out-fx-create.txt" || { echo "create did not stay chroot-gated" >>"$REPORT"; fail=1; }

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           NO-CHROOT FAMILY RUNS"; else echo "verdict           NO-CHROOT FAMILY OPEN"; fi
} >>"$REPORT"

cp "$REPORT" "$REPO/experiments/results/no-chroot-rung.txt"
if [ -d /out ]; then cp "$REPORT" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
