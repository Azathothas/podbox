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
#   9. detached (T-1411): one container through run -d, ps, logs,
#      stop and rm over the fixture, plus create and start on a second
#      one. The launcher drives the userland loader family with pidfd
#      supervision as today; the record carries the loader argv with the
#      libraries, the PODBOX_MAPS table and PODBOX_GUEST_EXE inside it.
#      Detached payloads stay builtin-only (no grandchild exec, clause
#      17): `while :` keeps the container up for ps and stop.
#   10. workdir (T-1412): -w to a missing directory refuses naming
#      the path and the image at 125, on the fixture and sane; -w /tmp
#      prints exactly /tmp (the //tmp read-back is fixed, T-1411).
#   11. maps (T-1407): the banner names the PODBOX_MAPS anchoring; a
#      caller table is scrubbed; --unsafe-host-paths opts out; an
#      absolute-path read sees the image; a /tmp write stays off host.
#   12. exe-force (T-1409): the loader banner names PODBOX_GUEST_EXE.
#   13. script (T-1413): an imported script refuses at 126 naming the
#      resolved path and its #! line.
#   14. quiet (T-1416): default runs silent; --verbose prints; -q
#      silences; refusals still print.
#   15. exec (T-1410): image and container exec drive at 0, the
#      fixture through the loader family.
#   16. records (T-1421): a --name foreground run leaves an Exited
#      record with logs; a duplicate --name refuses; --rm removes it.
#   17. grandchild limit (T-1411, informational): a shell grandchild
#      exec'ing an absolute path bypasses the loader mapping and fails
#      127 on userland, while the chroot control runs it at 0. It pins
#      the limit the loader banner names; it is not a regression gate.
#   18. exe (T-1409): `readlink /proc/self/exe` as a first-level
#      payload prints the guest path over the fixture; the static go
#      binary stays a pinned limit (exit 2 naming GOROOT on memfd).
#   19. gcc (T-1408): `gcc --version` runs on the gcc image over the
#      fixture, proving the loader-symlink fix on a second image.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no toolchain, no build, no pull, no namespaces).
set -u

# The Windows wrapper runs the job at /in/job.sh with /work as the
# working directory. Take the checkout from that directory, not $0.
REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep356-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-356.txt"

ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
DEBIAN='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
# T-1409 tracks the entry letter (tag, not digest); the pulled digest is
# recorded below beside the run so the input stays auditable.
GOLANG='public.ecr.aws/docker/library/golang:1.24.7-bookworm'
# T-1408 tracks the entry letter (tag, not digest); the pulled digest is
# recorded below beside the run so the input stays auditable.
GCC='public.ecr.aws/docker/library/gcc:13-bookworm'

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
NOTFOUND=$PODBOX_EXIT_NOT_FOUND
NOINVOKE=$PODBOX_EXIT_CANNOT_INVOKE

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
for img in "$ALPINE" "$DEBIAN" "$GOLANG" "$GCC"; do
	if ! timeout 600 "$PB" pull "$img" >>"$REPORT" 2>&1; then
		echo "pull $img FAILED: COULD NOT RUN" | tee -a "$REPORT"
		exit 2
	fi
done
echo "golang digest     $("$PB" inspect --format '{{.Digest}}' "$GOLANG")" >>"$REPORT"
echo "gcc digest        $("$PB" inspect --format '{{.Digest}}' "$GCC")" >>"$REPORT"

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

# The script image for the T-1413 clause: a `#!` payload with no ELF in
# it, imported the same way. The lane never runs it unfiltered, only
# the fixture refuses it at 126.
mkdir -p "$WORK/script-root" || exit 2
printf '#!/bin/sh\necho script-hi\n' >"$WORK/script-root/hello.sh"
chmod +x "$WORK/script-root/hello.sh"
(cd "$WORK/script-root" && tar -cf "$WORK/script-img.tar" hello.sh) 2>>"$REPORT" \
	|| { echo "script tar        FAILED" | tee -a "$REPORT"; exit 2; }
