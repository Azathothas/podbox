#!/bin/sh
# Question: on a host denying chroot(2) with ptmx, kvm and tun absent,
# does one no-chroot family run with the banner naming the rung, and
# does the -t refusal name ptmx beside the chroot sentence?
#
# TODO/enter.md T-0503 (ordering), T-1317 (no-chroot rung) and T-1648
# (a static payload refuses without --unsafe-host-paths).
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
#      naming the rung and PODVM_ACTIVE_MODE userland on payload stdout.
#   4b. T-1317 loader family (musl): alpine runs at 0 the same way,
#      through `sh` keeping the invoked name (busybox applet dispatch).
#   5. T-1317 memfd family: the static hello runs at 0 the same way,
#      under --unsafe-host-paths; at the quiet default the host-root line
#      prints and the banner does not. T-1652: without the opt-in it runs
#      at 0 on the native-static rung, whose banner names it; with the
#      prctl of Syscall User Dispatch denied (--deny-dispatch) it refuses
#      at 125 naming the static reason, the prctl and the emulate tier
#      (T-1648).
#   6. strict arm: --strict refuses the userland run at 125, and
#      refuses --unsafe-host-paths on the static hello (T-1415).
#   7. refusal arm: an unresolvable payload refuses at 125 naming
#      chroot and the family, with the rootfs byte-identical after.
#   8. forced ladder: PODVM_MODE=memfd enters the static hello without
#      chroot at 0 under --unsafe-host-paths (T-1648);
#      PODVM_MODE=memfd over dynamic alpine refuses
#      naming the unforced loader family; PODVM_MODE=fuse refuses
#      naming the force.
#   9. detached (T-1411): one container through run -d, ps, logs,
#      stop and rm over the fixture, plus create and start on a second
#      one. The launcher drives the userland loader family with pidfd
#      supervision as today; the record carries the loader argv with the
#      libraries, the PODVM_MAPS table and PODVM_GUEST_EXE inside it.
#      Detached payloads stay builtin-only: `while :` keeps the
#      container up for ps and stop.
#   10. workdir (T-1412): -w to a missing directory refuses naming
#      the path and the image at 125, on the fixture and sane; -w /tmp
#      prints exactly /tmp (the //tmp read-back is fixed, T-1411).
#   11. maps (T-1407): the banner names the PODVM_MAPS anchoring; a
#      caller table is scrubbed; --unsafe-host-paths opts out; an
#      absolute-path read sees the image; a /tmp write stays off host.
#   12. exe-force (T-1409): the loader banner names PODVM_GUEST_EXE.
#   13. script (T-1413, T-1649): an imported script with no /bin/sh in
#      its image refuses at 126 naming the resolved path and its #!
#      line; the same #!/bin/sh script beside the alpine rootfs runs at 0
#      through the interpreter of the image; the node image with a caller
#      command and no --entrypoint runs through its #! entrypoint, or
#      refuses naming `--entrypoint node`.
#   14. quiet (T-1416): default runs silent; --verbose prints; -q
#      silences; refusals still print.
#   15. exec (T-1410): image and container exec drive at 0, the
#      fixture through the loader family.
#   16. records (T-1421): a --name foreground run leaves an Exited
#      record with logs; a duplicate --name refuses; --rm removes it.
#   17. grandchild (T-1411, a gate since T-1646): a shell grandchild
#      exec'ing an absolute path runs at 0 on userland through the
#      image's loader, and the chroot control runs it at 0.
#   18. exe (T-1409): `readlink /proc/self/exe` as a first-level
#      payload prints the guest path over the fixture. T-1652: the
#      static go binary runs on the native-static rung: `go version`
#      exits 0, `go build` of a hello program exits 0, the built program
#      prints its line, and nothing lands on the host; the probe reports
#      measured_by.user_dispatch true on the fixture.
#   19. gcc (T-1408): `gcc --version` runs on the gcc image over the
#      fixture, proving the loader-symlink fix on a second image.
#   20. relative paths (T-1643): a relative read, a relative write
#      under -w /tmp, a dirfd-relative openat and `cd /; echo x >
#      rel.txt` land inside the rootfs, and none reaches the host.
#   21. dlopen (T-1645): `python3 -c 'import subprocess'` runs at 0 on
#      the python image, and a bare soname still loads.
#   22. glibc closure (T-1644): debian `echo hi` and gcc `gcc --version`
#      run at 0 with the glibc object preloaded.
#   23. grandchild exec (T-1646): `sh -c '/bin/ls /'` runs at 0 on
#      debian and alpine (busybox applet), a pipeline and a bare name
#      through alpine's absolute links run, a `#!` script grandchild
#      runs (direct and through `/usr/bin/env`), a grandchild reads its
#      own guest path from /proc/self/exe, and node:22-alpine runs
#      `node -e "execSync('ls /')"` (A-V-04 of PodUtils/podcrates#39).
#   24. strict capability (T-1650): `--strict` of `sh -c '/bin/ls /'`
#      runs at 0 with the listing and warns [capability] naming T-1646;
#      `--strict` of `/bin/echo hi` runs at 0; `--strict` of a shell
#      that runs a static program of the image refuses at 125 naming
#      T-1652. T-1697 closed its gap: `--strict` of `npm --version`
#      runs at 0 and names no T-1697 gap.
#   25. raw statx (T-1697): node's `fs.statSync` and `fs.lstatSync` of
#      guest paths answer from the rootfs (libuv calls `statx` through
#      libc's `syscall()`), and `npm ci` then `npm test` of a project
#      with no dependencies and a `postinstall` child exit 0 from the
#      node image, with no registry.
#   26. volumes (T-1653): `-v HOSTDIR:/data` reads a host file at the
#      guest path, a guest write and a relative write after `cd /data`
#      land in HOSTDIR, a shell grandchild reads the volume, `-w /data`
#      starts the payload in the volume (`pwd` prints `/data`, a relative
#      write lands in HOSTDIR), a first program `/data/busybox` copied
#      from the image runs from the volume, `:ro` refuses at 125 naming
#      T-1655, the memfd family refuses a volume at 125, and the banner
#      names a source on a noexec mount. Before podcrates v0.12.0 the
#      `-w` and the first program in a volume refused at 125.
#   27. the entered rung (T-1712, T-1713, T-1716): `--podvm-tier=chroot`
#      is a flag error at 125 naming the value; `system info` under the
#      fixture reads `.EnteredRung` = userland, the rung that the run
#      enters, whatever `.Rung` the probe selects; a `-v` pair under
#      /proc refuses at 125 naming /proc; a missing `-w` inside a volume
#      refuses at 125 naming the volume, not the image.
#   28. the exe of realpath (T-1704): GNU `realpath /proc/self/exe` in
#      debian prints /usr/bin/realpath, not the loader; a program built
#      with _FORTIFY_SOURCE=2 (it calls __realpath_chk and __readlink_chk)
#      prints its own guest path from realpath and from readlink.
#   30. read-only (T-1655): under --read-only on the fixture a write to the
#      image fails with EROFS and a write to a volume lands in the host
#      directory; on the capable rung --read-only refuses at 125 by name.
#   32. library directories (T-1719): on the fixture, a musl program in
#      alpine finds its library in /usr/local/lib, and a glibc program in
#      debian finds its library in a directory that only the image's
#      /etc/ld.so.conf.d names; each prints lib-42.
#   31. workdir (T-1707): `-w /bin ./busybox sh -c 'ls /etc/alpine-release'`
#      lists the file of the image on the fixture, and `-w /data
#      ./tool.sh` runs a script of a volume workdir.
#   29. the loader's paths (T-1705): a program of a debian image opens the
#      path that dladdr gives for its own library (a host path under the
#      rootfs, from the loader) at 0, and the path is not prefixed again.
#   33. the native-static rung for a child and a volume (T-1724): the
#      golang image's dynamic sh runs `go build` and the program it built,
#      with the output in the rootfs and nothing on the host; a static
#      payload on the rung reads and writes a file of a -v volume, and its
#      working directory in the volume reads back as the guest path.
#   34. absolute links (T-1725): in alpine, `cat` of an absolute link of
#      the image (`/home/l734 -> /opt/l734/f`) and of a path through an
#      absolute directory link reads the file of the rootfs, `ls -l` still
#      shows the link, and a link whose target only the host has gives
#      ENOENT.
#   35. exec and :ro (T-1726): `exec` into a container with a `:ro`
#      volume fails a write there, and the host directory has no file.
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
# T-1643 and T-1645 track the entry letter (tag, not digest); the pulled
# digest is recorded below beside the run so the input stays auditable.
PYTHON='public.ecr.aws/docker/library/python:3-alpine'
# T-1646 tracks the entry letter (tag, not digest); the pulled digest is
# recorded below beside the run so the input stays auditable.
NODE='public.ecr.aws/docker/library/node:22-alpine'
# ⚠ PODVM_356_MIRROR=1 takes the Docker-library images from mirror.gcr.io,
# for a host where public.ecr.aws answers image pulls with HTTP 429 (the
# cloud host of 2026-11-09 and 2026-11-10, TODO/probe.md T-1670). The
# alpine digest is the same there. ECR's debian repository is not the
# Docker library image, so DEBIAN becomes library/debian:bookworm-slim by
# tag; the conditions print each pulled digest, so the input stays
# auditable. Without the switch the pinned inputs above are unchanged.
if [ "${PODVM_356_MIRROR:-0}" = 1 ]; then
	ALPINE='mirror.gcr.io/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
	DEBIAN='mirror.gcr.io/library/debian:bookworm-slim'
	GOLANG='mirror.gcr.io/library/golang:1.24.7-bookworm'
	GCC='mirror.gcr.io/library/gcc:13-bookworm'
	PYTHON='mirror.gcr.io/library/python:3-alpine'
	NODE='mirror.gcr.io/library/node:22-alpine'