if timeout 120 "$PB" import "$WORK/script-img.tar" script-img:1 >>"$REPORT" 2>&1; then
	echo "script import     ok" >>"$REPORT"
else
	echo "script import     FAILED" | tee -a "$REPORT"
	exit 1
fi

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
# ⭐ The banner prints under --verbose since T-1416 made runs quiet by
# default; the payload's own line stays on stdout either way.
step fx-debian 0 $FX "$PB" run --rm --verbose "$DEBIAN" sh -c 'echo $PODBOX_ACTIVE_MODE'
grep -q "^userland$" "$WORK/out-fx-debian.txt" || { echo "loader run did not report userland" >>"$REPORT"; fail=1; }
grep -q "loader family" "$WORK/out-fx-debian.txt" || { echo "loader banner missing the family" >>"$REPORT"; fail=1; }
grep -q "grandchild" "$WORK/out-fx-debian.txt" || { echo "loader banner missing the grandchild limit" >>"$REPORT"; fail=1; }

# Clause 4b (T-1317 loader, musl): alpine runs without chroot at 0
# through the invoked name. Busybox dispatches on argv[0]'s basename,
# so `sh` must stay `sh` and not become the resolved `busybox` file.
step fx-alpine 0 $FX "$PB" run --rm --verbose "$ALPINE" sh -c 'echo $PODBOX_ACTIVE_MODE'
grep -q "^userland$" "$WORK/out-fx-alpine.txt" || { echo "musl loader run did not report userland" >>"$REPORT"; fail=1; }
grep -q "loader family" "$WORK/out-fx-alpine.txt" || { echo "musl loader banner missing the family" >>"$REPORT"; fail=1; }

# Clause 5 (T-1317 memfd): the static hello runs without chroot at 0.
step fx-hello 0 $FX "$PB" run --rm --verbose "$HELLO" /hello
grep -q "static-hi" "$WORK/out-fx-hello.txt" || { echo "memfd payload stdout missing" >>"$REPORT"; fail=1; }
grep -q "memfd family" "$WORK/out-fx-hello.txt" || { echo "memfd banner missing the family" >>"$REPORT"; fail=1; }

# Clause 6 (strict arm, T-1415): kind decides. `--strict` warns on the
# best-available rung and the dev-shim substitutions and runs at 0;
# `--strict=all` keeps the T-0804 refusal of every difference at 125.
step fx-strict 0 $FX "$PB" run --rm --strict "$ALPINE" true
grep -q "strict warning" "$WORK/out-fx-strict.txt" || { echo "--strict printed no substitution warning" >>"$REPORT"; fail=1; }
step fx-strict-all "$RUN_ERR" $FX "$PB" run --rm --strict=all "$ALPINE" true
grep -q "strict=all" "$WORK/out-fx-strict-all.txt" || { echo "--strict=all refusal unnamed" >>"$REPORT"; fail=1; }

# Clause 7 (refusal arm, T-1414): nothing resolvable refuses naming
# chroot and the family at 127 (not found follows the payload status
# on every rung), and the rootfs is byte-identical after.
ROOTFS=$("$PB" inspect --format '{{.RootfsPath}}' "$ALPINE" 2>/dev/null) || ROOTFS=""
if [ -n "$ROOTFS" ] && [ -d "$ROOTFS" ]; then
	find "$ROOTFS" -type f -exec sha256sum {} + | sort >"$WORK/rootfs-before.txt" 2>/dev/null
	echo "rootfs files      $(wc -l <"$WORK/rootfs-before.txt")" >>"$REPORT"
else
	echo "rootfs not extracted: FAILED" >>"$REPORT"; fail=1
fi
step fx-refuse "$NOTFOUND" $FX "$PB" run --rm "$ALPINE" /nonexistent-probe-target
grep -q "names no file in the image" "$WORK/out-fx-refuse.txt" || { echo "refusal did not name the missing payload" >>"$REPORT"; fail=1; }
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

# Clause 9 (T-1411 detached): one container through run -d, ps,
# logs, stop and rm over the fixture, plus create and start on a
# second one. The launcher drives the userland loader family with
# pidfd supervision as today.
# ⚠ Detached payloads stay builtin-only: `while :` keeps the container
# up for ps and stop with no grandchild exec (clause 17), and the first
# echo proves logs.
step fx-detached 0 $FX "$PB" run -d --name ul356d "$ALPINE" sh -c 'echo detached-hi; while :; do :; done'
step fx-ps 0 $FX "$PB" ps
grep -q "ul356d" "$WORK/out-fx-ps.txt" || { echo "ps did not list the detached container" >>"$REPORT"; fail=1; }
step fx-logs 0 $FX "$PB" logs ul356d
grep -q "detached-hi" "$WORK/out-fx-logs.txt" || { echo "logs did not carry the payload output" >>"$REPORT"; fail=1; }
step fx-stop 0 $FX "$PB" stop ul356d
step fx-rm 0 $FX "$PB" rm ul356d
step fx-create 0 $FX "$PB" create --name ul356c "$ALPINE" sh -c 'echo created-hi; while :; do :; done'
step fx-start 0 $FX "$PB" start ul356c
step fx-ps2 0 $FX "$PB" ps
grep -q "ul356c" "$WORK/out-fx-ps2.txt" || { echo "ps did not list the started container" >>"$REPORT"; fail=1; }
step fx-logs2 0 $FX "$PB" logs ul356c
grep -q "created-hi" "$WORK/out-fx-logs2.txt" || { echo "logs did not carry the started payload output" >>"$REPORT"; fail=1; }
step fx-stop2 0 $FX "$PB" stop ul356c
step fx-rm2 0 $FX "$PB" rm ul356c

# Clause 10 (T-1412 workdir): a missing directory refuses naming the
# path and the image at 125, on the fixture and on the sane host;
# `-w /tmp` is the passing control.
step fx-workdir "$RUN_ERR" $FX "$PB" run --rm -w /no/such/dir "$ALPINE" true
grep -q "/no/such/dir" "$WORK/out-fx-workdir.txt" || { echo "workdir refusal did not name the path" >>"$REPORT"; fail=1; }
grep -q "for image" "$WORK/out-fx-workdir.txt" || { echo "workdir refusal did not name the image" >>"$REPORT"; fail=1; }
step fx-workdir-tmp 0 $FX "$PB" run --rm -w /tmp "$ALPINE" pwd
# ⭐ T-1411: the read-back joins with one separator. `-w /tmp` prints
# exactly `/tmp`; `//tmp` was the doubled join and fails the clause.
grep -qx "/tmp" "$WORK/out-fx-workdir-tmp.txt" || { echo "-w /tmp control did not print exactly /tmp" >>"$REPORT"; fail=1; }
grep -qx "//tmp" "$WORK/out-fx-workdir-tmp.txt" && { echo "-w /tmp printed the doubled //tmp" >>"$REPORT"; fail=1; }
step sane-workdir "$RUN_ERR" "$PB" run --rm -w /no/such/dir "$ALPINE" true
grep -q "/no/such/dir" "$WORK/out-sane-workdir.txt" || { echo "sane-host workdir refusal did not name the path" >>"$REPORT"; fail=1; }