fi

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
PB="$REPO/target/x86_64-unknown-linux-musl/debug/podvm"
[ -x "$PB" ] || { echo "binary missing after build: FAILED" | tee -a "$REPORT"; exit 1; }
echo "podvm            $("$PB" version)" >>"$REPORT"

# shellcheck source=../scripts/common/exit-codes.sh
. "$REPO/scripts/common/exit-codes.sh"
podvm_exit_codes "$PB" || { echo "cannot read podvm's exit-code table: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
FLAG=$PODVM_EXIT_FLAG_ERROR
RUN_ERR=$PODVM_EXIT_RUNTIME_ERROR
NOTFOUND=$PODVM_EXIT_NOT_FOUND
NOINVOKE=$PODVM_EXIT_CANNOT_INVOKE

STORE="$WORK/store"
mkdir -p "$STORE" || exit 2
export PODVM_STORE="$STORE"
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
for img in "$ALPINE" "$DEBIAN" "$GOLANG" "$GCC" "$PYTHON" "$NODE"; do
	if ! timeout 600 "$PB" pull "$img" >>"$REPORT" 2>&1; then
		echo "pull $img FAILED: COULD NOT RUN" | tee -a "$REPORT"
		exit 2
	fi
done
if [ "${PODVM_356_MIRROR:-0}" = 1 ]; then
	echo "image source      mirror.gcr.io (PODVM_356_MIRROR=1)" >>"$REPORT"
else
	echo "image source      the pinned inputs" >>"$REPORT"
fi
echo "alpine digest     $("$PB" inspect --format '{{.Digest}}' "$ALPINE")" >>"$REPORT"
echo "debian digest     $("$PB" inspect --format '{{.Digest}}' "$DEBIAN")" >>"$REPORT"
echo "golang digest     $("$PB" inspect --format '{{.Digest}}' "$GOLANG")" >>"$REPORT"
echo "gcc digest        $("$PB" inspect --format '{{.Digest}}' "$GCC")" >>"$REPORT"
echo "python digest     $("$PB" inspect --format '{{.Digest}}' "$PYTHON")" >>"$REPORT"
echo "node digest       $("$PB" inspect --format '{{.Digest}}' "$NODE")" >>"$REPORT"

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
printf '#!/bin/sh\necho script-hi "$0" "$1"\n' >"$WORK/script-root/hello.sh"
chmod +x "$WORK/script-root/hello.sh"
(cd "$WORK/script-root" && tar -cf "$WORK/script-img.tar" hello.sh) 2>>"$REPORT" \
	|| { echo "script tar        FAILED" | tee -a "$REPORT"; exit 2; }
if timeout 120 "$PB" import "$WORK/script-img.tar" script-img:1 >>"$REPORT" 2>&1; then
	echo "script import     ok" >>"$REPORT"
else
	echo "script import     FAILED" | tee -a "$REPORT"
	exit 1
fi
# T-1649 and T-1650: the same script and the static hello beside the
# alpine rootfs, so /bin/sh of the image runs the script, and a shell
# of the image can start a static program.
timeout 600 "$PB" extract "$ALPINE" >>"$REPORT" 2>&1 || { echo "alpine extract FAILED" | tee -a "$REPORT"; exit 1; }
AROOT=$("$PB" inspect --format '{{.RootfsPath}}' "$ALPINE" 2>/dev/null) || AROOT=""
[ -d "$AROOT" ] || { echo "alpine rootfs missing: FAILED" | tee -a "$REPORT"; exit 1; }
tar -cf "$WORK/script-alpine.tar" -C "$AROOT" . -C "$WORK/script-root" hello.sh -C "$WORK/static-root" hello 2>>"$REPORT" \
	|| { echo "script-alpine tar FAILED" | tee -a "$REPORT"; exit 2; }
if timeout 120 "$PB" import "$WORK/script-alpine.tar" script-alpine:1 >>"$REPORT" 2>&1; then
	echo "script-alpine     ok" >>"$REPORT"
else
	echo "script-alpine     FAILED" | tee -a "$REPORT"
	exit 1
fi

# Clause 1: capable-host sanity, unfiltered.
step sane-run 0 "$PB" run --rm "$ALPINE" echo capable-hi
grep -q "capable-hi" "$WORK/out-sane-run.txt" || { echo "capable payload stdout missing" >>"$REPORT"; fail=1; }
step sane-tty 0 "$PB" run --rm -t "$ALPINE" true
"$PB" system info --format '{{.EnteredRung}}' >"$WORK/entered.txt" 2>&1
# ⚠ A capable host enters a rung with a root change: chroot, or namespace
# where the host permits user and mount namespaces (the cloud host of
# TODO/probe.md T-1670 does). Either is the control; the run records which.
grep -qE "^(chroot|namespace)$" "$WORK/entered.txt" || { echo "EnteredRung is not a capable rung here:" >>"$REPORT"; cat "$WORK/entered.txt" >>"$REPORT"; fail=1; }
echo "capable rung      $(cat "$WORK/entered.txt")" >>"$REPORT"

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
# run podvm. /dev/ptmx is a symlink, so the hide covers /dev/pts
# itself: open answers ENOENT there.
mount -t tmpfs tmpfs /dev/pts || exit 2
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF
FX="unshare -Urm sh $WORK/wrap.sh $WORK"
# T-1652: the same fixture, with prctl(PR_SET_SYSCALL_USER_DISPATCH) denied.
FXD="unshare -Urm sh $WORK/wrap.sh $WORK --deny-dispatch"
export FX FXD
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
step fx-debian 0 $FX "$PB" run --rm --verbose "$DEBIAN" sh -c 'echo $PODVM_ACTIVE_MODE'
grep -q "^userland$" "$WORK/out-fx-debian.txt" || { echo "loader run did not report userland" >>"$REPORT"; fail=1; }
grep -q "loader family" "$WORK/out-fx-debian.txt" || { echo "loader banner missing the family" >>"$REPORT"; fail=1; }
# T-1646 reworded the banner: a child enters through the image's loader.
grep -q "a child that execs" "$WORK/out-fx-debian.txt" || { echo "loader banner missing the child route" >>"$REPORT"; fail=1; }

# Clause 4b (T-1317 loader, musl): alpine runs without chroot at 0
# through the invoked name. Busybox dispatches on argv[0]'s basename,
# so `sh` must stay `sh` and not become the resolved `busybox` file.
step fx-alpine 0 $FX "$PB" run --rm --verbose "$ALPINE" sh -c 'echo $PODVM_ACTIVE_MODE'
grep -q "^userland$" "$WORK/out-fx-alpine.txt" || { echo "musl loader run did not report userland" >>"$REPORT"; fail=1; }
grep -q "loader family" "$WORK/out-fx-alpine.txt" || { echo "musl loader banner missing the family" >>"$REPORT"; fail=1; }

# Clause 5 (T-1317 memfd): the static hello runs without chroot at 0.
# ⭐ T-1648: the memfd family resolves absolute paths on the host, so it
# runs only under --unsafe-host-paths. ⭐ T-1652: without it the payload
# runs on the native-static rung; where the prctl is denied it refuses
# at 125 before the exec, naming the static reason and the routes.
step fx-hello 0 $FX "$PB" run --rm --verbose --unsafe-host-paths "$HELLO" /hello
grep -q "static-hi" "$WORK/out-fx-hello.txt" || { echo "memfd payload stdout missing" >>"$REPORT"; fail=1; }
grep -q "memfd family" "$WORK/out-fx-hello.txt" || { echo "memfd banner missing the family" >>"$REPORT"; fail=1; }
step fx-hello-native 0 $FX "$PB" run --rm --verbose "$HELLO" /hello
grep -q "static-hi" "$WORK/out-fx-hello-native.txt" || { echo "native-static payload stdout missing" >>"$REPORT"; fail=1; }
grep -q "native-static rung" "$WORK/out-fx-hello-native.txt" || { echo "native-static banner missing the rung" >>"$REPORT"; fail=1; }
step fx-hello-refuse "$RUN_ERR" $FXD "$PB" run --rm "$HELLO" /hello
grep -q "statically linked" "$WORK/out-fx-hello-refuse.txt" || { echo "static refusal did not name the static reason" >>"$REPORT"; fail=1; }
grep -q -- "--unsafe-host-paths" "$WORK/out-fx-hello-refuse.txt" || { echo "static refusal did not name the opt-in" >>"$REPORT"; fail=1; }
grep -q "emulate tier" "$WORK/out-fx-hello-refuse.txt" || { echo "static refusal did not name the emulate tier" >>"$REPORT"; fail=1; }
grep -q "prctl(PR_SET_SYSCALL_USER_DISPATCH)" "$WORK/out-fx-hello-refuse.txt" || { echo "static refusal did not name the denied prctl" >>"$REPORT"; fail=1; }
grep -q "static-hi" "$WORK/out-fx-hello-refuse.txt" && { echo "static refusal ran the payload" >>"$REPORT"; fail=1; }
# At the quiet default the opt-in's host-root line still prints, and
# the rest of the banner stays off.
step fx-hello-quiet 0 $FX "$PB" run --rm --unsafe-host-paths "$HELLO" /hello
grep -q "running with --unsafe-host-paths" "$WORK/out-fx-hello-quiet.txt" || { echo "host-root line hidden at the quiet default" >>"$REPORT"; fail=1; }
grep -q "memfd family" "$WORK/out-fx-hello-quiet.txt" && { echo "quiet opt-in printed the banner" >>"$REPORT"; fail=1; }
grep -q "static-hi" "$WORK/out-fx-hello-quiet.txt" || { echo "quiet opt-in payload stdout missing" >>"$REPORT"; fail=1; }

# Clause 6 (strict arm, T-1415): kind decides. `--strict` warns on the
# best-available rung and the dev-shim substitutions and runs at 0;
# `--strict=all` keeps the T-0804 refusal of every difference at 125.
step fx-strict 0 $FX "$PB" run --rm --strict "$ALPINE" true
grep -q "strict warning" "$WORK/out-fx-strict.txt" || { echo "--strict printed no substitution warning" >>"$REPORT"; fail=1; }
step fx-strict-all "$RUN_ERR" $FX "$PB" run --rm --strict=all "$ALPINE" true
grep -q "strict=all" "$WORK/out-fx-strict-all.txt" || { echo "--strict=all refusal unnamed" >>"$REPORT"; fail=1; }
step fx-strict-unsafe "$RUN_ERR" $FX "$PB" run --rm --strict --unsafe-host-paths "$HELLO" /hello
grep -q "static-hi" "$WORK/out-fx-strict-unsafe.txt" && { echo "--strict ran the opt-in" >>"$REPORT"; fail=1; }

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
step fx-forced-hello 0 env PODVM_MODE=memfd $FX "$PB" run --rm --unsafe-host-paths "$HELLO" /hello
grep -q "static-hi" "$WORK/out-fx-forced-hello.txt" || { echo "forced memfd payload stdout missing" >>"$REPORT"; fail=1; }
step fx-forced-memfd "$RUN_ERR" env PODVM_MODE=memfd $FX "$PB" run --rm "$ALPINE" echo forced-hi
grep -q "without PODVM_MODE" "$WORK/out-fx-forced-memfd.txt" || { echo "forced memfd over dynamic did not name the unforced family" >>"$REPORT"; fail=1; }
step fx-forced-fuse "$RUN_ERR" env PODVM_MODE=fuse $FX "$PB" run --rm "$ALPINE" true
grep -q "PODVM_MODE=fuse" "$WORK/out-fx-forced-fuse.txt" || { echo "forced fuse refusal unnamed" >>"$REPORT"; fail=1; }

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

# Clause 11 (T-1407 maps): the banner names the PODVM_MAPS anchoring;
# a caller-supplied table is scrubbed; `--unsafe-host-paths` opts out;
# an absolute-path read sees the image and a /tmp write never lands on
# the host.
# ⚠ Absolute applet paths: the userland PATH is the caller's, so a bare
# name would read the host's answer or none. Builtins only (`read`,
# `echo`, redirects): grandchild exec of an external binary is a
# separate probe below, and must not gate the anchored-read leg.
step fx-maps 0 $FX "$PB" run --rm --verbose "$ALPINE" sh -c 'read rel < /etc/alpine-release && echo "release=$rel"'
grep -q "PODVM_MAPS" "$WORK/out-fx-maps.txt" || { echo "maps banner missing" >>"$REPORT"; fail=1; }
grep -q "^release=3\.20" "$WORK/out-fx-maps.txt" || { echo "anchored read did not see the image release" >>"$REPORT"; fail=1; }
step fx-maps-scrub 0 $FX "$PB" run --rm -e PODVM_MAPS=/:evil "$ALPINE" sh -c 'echo "maps=$PODVM_MAPS"'
grep -q "evil" "$WORK/out-fx-maps-scrub.txt" && { echo "caller PODVM_MAPS leaked through" >>"$REPORT"; fail=1; }
grep -q "^maps=/:" "$WORK/out-fx-maps-scrub.txt" || { echo "scrubbed run lost the built table" >>"$REPORT"; fail=1; }
step fx-tmpwrite 0 $FX "$PB" run --rm "$ALPINE" sh -c 'echo probe > /tmp/ul356-probe && echo wrote-tmp'
grep -q "wrote-tmp" "$WORK/out-fx-tmpwrite.txt" || { echo "tmp write payload output missing" >>"$REPORT"; fail=1; }
if [ -e /tmp/ul356-probe ]; then echo "payload write escaped to host /tmp" >>"$REPORT"; fail=1; else echo "host tmp stayed clean ok" >>"$REPORT"; fi
rm -f /tmp/ul356-probe
step fx-unsafe 0 $FX "$PB" run --rm --verbose --unsafe-host-paths "$ALPINE" true
grep -q -- "--unsafe-host-paths" "$WORK/out-fx-unsafe.txt" || { echo "opt-out banner missing" >>"$REPORT"; fail=1; }

# Clause 17 (T-1411, a gate since T-1646): a shell grandchild exec'ing
# an absolute path runs at 0 on userland. The interposer's exec family
# runs it through the image's loader inside the rootfs; before T-1646
# it failed 127 here. The chroot control runs it at 0.
step fx-grandchild 0 $FX "$PB" run --rm "$ALPINE" sh -c '/bin/busybox echo grandchild-ok'
grep -qx "grandchild-ok" "$WORK/out-fx-grandchild.txt" || { echo "userland grandchild did not run" >>"$REPORT"; fail=1; }
step sane-grandchild 0 "$PB" run --rm "$ALPINE" sh -c '/bin/busybox echo grandchild-ok'
grep -q "grandchild-ok" "$WORK/out-sane-grandchild.txt" || { echo "chroot control did not run the grandchild" >>"$REPORT"; fail=1; }

# Clause 12 (T-1409 exe-force): beside a caller PODVM_GUEST_EXE the
# loader run answers the exe-force banner line. The exact-"1" value,
# the stale-value scrub and the memfd exclusion are unit-guarded.
step fx-exeforce 0 $FX "$PB" run --rm --verbose -e PODVM_GUEST_EXE=/bin/echo "$DEBIAN" echo exeforce-hi
grep -q "PODVM_GUEST_EXE" "$WORK/out-fx-exeforce.txt" || { echo "exe-force banner missing" >>"$REPORT"; fail=1; }
grep -q "exeforce-hi" "$WORK/out-fx-exeforce.txt" || { echo "exe-force payload stdout missing" >>"$REPORT"; fail=1; }

# Clause 13 (T-1413, T-1649 script): a top-level `#!` payload runs its
# interpreter from the image, with the script path as its argument. An
# image with no /bin/sh refuses at 126 naming the resolved path and its
# `#!` line; podvm never takes the interpreter from the host.
step fx-script "$NOINVOKE" $FX "$PB" run --rm script-img:1 /hello.sh
grep -q "/hello.sh" "$WORK/out-fx-script.txt" || { echo "script refusal did not name the path" >>"$REPORT"; fail=1; }
grep -q "#!/bin/sh" "$WORK/out-fx-script.txt" || { echo "script refusal did not name the shebang" >>"$REPORT"; fail=1; }
step fx-script-run 0 $FX "$PB" run --rm script-alpine:1 /hello.sh arg1
grep -qx "script-hi /hello.sh arg1" "$WORK/out-fx-script-run.txt" || { echo "a #!/bin/sh script of the image did not run" >>"$REPORT"; fail=1; }
# The node image with a caller command and no --entrypoint (issue 92):
# the #! entrypoint runs, or the refusal names `--entrypoint node`.
timeout 120 $FX "$PB" run --rm "$NODE" node -e 'console.log("entry-ok")' >"$WORK/out-fx-node-entry.txt" 2>&1
rc=$?
{ echo ""; echo "== fx-node-entry"; echo "exit              $rc"; sed 's/^/  /' "$WORK/out-fx-node-entry.txt"; } >>"$REPORT"
if [ "$rc" -eq 0 ]; then
	grep -qx "entry-ok" "$WORK/out-fx-node-entry.txt" || { echo "the node entrypoint run did not reach the payload" >>"$REPORT"; fail=1; }
else
	grep -q -- "--entrypoint node" "$WORK/out-fx-node-entry.txt" || { echo "the entrypoint refusal did not name --entrypoint node" >>"$REPORT"; fail=1; }
fi

# Clause 14 (T-1416 quiet): the banner never prints at default and stays
# off under `-q`; a refusal still prints. Split streams per case: stdout
# stays empty outright (the payload owns it), and neither stream carries
# a banner line. ⚠ Extraction progress rides stderr through the
# extract-side `out` on every run (pre-existing: the committed results
# show it too), so the clause asserts banner-silence, not total
# silence: total silence is the extract side's to prove.
quiet_banner() {
	grep -q "mode=userland\|podvm: userland:\|podvm: interpose:\|podvm: complete:\|entering without chroot" "$1" \
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
# PODVM_GUEST_EXE even where the real call succeeds, with no extra -e.
# (A python proof was considered; bookworm-slim holds no python3. The
# readlink shape is the exact code the entry changed, so it proves the
# mechanism directly.)
# The golang image's `go` binary is statically linked (measured with
# file(1): ELF static, no INTERP). T-1648 refused it at 125 where chroot
# is denied; ⭐ TODO/enter.md T-1652 runs it on the native-static rung
# (the go clauses after the readlink one).
step fx-exe-readlink 0 $FX "$PB" run --rm "$DEBIAN" readlink /proc/self/exe
# The answer line is exactly the guest path. An earlier whole-file grep
# for "rootfs" false-fired on podvm's own extraction diagnostics (lane
# job 947e9e1e6ad11d71: the answer was /usr/bin/readlink while stderr
# carried store/rootfs/ lines), so the assertion matches the full line:
# a leaked loader host path would BE the answer and fail this match.
grep -qx "/usr/bin/readlink" "$WORK/out-fx-exe-readlink.txt" || { echo "readlink did not answer exactly the guest path" >>"$REPORT"; fail=1; }
# ⭐ T-1652: go and its compile and link children run on the
# native-static rung. The source comes from the image's own dash on the
# loader family, which writes inside the rootfs; the build output lands
# in the rootfs, and the host keeps no /tmp/g356. The three runs keep
# their records (no --rm): a run with --rm extracts the image again, so
# a write of one run is not in the next (measured, 2026-11-11).
rm -rf /tmp/g356
step fx-go-version 0 $FX "$PB" run --rm "$GOLANG" go version
grep -q "^go version go1\.24\.7 linux/amd64$" "$WORK/out-fx-go-version.txt" || { echo "go version did not answer on the native-static rung" >>"$REPORT"; fail=1; }
step fx-go-src 0 $FX "$PB" run --name g356src "$GOLANG" sh -c 'mkdir -p /tmp/g356 && printf "package main\nimport \"fmt\"\nfunc main() { fmt.Println(\"g356-ok\") }\n" > /tmp/g356/main.go'
step fx-go-build 0 $FX "$PB" run --name g356build -w /tmp/g356 -e GOTOOLCHAIN=local "$GOLANG" go build -o /tmp/g356/h main.go
step fx-go-run 0 $FX "$PB" run --name g356run "$GOLANG" /tmp/g356/h
grep -qx "g356-ok" "$WORK/out-fx-go-run.txt" || { echo "the program that go built did not print its line" >>"$REPORT"; fail=1; }
[ -e /tmp/g356 ] && { echo "the go build wrote on the host" >>"$REPORT"; fail=1; }
$FX "$PB" rm g356src g356build g356run >/dev/null 2>&1
step fx-dispatch-probe 0 $FX "$PB" probe --json
grep -q '"user_dispatch":true' "$WORK/out-fx-dispatch-probe.txt" || { echo "the probe did not report measured_by.user_dispatch true" >>"$REPORT"; fail=1; }

# Clause 19 (T-1408 gcc): the loader symlink resolves inside the rootfs
# on a second image. Debian's /lib64/ld-linux carries an absolute
# target dangling on the host; the gcc image exercises the same
# resolve_in path as clause 4 with its own loader and libraries.
step fx-gcc-version 0 $FX "$PB" run --rm "$GCC" gcc --version
# T-1646: gcc compiles and the output runs, with cc1 and ld as children
# that the exec family routes through the image's loader.
step fx-gcc-build 0 $FX "$PB" run --rm "$GCC" sh -c 'printf "#include <stdio.h>\nint main(void){puts(\"h-ok\");return 0;}\n" > /tmp/h.c && gcc /tmp/h.c -o /tmp/h && /tmp/h'
grep -q "^h-ok$" "$WORK/out-fx-gcc-build.txt" || { echo "gcc build did not run its output" >>"$REPORT"; fail=1; }
grep -q "gcc" "$WORK/out-fx-gcc-version.txt" || { echo "gcc --version did not answer" >>"$REPORT"; fail=1; }

# Clause 20 (T-1643 relative paths): a relative read, a relative write
# under -w /tmp, a dirfd-relative openat and `cd /; echo x > rel.txt`
# each land inside the rootfs. Each payload reads its write back by the
# absolute guest path, and the host keeps none of the files. Before
# T-1643 each step missed with ENOENT: the join kept the host cwd, and
# the `/` pair added the rootfs prefix a second time.
rm -f /rel.txt /tmp/ul356-relw.txt /tmp/ul356-dirfd.txt
step fx-relread 0 $FX "$PB" run --rm -w /etc "$ALPINE" sh -c 'read r < alpine-release && echo "rel=$r"'
grep -q "^rel=3\.20" "$WORK/out-fx-relread.txt" || { echo "relative read did not see the image release" >>"$REPORT"; fail=1; }
step fx-relwrite 0 $FX "$PB" run --rm -w /tmp "$ALPINE" sh -c 'echo w356 > ul356-relw.txt && read y < /tmp/ul356-relw.txt && echo "back=$y"'
grep -qx "back=w356" "$WORK/out-fx-relwrite.txt" || { echo "relative write under -w /tmp did not read back" >>"$REPORT"; fail=1; }
step fx-dirfd 0 $FX "$PB" run --rm "$PYTHON" python3 -c 'import os
d = os.open("/tmp", os.O_RDONLY | os.O_DIRECTORY)
h = os.open("ul356-dirfd.txt", os.O_WRONLY | os.O_CREAT, 0o644, dir_fd=d)
os.write(h, b"d356"); os.close(h)
print("dirfd-back=" + open("/tmp/ul356-dirfd.txt").read())'
grep -qx "dirfd-back=d356" "$WORK/out-fx-dirfd.txt" || { echo "dirfd-relative openat did not land in the rootfs" >>"$REPORT"; fail=1; }
step fx-cdroot 0 $FX "$PB" run --rm -w /tmp "$ALPINE" sh -c 'cd / && echo x356 > rel.txt && read z < /rel.txt && echo "root=$z"'
grep -qx "root=x356" "$WORK/out-fx-cdroot.txt" || { echo "cd / then a relative write did not read back" >>"$REPORT"; fail=1; }
for f in /rel.txt /tmp/ul356-relw.txt /tmp/ul356-dirfd.txt; do
	[ -e "$f" ] && { echo "relative write escaped to host $f" >>"$REPORT"; fail=1; }
done
rm -f /rel.txt /tmp/ul356-relw.txt /tmp/ul356-dirfd.txt

# Clause 21 (T-1645 dlopen): a C extension loads by its guest path. The
# interposer's `dlopen` routes a name with a `/` through the table, so
# `import subprocess` loads `_posixsubprocess` from the rootfs. A bare
# soname stays a search in the loader list.
step fx-dlopen 0 $FX "$PB" run --rm "$PYTHON" python3 -c 'import subprocess, _posixsubprocess; print("import-ok")'
grep -qx "import-ok" "$WORK/out-fx-dlopen.txt" || { echo "python could not import subprocess through dlopen" >>"$REPORT"; fail=1; }
step fx-dlopen-soname 0 $FX "$PB" run --rm "$PYTHON" python3 -c 'import ctypes; ctypes.CDLL("libffi.so.8"); print("soname-ok")'
grep -qx "soname-ok" "$WORK/out-fx-dlopen-soname.txt" || { echo "a bare soname did not load through dlopen" >>"$REPORT"; fail=1; }

# Clause 22 (T-1644 glibc closure): debian and gcc run at 0 with the glibc
# object preloaded. The admission reads the object's whole DT_NEEDED
# closure and the interpreter inside the rootfs, so the image's absolute
# /lib64 loader link never reads a host file. beta.12 declined both at 126
# naming `__tls_get_addr`.
step fx-glibc-echo 0 $FX "$PB" run --rm --verbose "$DEBIAN" echo hi
grep -qx "hi" "$WORK/out-fx-glibc-echo.txt" || { echo "debian echo hi did not answer" >>"$REPORT"; fail=1; }
grep -q "interpose.so (glibc object) is preloaded" "$WORK/out-fx-glibc-echo.txt" || { echo "debian ran without the glibc object" >>"$REPORT"; fail=1; }
step fx-glibc-gcc 0 $FX "$PB" run --rm --verbose "$GCC" gcc --version
grep -q "^gcc" "$WORK/out-fx-glibc-gcc.txt" || { echo "gcc --version did not answer with the object" >>"$REPORT"; fail=1; }
grep -q "interpose.so (glibc object) is preloaded" "$WORK/out-fx-glibc-gcc.txt" || { echo "gcc ran without the glibc object" >>"$REPORT"; fail=1; }

# Clause 23 (T-1646 grandchild exec): a dynamic grandchild enters
# through the image's loader inside the rootfs, never the host's. Before
# T-1646 debian's `/bin/ls` grandchild ran under the host loader and
# aborted (`stack smashing detected`, 134), and alpine's failed 127.
step fx-gc-debian 0 $FX "$PB" run --rm "$DEBIAN" sh -c '/bin/ls /'
grep -qx "etc" "$WORK/out-fx-gc-debian.txt" || { echo "debian /bin/ls grandchild did not list /" >>"$REPORT"; fail=1; }
step fx-gc-alpine 0 $FX "$PB" run --rm "$ALPINE" sh -c '/bin/ls / && echo pipe-ok | /bin/cat'
grep -qx "etc" "$WORK/out-fx-gc-alpine.txt" || { echo "alpine /bin/ls applet did not list /" >>"$REPORT"; fail=1; }
grep -qx "pipe-ok" "$WORK/out-fx-gc-alpine.txt" || { echo "alpine pipeline did not run" >>"$REPORT"; fail=1; }
# A bare name: ash finds it with stat, through `/bin/chmod -> /bin/busybox`.
step fx-gc-bare 0 $FX "$PB" run --rm "$ALPINE" sh -c 'ls -d /etc && env -- ls -d /bin'
grep -qx "/etc" "$WORK/out-fx-gc-bare.txt" || { echo "a bare-name grandchild did not run" >>"$REPORT"; fail=1; }
grep -qx "/bin" "$WORK/out-fx-gc-bare.txt" || { echo "env did not find ls on PATH" >>"$REPORT"; fail=1; }
step fx-gc-script 0 $FX "$PB" run --rm "$ALPINE" sh -c 'printf "#!/bin/sh -e\necho script-ok \"\$0\" \"\$1\"\n" > /tmp/s.sh && chmod +x /tmp/s.sh && /tmp/s.sh arg1'
grep -qx "script-ok /tmp/s.sh arg1" "$WORK/out-fx-gc-script.txt" || { echo "a #! script grandchild did not run" >>"$REPORT"; fail=1; }
step fx-gc-envscript 0 $FX "$PB" run --rm "$DEBIAN" sh -c 'printf "#!/usr/bin/env sh\necho env-script-ok \"\$1\"\n" > /tmp/s.sh && chmod +x /tmp/s.sh && /tmp/s.sh arg1'
grep -qx "env-script-ok arg1" "$WORK/out-fx-gc-envscript.txt" || { echo "a #!/usr/bin/env script did not run" >>"$REPORT"; fail=1; }
step fx-gc-exe 0 $FX "$PB" run --rm "$DEBIAN" sh -c 'readlink /proc/self/exe'
grep -qx "/usr/bin/readlink" "$WORK/out-fx-gc-exe.txt" || { echo "a grandchild did not read its own guest exe" >>"$REPORT"; fail=1; }
# A-V-04: node starts a shell, and the shell starts ls. `--entrypoint`
# keeps this clause on node alone; clause 13 drives the #! entrypoint
# of the image (T-1649).
step fx-node-version 0 $FX "$PB" run --rm --entrypoint node "$NODE" --version
grep -q "^v22\." "$WORK/out-fx-node-version.txt" || { echo "node --version did not print" >>"$REPORT"; fail=1; }
step fx-node-execsync 0 $FX "$PB" run --rm --entrypoint node "$NODE" -e "process.stdout.write(require('child_process').execSync('ls /').toString())"
grep -qx "etc" "$WORK/out-fx-node-execsync.txt" || { echo "node execSync('ls /') did not list /" >>"$REPORT"; fail=1; }

# Clause 24 (T-1650 strict capability): `--strict` names each gap of
# the loader family. It refuses a gap that the command hits and warns
# on a gap that podvm cannot tell; `--strict=all` (clause 6) refuses
# each. T-1646 routes the shell grandchild, so issue 93's command runs.
step fx-strict-ls 0 $FX "$PB" run --rm --strict "$ALPINE" /bin/sh -c '/bin/ls /'
grep -qx "etc" "$WORK/out-fx-strict-ls.txt" || { echo "--strict sh -c '/bin/ls /' did not list /" >>"$REPORT"; fail=1; }
grep -q "strict warning \[capability\].*T-1651" "$WORK/out-fx-strict-ls.txt" || { echo "--strict did not warn on the T-1651 gap" >>"$REPORT"; fail=1; }
step fx-strict-echo 0 $FX "$PB" run --rm --strict "$ALPINE" /bin/echo hi
grep -qx "hi" "$WORK/out-fx-strict-echo.txt" || { echo "--strict /bin/echo hi did not print" >>"$REPORT"; fail=1; }
# T-1724: a static child takes the native-static rung where Syscall User
# Dispatch arms, so --strict runs it; with the prctl denied it hits the gap.
step fx-strict-static 0 $FX "$PB" run --rm --strict script-alpine:1 /bin/sh -c '/hello'
grep -q "static-hi" "$WORK/out-fx-strict-static.txt" || { echo "--strict did not run the static child on the native-static rung" >>"$REPORT"; fail=1; }
step fx-strict-static-denied "$RUN_ERR" $FXD "$PB" run --rm --strict script-alpine:1 /bin/sh -c '/hello'
grep -q "\[capability\].*T-1651.*This command hits it" "$WORK/out-fx-strict-static-denied.txt" || { echo "--strict did not refuse the static child naming T-1651" >>"$REPORT"; fail=1; }
grep -q "static-hi" "$WORK/out-fx-strict-static-denied.txt" && { echo "--strict ran the static child with the prctl denied" >>"$REPORT"; fail=1; }
step fx-strict-npm 0 $FX "$PB" run --rm --strict "$NODE" npm --version
grep -qx "[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*" "$WORK/out-fx-strict-npm.txt" || { echo "--strict npm --version did not print a version" >>"$REPORT"; fail=1; }
grep -q "T-1697" "$WORK/out-fx-strict-npm.txt" && { echo "--strict still names the closed T-1697 gap" >>"$REPORT"; fail=1; }

# Clause 25 (T-1697 raw statx): libuv stats a path through libc's
# `syscall()`, and the interposer maps that number through the table.
# Before T-1697, `statSync` and `lstatSync` of a guest path missed with
# ENOENT on the host, and npm could not find its own modules. The
# project has no dependencies, so `npm ci` needs no registry. The image
# is the node rootfs with the project at /app, imported as clause 13
# imports its script image.
step fx-node-stat 0 $FX "$PB" run --rm --entrypoint node "$NODE" -e 'const fs = require("fs"); console.log("stat=" + fs.statSync("/etc/alpine-release").isFile()); console.log("lstat=" + fs.lstatSync("/usr/local/lib/node_modules").isDirectory())'
grep -qx "stat=true" "$WORK/out-fx-node-stat.txt" || { echo "node statSync of a guest file did not answer from the rootfs" >>"$REPORT"; fail=1; }
grep -qx "lstat=true" "$WORK/out-fx-node-stat.txt" || { echo "node lstatSync of a guest directory did not answer from the rootfs" >>"$REPORT"; fail=1; }
timeout 600 "$PB" extract "$NODE" >>"$REPORT" 2>&1 || { echo "node extract FAILED" | tee -a "$REPORT"; exit 1; }
NROOT=$("$PB" inspect --format '{{.RootfsPath}}' "$NODE" 2>/dev/null) || NROOT=""
[ -d "$NROOT" ] || { echo "node rootfs missing: FAILED" | tee -a "$REPORT"; exit 1; }
mkdir -p "$WORK/npm-root/app" || exit 2
cat >"$WORK/npm-root/app/package.json" <<'EOF'
{
  "name": "p1697",
  "version": "1.0.0",
  "private": true,
  "scripts": {
    "postinstall": "node -e \"process.stdout.write(require('child_process').execSync('echo postinstall-child-ok').toString())\"",
    "test": "node test.js"
  }
}
EOF
cat >"$WORK/npm-root/app/package-lock.json" <<'EOF'
{
  "name": "p1697",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "": {
      "name": "p1697",
      "version": "1.0.0",
      "hasInstallScript": true
    }
  }
}
EOF
cat >"$WORK/npm-root/app/test.js" <<'EOF'
const fs = require("fs");
const s = fs.statSync("/usr/local/lib/node_modules/npm/package.json");
fs.lstatSync("/usr/local/lib/node_modules");
console.log("npm-test-ok " + (s.size > 0));
EOF
tar -cf "$WORK/node-app.tar" -C "$NROOT" . -C "$WORK/npm-root" app 2>>"$REPORT" \
	|| { echo "node-app tar      FAILED" | tee -a "$REPORT"; exit 2; }
if timeout 600 "$PB" import "$WORK/node-app.tar" node-app:1697 >>"$REPORT" 2>&1; then
	echo "node-app import   ok" >>"$REPORT"
else
	echo "node-app import   FAILED" | tee -a "$REPORT"
	exit 1
fi
step fx-npm-ci 0 $FX "$PB" run --rm -w /app -e PATH=/usr/local/bin:/usr/bin:/bin -e HOME=/root -e npm_config_update_notifier=false node-app:1697 /bin/sh -c 'npm ci --no-audit --no-fund && npm test'
grep -qx "postinstall-child-ok" "$WORK/out-fx-npm-ci.txt" || { echo "npm ci did not run the postinstall child" >>"$REPORT"; fail=1; }
grep -qx "npm-test-ok true" "$WORK/out-fx-npm-ci.txt" || { echo "npm test did not run" >>"$REPORT"; fail=1; }

# Clause 26 (T-1653 volumes): each -v is one GUEST:HOST pair of the
# PODVM_MAPS table beside the root pair, served by the loader family.
# Before T-1653 each -v refused with status None. The noexec wrap mounts a
# noexec tmpfs on its second argument inside the namespace first.
VOL="$WORK/vol-host"; NX="$WORK/vol-nx"
rm -rf "$VOL" "$NX"; mkdir -p "$VOL" "$NX" || exit 2
printf 'vol-in\n' >"$VOL/in.txt" || exit 2
cat >"$WORK/wrap-nx.sh" <<'EOF'
#!/bin/sh
# As wrap.sh, with a noexec tmpfs on $2 holding one file.
mount -t tmpfs tmpfs /dev/pts || exit 2
mount -t tmpfs -o noexec tmpfs "$2" || exit 2
printf 'nx-data\n' >"$2/in.txt" || exit 2
MNT="$1"; shift 2
exec "$MNT/deny-chroot" "$@"
EOF
step fx-vol-read 0 $FX "$PB" run --rm --verbose -v "$VOL:/data" "$ALPINE" cat /data/in.txt
grep -qx "vol-in" "$WORK/out-fx-vol-read.txt" || { echo "a volume read did not see the host file" >>"$REPORT"; fail=1; }
grep -q "is a path rewrite through PODVM_MAPS" "$WORK/out-fx-vol-read.txt" || { echo "the banner did not name the volume" >>"$REPORT"; fail=1; }
step fx-vol-write 0 $FX "$PB" run --rm -w /tmp -v "$VOL:/data" "$ALPINE" sh -c 'echo w356 > /data/out.txt && cd /data && echo r356 > rel.txt && pwd && sh -c "cat /data/in.txt"'
grep -qx "w356" "$VOL/out.txt" 2>/dev/null || { echo "a guest write did not land in the host directory" >>"$REPORT"; fail=1; }
grep -qx "r356" "$VOL/rel.txt" 2>/dev/null || { echo "a relative write in the volume did not land in the host directory" >>"$REPORT"; fail=1; }
grep -qx "/data" "$WORK/out-fx-vol-write.txt" || { echo "pwd in the volume did not read /data" >>"$REPORT"; fail=1; }
grep -qx "vol-in" "$WORK/out-fx-vol-write.txt" || { echo "a shell grandchild did not read the volume" >>"$REPORT"; fail=1; }
# -w and the first program resolve through the table (pod-enter of
# podcrates v0.12.0): the volume serves each, not the image.
step fx-vol-workdir 0 $FX "$PB" run --rm -w /data -v "$VOL:/data" "$ALPINE" sh -c 'pwd && echo wd356 > wd.txt'
grep -qx "/data" "$WORK/out-fx-vol-workdir.txt" || { echo "-w in the volume did not start in /data" >>"$REPORT"; fail=1; }
grep -qx "wd356" "$VOL/wd.txt" 2>/dev/null || { echo "a relative write under -w did not land in the host directory" >>"$REPORT"; fail=1; }
step fx-vol-copy 0 $FX "$PB" run --rm -v "$VOL:/data" "$ALPINE" cp /bin/busybox /data/busybox
step fx-vol-prog 0 $FX "$PB" run --rm -v "$VOL:/data" "$ALPINE" /data/busybox echo vol-prog
grep -qx "vol-prog" "$WORK/out-fx-vol-prog.txt" || { echo "a first program in the volume did not run" >>"$REPORT"; fail=1; }
# T-1655: a `:ro` volume reads, and a write in it fails with EROFS and
# makes no host file.
step fx-vol-ro 0 $FX "$PB" run --rm -v "$VOL:/data:ro" "$ALPINE" sh -c 'cat /data/in.txt; touch /data/ro356 2>&1; echo done'
grep -qx "vol-in" "$WORK/out-fx-vol-ro.txt" || { echo "a :ro volume did not read" >>"$REPORT"; fail=1; }
grep -q "Read-only file system" "$WORK/out-fx-vol-ro.txt" || { echo "a write in a :ro volume did not fail with EROFS" >>"$REPORT"; fail=1; }
[ -e "$VOL/ro356" ] && { echo "a write in a :ro volume reached the host" >>"$REPORT"; fail=1; }
step fx-vol-memfd "$RUN_ERR" $FX "$PB" run --rm --unsafe-host-paths -v "$VOL:/data" script-alpine:1 /hello
grep -q "memfd family" "$WORK/out-fx-vol-memfd.txt" || { echo "the memfd family did not refuse the volume" >>"$REPORT"; fail=1; }
grep -q "static-hi" "$WORK/out-fx-vol-memfd.txt" && { echo "the memfd family ran with a volume" >>"$REPORT"; fail=1; }
step fx-vol-noexec 0 unshare -Urm sh "$WORK/wrap-nx.sh" "$WORK" "$NX" "$PB" run --rm --verbose -v "$NX:/nx" "$ALPINE" cat /nx/in.txt
grep -qx "nx-data" "$WORK/out-fx-vol-noexec.txt" || { echo "the noexec volume did not read" >>"$REPORT"; fail=1; }
grep -q "is on a noexec mount" "$WORK/out-fx-vol-noexec.txt" || { echo "the banner did not name the noexec source" >>"$REPORT"; fail=1; }

# Clause 27 (T-1712, T-1713, T-1716): the entered rung, as a caller
# reads it before a run, and the volume rules of pod-enter v0.15.0.
step fx-tier-chroot "$FLAG" $FX "$PB" run --rm --podvm-tier=chroot "$ALPINE" true
grep -q 'takes machine or incus, not "chroot"' "$WORK/out-fx-tier-chroot.txt" || { echo "--podvm-tier=chroot was not refused by name" >>"$REPORT"; fail=1; }
step fx-entered 0 $FX "$PB" system info --format '{{.Rung}} {{.EnteredRung}}'
grep -qE '^[a-z]+ userland$' "$WORK/out-fx-entered.txt" || { echo ".EnteredRung did not read userland under the fixture" >>"$REPORT"; fail=1; }
echo "fixture rungs     $(tail -1 "$WORK/out-fx-entered.txt")" >>"$REPORT"
step fx-vol-proc "$FLAG" $FX "$PB" run --rm -v "$VOL:/proc/x" "$ALPINE" true
grep -q "never rewrites a path under /proc" "$WORK/out-fx-vol-proc.txt" || { echo "a volume under /proc was not refused by name" >>"$REPORT"; fail=1; }
step fx-vol-wd-missing "$RUN_ERR" $FX "$PB" run --rm -w /data/none -v "$VOL:/data" "$ALPINE" true
grep -q ":/data is not a directory in the volume" "$WORK/out-fx-vol-wd-missing.txt" || { echo "a missing -w in a volume did not name the volume" >>"$REPORT"; fail=1; }

# Clause 28 (T-1704): realpath of the exe names the program, as readlink
# does. coreutils reads /proc/PID/exe after it follows /proc/self.
step fx-realpath-exe 0 $FX "$PB" run --rm "$DEBIAN" realpath /proc/self/exe
grep -qx "/usr/bin/realpath" "$WORK/out-fx-realpath-exe.txt" || { echo "realpath of /proc/self/exe did not name the program" >>"$REPORT"; fail=1; }
mkdir -p "$WORK/fort" || exit 2
cat >"$WORK/fort.c" <<'FORT'
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
int main(int argc, char **argv) {
	size_t n = (size_t)atoi(argc > 1 ? argv[1] : "4000");
	char rp[PATH_MAX], rl[PATH_MAX];
	char *r = realpath("/proc/self/exe", rp);
	ssize_t k = readlink("/proc/self/exe", rl, n);
	if (k >= 0) rl[k] = 0;
	printf("realpath %s\nreadlink %s\n", r ? r : "(null)", k >= 0 ? rl : "(err)");
	return 0;
}
FORT
if cc -O2 -U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=2 -o "$WORK/fort/fort" "$WORK/fort.c" 2>>"$REPORT"; then
	echo "fortified program $(nm -D "$WORK/fort/fort" | grep -oE '__(realpath|readlink)_chk' | sort -u | tr '\n' ' ')" >>"$REPORT"
	step fx-fortified 0 $FX "$PB" run --rm -v "$WORK/fort:/fort" "$DEBIAN" /fort/fort
	grep -qx "realpath /fort/fort" "$WORK/out-fx-fortified.txt" || { echo "a fortified realpath did not name the program" >>"$REPORT"; fail=1; }
	grep -qx "readlink /fort/fort" "$WORK/out-fx-fortified.txt" || { echo "a fortified readlink did not name the program" >>"$REPORT"; fail=1; }
else
	echo "fortified program COMPILE FAILED" | tee -a "$REPORT"; fail=1
fi

# Clause 29 (T-1705): a path that the loader gives is a host path under
# the rootfs, and an open of it is not prefixed a second time.
timeout 600 "$PB" extract "$DEBIAN" >>"$REPORT" 2>&1 || { echo "debian extract FAILED" | tee -a "$REPORT"; fail=1; }
DROOT=$("$PB" inspect --format '{{.RootfsPath}}' "$DEBIAN" 2>/dev/null) || DROOT=""
mkdir -p "$WORK/dl-root/usr/lib/x86_64-linux-gnu" "$WORK/dl-root/usr/bin" || exit 2
printf 'int x356(void) { return 7; }\n' >"$WORK/x356.c"
cat >"$WORK/dl356.c" <<'DL356'
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdio.h>
int x356(void);
int main(void) {
	Dl_info info;
	if (!dladdr((void *)x356, &info)) { puts("dladdr failed"); return 2; }
	int fd = open(info.dli_fname, O_RDONLY);
	printf("dladdr %s\nopen %s\n", info.dli_fname, fd >= 0 ? "ok" : "failed");
	return fd >= 0 ? 0 : 1;
}
DL356
if [ -d "$DROOT" ] &&
	cc -shared -fPIC -o "$WORK/dl-root/usr/lib/x86_64-linux-gnu/libx356.so" "$WORK/x356.c" 2>>"$REPORT" &&
	cc -o "$WORK/dl-root/usr/bin/dl356" "$WORK/dl356.c" -L"$WORK/dl-root/usr/lib/x86_64-linux-gnu" -lx356 2>>"$REPORT" &&
	tar -cf "$WORK/dl-debian.tar" -C "$DROOT" . -C "$WORK/dl-root" usr/lib/x86_64-linux-gnu/libx356.so usr/bin/dl356 2>>"$REPORT" &&
	timeout 300 "$PB" import "$WORK/dl-debian.tar" dl-debian:356 >>"$REPORT" 2>&1; then
	step fx-dladdr 0 $FX "$PB" run --rm dl-debian:356 /usr/bin/dl356
	grep -qx "open ok" "$WORK/out-fx-dladdr.txt" || { echo "the path from dladdr did not open" >>"$REPORT"; fail=1; }
	grep -q "^dladdr .*/rootfs/.*lib/x86_64-linux-gnu/libx356.so$" "$WORK/out-fx-dladdr.txt" || { echo "dladdr did not name the library" >>"$REPORT"; fail=1; }
else
	echo "dladdr image     BUILD FAILED" | tee -a "$REPORT"; fail=1
fi

# Clause 30 (T-1655): the read-only layer of the interposer.
step fx-read-only 0 $FX "$PB" run --rm --read-only -v "$VOL:/workspace" "$ALPINE" sh -c 'touch /etc/ro356 2>&1; echo w > /workspace/rw356 && echo rw-ok; cat /etc/hostname >/dev/null && echo read-ok'
grep -q "Read-only file system" "$WORK/out-fx-read-only.txt" || { echo "a write to the image under --read-only did not fail with EROFS" >>"$REPORT"; fail=1; }
grep -qx "rw-ok" "$WORK/out-fx-read-only.txt" || { echo "a write to a volume under --read-only failed" >>"$REPORT"; fail=1; }
grep -qx "read-ok" "$WORK/out-fx-read-only.txt" || { echo "a read under --read-only failed" >>"$REPORT"; fail=1; }
grep -qx "w" "$VOL/rw356" 2>/dev/null || { echo "the volume write did not land in the host directory" >>"$REPORT"; fail=1; }
step sane-read-only "$RUN_ERR" "$PB" run --rm --read-only "$ALPINE" true
grep -q "read-only layer" "$WORK/out-sane-read-only.txt" || { echo "--read-only on the capable rung was not refused by name" >>"$REPORT"; fail=1; }

# Clause 31 (T-1707): a relative program resolves against -w.
# A child of the relative program needs the interposer: without it the
# kernel runs the host's ls, which has no /etc/alpine-release.
step fx-workdir-rel 0 $FX "$PB" run --rm -w /bin "$ALPINE" ./busybox sh -c 'ls /etc/alpine-release'
grep -qx "/etc/alpine-release" "$WORK/out-fx-workdir-rel.txt" || { echo "a relative program ran without the image's view" >>"$REPORT"; fail=1; }
printf '#!/bin/sh\necho vol-workdir-ok\n' >"$VOL/tool356.sh"; chmod +x "$VOL/tool356.sh"
step fx-workdir-vol 0 $FX "$PB" run --rm -v "$VOL:/data" -w /data "$ALPINE" ./tool356.sh
grep -qx "vol-workdir-ok" "$WORK/out-fx-workdir-vol.txt" || { echo "a relative script of a volume workdir did not run" >>"$REPORT"; fail=1; }

# Clause 32 (T-1719): the library directories of the image.
L="$WORK/t1719"; mkdir -p "$L/musl/usr/local/lib" "$L/glibc/opt/t1719/lib" "$L/glibc/etc/ld.so.conf.d"
printf 'int t1719_answer(void){return 42;}\n' >"$L/foo.c"
printf '#include <stdio.h>\nint t1719_answer(void);\nint main(void){printf("lib-%%d\\n", t1719_answer());return 0;}\n' >"$L/main.c"
if zig cc -target x86_64-linux-musl -shared -fPIC -Wl,-soname,libt1719.so -o "$L/musl/usr/local/lib/libt1719.so" "$L/foo.c" 2>>"$REPORT" &&
	zig cc -target x86_64-linux-musl -dynamic -o "$L/musl/uselib" "$L/main.c" -L"$L/musl/usr/local/lib" -lt1719 2>>"$REPORT" &&
	cc -shared -fPIC -Wl,-soname,libt1719.so -o "$L/glibc/opt/t1719/lib/libt1719.so" "$L/foo.c" 2>>"$REPORT" &&
	cc -o "$L/glibc/uselib" "$L/main.c" -L"$L/glibc/opt/t1719/lib" -lt1719 2>>"$REPORT"; then
	echo /opt/t1719/lib >"$L/glibc/etc/ld.so.conf.d/t1719.conf"
	# An earlier --rm run removed each rootfs that no record names.
	"$PB" extract "$ALPINE" >/dev/null 2>&1
	AROOT=$("$PB" inspect --format '{{.RootfsPath}}' "$ALPINE" 2>/dev/null) || AROOT=""
	"$PB" extract "$DEBIAN" >/dev/null 2>&1
	DROOT=$("$PB" inspect --format '{{.RootfsPath}}' "$DEBIAN" 2>/dev/null) || DROOT=""
	tar -cf "$L/musl.tar" -C "$AROOT" . -C "$L/musl" ./usr/local/lib/libt1719.so ./uselib 2>>"$REPORT" &&
		"$PB" import "$L/musl.tar" t1719-musl:1 >/dev/null 2>>"$REPORT" ||
		{ echo "the musl image of clause 32 did not build" >>"$REPORT"; fail=1; }
	if [ -d "$DROOT" ]; then
		tar -cf "$L/glibc.tar" -C "$DROOT" . -C "$L/glibc" ./opt/t1719/lib/libt1719.so ./etc/ld.so.conf.d/t1719.conf ./uselib 2>>"$REPORT" &&
			"$PB" import "$L/glibc.tar" t1719-glibc:1 >/dev/null 2>>"$REPORT" ||
			{ echo "the glibc image of clause 32 did not build" >>"$REPORT"; fail=1; }
	else
		echo "the debian rootfs of clause 32 is missing" >>"$REPORT"; fail=1
	fi
	step fx-libdir-musl 0 $FX "$PB" run --rm t1719-musl:1 /uselib
	grep -qx "lib-42" "$WORK/out-fx-libdir-musl.txt" || { echo "a library in /usr/local/lib was not found" >>"$REPORT"; fail=1; }
	step fx-libdir-glibc 0 $FX "$PB" run --rm t1719-glibc:1 /uselib
	grep -qx "lib-42" "$WORK/out-fx-libdir-glibc.txt" || { echo "a library of ld.so.conf.d was not found" >>"$REPORT"; fail=1; }
else
	echo "the probes of clause 32 did not build" >>"$REPORT"; fail=1
fi

# Clause 33 (T-1724): a static child of the loader family, and a volume,
# on the native-static rung. The probe is the static test payload of
# podvm-dispatch, which the build above made.
rm -rf /tmp/g724 /go
step fx-go-child 0 $FX "$PB" run --name g724 "$GOLANG" sh -c 'mkdir -p /tmp/g724 && cd /tmp/g724 && printf "package main\nimport \"fmt\"\nfunc main() { fmt.Println(\"g724-ok\") }\n" > main.go && GOTOOLCHAIN=local go build -o h main.go && ./h'
grep -qx "g724-ok" "$WORK/out-fx-go-child.txt" || { echo "go build under sh did not run its program" >>"$REPORT"; fail=1; }
{ [ -e /tmp/g724 ] || [ -e /go ]; } && { echo "go build under sh wrote on the host" >>"$REPORT"; fail=1; }
$FX "$PB" rm g724 >/dev/null 2>&1
V7="$WORK/vol724"; rm -rf "$V7" "$WORK/img724"; mkdir -p "$V7" "$WORK/img724/etc" "$WORK/img724/tmp"
echo invol >"$V7/f"
if cp "$REPO/target/x86_64-unknown-linux-musl/debug/podvm-dispatch-probe" "$WORK/img724/probe" &&
	(cd "$WORK/img724" && tar -cf "$WORK/img724.tar" .) && "$PB" import "$WORK/img724.tar" probe724:1 >/dev/null 2>>"$REPORT"; then
	step fx-vol-native-read 0 $FX "$PB" run --rm --verbose -v "$V7:/data" probe724:1 /probe cat /data/f
	grep -qx "invol" "$WORK/out-fx-vol-native-read.txt" || { echo "a static payload did not read its volume" >>"$REPORT"; fail=1; }
	grep -q "native-static rung" "$WORK/out-fx-vol-native-read.txt" || { echo "the volume run was not on the native-static rung" >>"$REPORT"; fail=1; }
	step fx-vol-native-write 0 $FX "$PB" run --rm -v "$V7:/data" probe724:1 /probe write /data/a w724 /data/b
	grep -qx "w724" "$V7/b" 2>/dev/null || { echo "a write of a static payload did not land in the volume" >>"$REPORT"; fail=1; }
	step fx-vol-native-cwd 0 $FX "$PB" run --rm -w /data -v "$V7:/data" probe724:1 /probe cwd
	grep -qx "/data" "$WORK/out-fx-vol-native-cwd.txt" || { echo "the working directory in a volume did not read back as /data" >>"$REPORT"; fail=1; }
else
	echo "the probe image of clause 33 did not build" >>"$REPORT"; fail=1
fi

# Clause 34 (T-1725): an absolute link of the image resolves in the
# rootfs. The links are planted in the alpine rootfs of the store.
AR34=$("$PB" inspect --format '{{.RootfsPath}}' "$ALPINE" 2>/dev/null)
if [ -n "$AR34" ] && [ -d "$AR34" ]; then
	mkdir -p "$AR34/opt/l734" && echo in-rootfs >"$AR34/opt/l734/f"
	ln -sfn /opt/l734/f "$AR34/home/l734"; ln -sfn /opt/l734 "$AR34/home/d734"
	ln -sfn /etc/hostname-t1725-host "$AR34/home/h734"; echo host-only >/etc/hostname-t1725-host
	step fx-abslink 0 $FX "$PB" run --rm "$ALPINE" sh -c 'cat /home/l734 /home/d734/f; ls -l /home/l734'
	[ "$(grep -c '^in-rootfs$' "$WORK/out-fx-abslink.txt")" -eq 2 ] || { echo "an absolute link did not resolve in the rootfs" >>"$REPORT"; fail=1; }
	grep -q "l734 -> /opt/l734/f" "$WORK/out-fx-abslink.txt" || { echo "ls -l did not show the link" >>"$REPORT"; fail=1; }
	step fx-abslink-host 1 $FX "$PB" run --rm "$ALPINE" cat /home/h734
	grep -q "host-only" "$WORK/out-fx-abslink-host.txt" && { echo "a link of the rootfs read a file of the host" >>"$REPORT"; fail=1; }
	rm -f /etc/hostname-t1725-host "$AR34/home/l734" "$AR34/home/d734" "$AR34/home/h734"; rm -rf "$AR34/opt/l734"
else
	echo "the alpine rootfs of clause 34 is missing" >>"$REPORT"; fail=1
fi

# Clause 35 (T-1726): exec into a container with a :ro volume.
V35="$WORK/vol735"; rm -rf "$V35"; mkdir -p "$V35"
$FX "$PB" run -d --name ro735 -v "$V35:/e:ro" "$ALPINE" sleep 60 >/dev/null 2>&1
step fx-exec-ro 1 $FX "$PB" exec ro735 sh -c 'echo x > /e/w.txt'
[ -e "$V35/w.txt" ] && { echo "exec wrote under a :ro volume" >>"$REPORT"; fail=1; }
$FX "$PB" rm -f ro735 >/dev/null 2>&1

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           NO-CHROOT FAMILY RUNS"; else echo "verdict           NO-CHROOT FAMILY OPEN"; fi
} >>"$REPORT"

cp "$REPORT" "$REPO/experiments/results/no-chroot-rung.txt"
if [ -d /out ]; then cp "$REPORT" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