# Clause 11 (T-1407 maps): the banner names the PODBOX_MAPS anchoring;
# a caller-supplied table is scrubbed; `--unsafe-host-paths` opts out;
# an absolute-path read sees the image and a /tmp write never lands on
# the host.
# ⚠ Absolute applet paths: the userland PATH is the caller's, so a bare
# name would read the host's answer or none. Builtins only (`read`,
# `echo`, redirects): grandchild exec of an external binary is a
# separate probe below, and must not gate the anchored-read leg.
step fx-maps 0 $FX "$PB" run --rm --verbose "$ALPINE" sh -c 'read rel < /etc/alpine-release && echo "release=$rel"'
grep -q "PODBOX_MAPS" "$WORK/out-fx-maps.txt" || { echo "maps banner missing" >>"$REPORT"; fail=1; }
grep -q "^release=3\.20" "$WORK/out-fx-maps.txt" || { echo "anchored read did not see the image release" >>"$REPORT"; fail=1; }
step fx-maps-scrub 0 $FX "$PB" run --rm -e PODBOX_MAPS=/:evil "$ALPINE" sh -c 'echo "maps=$PODBOX_MAPS"'
grep -q "evil" "$WORK/out-fx-maps-scrub.txt" && { echo "caller PODBOX_MAPS leaked through" >>"$REPORT"; fail=1; }
grep -q "^maps=/:" "$WORK/out-fx-maps-scrub.txt" || { echo "scrubbed run lost the built table" >>"$REPORT"; fail=1; }
step fx-tmpwrite 0 $FX "$PB" run --rm "$ALPINE" sh -c 'echo probe > /tmp/ul356-probe && echo wrote-tmp'
grep -q "wrote-tmp" "$WORK/out-fx-tmpwrite.txt" || { echo "tmp write payload output missing" >>"$REPORT"; fail=1; }
if [ -e /tmp/ul356-probe ]; then echo "payload write escaped to host /tmp" >>"$REPORT"; fail=1; else echo "host tmp stayed clean ok" >>"$REPORT"; fi
rm -f /tmp/ul356-probe
step fx-unsafe 0 $FX "$PB" run --rm --verbose --unsafe-host-paths "$ALPINE" true
grep -q -- "--unsafe-host-paths" "$WORK/out-fx-unsafe.txt" || { echo "opt-out banner missing" >>"$REPORT"; fail=1; }

# Clause 17 (T-1411 grandchild limit, informational): a shell
# grandchild exec'ing an absolute path bypasses the loader mapping and
# fails 127 on userland, while the chroot control runs it at 0. It pins
# the limit the loader banner names above; it is not a regression gate.
step fx-grandchild "$NOTFOUND" $FX "$PB" run --rm "$ALPINE" sh -c '/bin/busybox echo grandchild-ok'
grep -q "not found" "$WORK/out-fx-grandchild.txt" || { echo "grandchild limit did not read as not-found" >>"$REPORT"; fail=1; }
step sane-grandchild 0 "$PB" run --rm "$ALPINE" sh -c '/bin/busybox echo grandchild-ok'
grep -q "grandchild-ok" "$WORK/out-sane-grandchild.txt" || { echo "chroot control did not run the grandchild" >>"$REPORT"; fail=1; }

# Clause 12 (T-1409 exe-force): beside a caller PODBOX_GUEST_EXE the
# loader run answers the exe-force banner line. The exact-"1" value,
# the stale-value scrub and the memfd exclusion are unit-guarded.
step fx-exeforce 0 $FX "$PB" run --rm --verbose -e PODBOX_GUEST_EXE=/bin/echo "$DEBIAN" echo exeforce-hi
grep -q "PODBOX_GUEST_EXE" "$WORK/out-fx-exeforce.txt" || { echo "exe-force banner missing" >>"$REPORT"; fail=1; }
grep -q "exeforce-hi" "$WORK/out-fx-exeforce.txt" || { echo "exe-force payload stdout missing" >>"$REPORT"; fail=1; }

# Clause 13 (T-1413 script): an imported script refuses at 126 naming
# the resolved path and its `#!` line.
step fx-script "$NOINVOKE" $FX "$PB" run --rm script-img:1 /hello.sh
grep -q "/hello.sh" "$WORK/out-fx-script.txt" || { echo "script refusal did not name the path" >>"$REPORT"; fail=1; }
grep -q "#!/bin/sh" "$WORK/out-fx-script.txt" || { echo "script refusal did not name the shebang" >>"$REPORT"; fail=1; }

# Clause 14 (T-1416 quiet): the banner never prints at default and stays
# off under `-q`; a refusal still prints. Split streams per case: stdout
# stays empty outright (the payload owns it), and neither stream carries
# a banner line. ⚠ Extraction progress rides stderr through the
# extract-side `out` on every run (pre-existing: the committed results
# show it too), so the clause asserts banner-silence, not total
# silence: total silence is the extract side's to prove.
quiet_banner() {
	grep -q "mode=userland\|podbox: userland:\|podbox: interpose:\|podbox: complete:\|entering without chroot" "$1" \
		&& { echo "$2 carries banner lines" >>"$REPORT"; fail=1; }
}
step fx-quiet 0 $FX "$PB" run --rm "$ALPINE" true
timeout 120 $FX "$PB" run --rm "$ALPINE" true >"$WORK/quiet-out.txt" 2>"$WORK/quiet-err.txt" \
	|| { echo "quiet split-stream rerun exited $?" >>"$REPORT"; fail=1; }
[ -s "$WORK/quiet-out.txt" ] && { echo "payload stdout polluted at default" >>"$REPORT"; fail=1; }
quiet_banner "$WORK/quiet-out.txt" "default stdout"
quiet_banner "$WORK/quiet-err.txt" "default stderr"
step fx-quiet-off 0 $FX "$PB" run --rm --verbose -q "$ALPINE" true
timeout 120 $FX "$PB" run --rm --verbose -q "$ALPINE" true >"$WORK/quiet-off-out.txt" 2>"$WORK/quiet-off-err.txt" \
	|| { echo "-q split-stream rerun exited $?" >>"$REPORT"; fail=1; }
[ -s "$WORK/quiet-off-out.txt" ] && { echo "payload stdout polluted under -q" >>"$REPORT"; fail=1; }
quiet_banner "$WORK/quiet-off-out.txt" "-q stdout"
quiet_banner "$WORK/quiet-off-err.txt" "-q stderr"
[ -s "$WORK/out-fx-refuse.txt" ] || { echo "refusal printed nothing at default" >>"$REPORT"; fail=1; }

# Clause 15 (T-1410 exec): image and container exec share the entry
# decision; on the fixture the loader family drives at 0, and a fresh
# container re-entry needs nothing running (T-0505). ⚠ `--rm` prunes
# the image rootfs where no container references it (T-1322), so every
# clause above leaves no rootfs behind: the explicit `extract` ahead of
# the image exec is what a keeper would keep.
step fx-extract 0 $FX "$PB" extract "$ALPINE"
step fx-exec-img 0 $FX "$PB" exec "$ALPINE" echo exec-img-hi
grep -q "exec-img-hi" "$WORK/out-fx-exec-img.txt" || { echo "fixture image-exec stdout missing" >>"$REPORT"; fail=1; }
step sane-exec-create 0 "$PB" create --name ul356e "$ALPINE" sleep 60
step sane-exec 0 "$PB" exec ul356e echo exec-ctr-hi
grep -q "exec-ctr-hi" "$WORK/out-sane-exec.txt" || { echo "container exec stdout missing" >>"$REPORT"; fail=1; }
step sane-exec-rm 0 "$PB" rm -f ul356e

# Clause 16 (T-1421 records): a foreground `--name` run leaves a record
# `ps -a` lists as Exited with its output in `logs`; a duplicate
# `--name` refuses; `--rm` removes the record again.
step sane-named 0 "$PB" run --name ul356fg "$ALPINE" echo fg-hi
grep -q "fg-hi" "$WORK/out-sane-named.txt" || { echo "named run stdout missing" >>"$REPORT"; fail=1; }
step sane-dup "$RUN_ERR" "$PB" run --name ul356fg "$ALPINE" echo dup-hi
grep -q "ul356fg" "$WORK/out-sane-dup.txt" || { echo "duplicate --name refusal did not name the container" >>"$REPORT"; fail=1; }
step sane-ps 0 "$PB" ps -a
grep -q "ul356fg" "$WORK/out-sane-ps.txt" || { echo "ps -a did not list the foreground record" >>"$REPORT"; fail=1; }
grep -q "Exited" "$WORK/out-sane-ps.txt" || { echo "ps -a did not show the record Exited" >>"$REPORT"; fail=1; }
step sane-logs 0 "$PB" logs ul356fg
grep -q "fg-hi" "$WORK/out-sane-logs.txt" || { echo "logs did not carry the payload output" >>"$REPORT"; fail=1; }
step sane-rmnamed 0 "$PB" rm -f ul356fg
step sane-gone 0 "$PB" run --rm --name ul356rm "$ALPINE" true
step sane-ps2 0 "$PB" ps -a
grep -q "ul356rm" "$WORK/out-sane-ps2.txt" && { echo "--rm left its record behind" >>"$REPORT"; fail=1; }

# Clause 18 (T-1409 exe): the loader run answers the payload-visible
# exe. `readlink /proc/self/exe` as a first-level payload on the debian
# image over the fixture prints the guest path, never the loader's host
# path: readlink is dynamic, so the interposer answers from
# PODBOX_GUEST_EXE even where the real call succeeds, with no extra -e.
# (A python proof was considered; bookworm-slim holds no python3. The
# readlink shape is the exact code the entry changed, so it proves the
# mechanism directly.)
# The entry's go trio is superseded in writing, with the reason. The
# golang image's `go` binary is statically linked (measured with
# file(1): ELF static, no INTERP), so the tier enters it on the memfd
# family, where no interposer loads and the kernel reports the staged
# memfd path: `go version` answers exit 2 naming GOROOT, and go's own
# telemetry child fails re-exec'ing the memfd path. Three routes were
# tested: go-is-dynamic (refuted by file(1)), interpose-the-static
# payload (refuted: raw syscalls never reach LD_PRELOAD, and the
# telemetry error quotes the memfd path), and injecting GOROOT for go
# payloads (refused: payload-specific magic against the entry's own
# decision). The memfd banner above names the limit; the clause pins
# go's answer as the boundary rather than as a regression.
step fx-exe-readlink 0 $FX "$PB" run --rm "$DEBIAN" readlink /proc/self/exe
# The answer line is exactly the guest path. An earlier whole-file grep
# for "rootfs" false-fired on podbox's own extraction diagnostics (lane
# job 947e9e1e6ad11d71: the answer was /usr/bin/readlink while stderr
# carried store/rootfs/ lines), so the assertion matches the full line:
# a leaked loader host path would BE the answer and fail this match.
grep -qx "/usr/bin/readlink" "$WORK/out-fx-exe-readlink.txt" || { echo "readlink did not answer exactly the guest path" >>"$REPORT"; fail=1; }
step fx-go-limit 2 $FX "$PB" run --rm "$GOLANG" go version
# want is the payload's own 2 (T-0802: passed through unchanged), not 127:
# the limit is that go fails naming GOROOT, not that podbox refuses it.
grep -q "GOROOT" "$WORK/out-fx-go-limit.txt" || { echo "go limit did not name GOROOT" >>"$REPORT"; fail=1; }

# Clause 19 (T-1408 gcc): the loader symlink resolves inside the rootfs
# on a second image. Debian's /lib64/ld-linux carries an absolute
# target dangling on the host; the gcc image exercises the same
# resolve_in path as clause 4 with its own loader and libraries.
step fx-gcc-version 0 $FX "$PB" run --rm "$GCC" gcc --version
grep -q "gcc" "$WORK/out-fx-gcc-version.txt" || { echo "gcc --version did not answer" >>"$REPORT"; fail=1; }

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           NO-CHROOT FAMILY RUNS"; else echo "verdict           NO-CHROOT FAMILY OPEN"; fi
} >>"$REPORT"

cp "$REPORT" "$REPO/experiments/results/no-chroot-rung.txt"
if [ -d /out ]; then cp "$REPORT" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
