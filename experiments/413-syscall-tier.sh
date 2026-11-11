#!/bin/sh
# Question: does `run --podvm-tier=syscall` give a static payload the view
# of a syscall-interpreting tier (the image's files, a fixed kernel
# identity, a sized /tmp, a /workspace that writes through) with no
# ptrace, no qemu and no user namespace of podvm's own, and does it say
# that it is not a boundary?
#
# TODO/enter.md T-1657 (issue 169). Each run goes through the fixture of
# experiment 356: user and mount namespaces, /dev/pts hidden, chroot(2)
# denied. The payload is the static busybox of busybox:musl, in an image
# that `podvm import` builds with an /etc/os-release of its own, so a read
# of that file can only come from the image.
#
# Pinned input: busybox:musl by tag (the conditions print its digest).
# PODVM_413_MIRROR=1 takes it from mirror.gcr.io.
#
# Clauses:
#   1. `cat /etc/os-release` prints the image's file, not the host's.
#   2. no ptrace and no qemu: TracerPid is 0 in /proc/self/status, the
#      entered rung is `syscall`, and no qemu process runs.
#   3. a write to /workspace (`-v HOST:/workspace`) is on the host after
#      the run.
#   4. `uname -r` prints the release that names the tier.
#   5. the banner (`--verbose`) says that the layer is not a boundary.
#   6. `--tmpfs /tmp:size=1m`: a write of 2 MiB under /tmp stops at the
#      cap with ENOSPC, and /tmp is gone after the run.
#   7. a dynamic program refuses by name (exit 125), and so does `run -d`.
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep413-work"
rm -rf "$WORK"; mkdir -p "$WORK/ws" || exit 2
OUT="$REPO/experiments/results/syscall-tier.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
BUSYBOX='public.ecr.aws/docker/library/busybox:musl'
ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
if [ "${PODVM_413_MIRROR:-0}" = 1 ]; then
	BUSYBOX='mirror.gcr.io/library/busybox:musl'
	ALPINE='mirror.gcr.io/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
fi
IMG='podvm413:1'
export PODVM_STORE="$WORK/store"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }

[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" >&2; exit 2; }
unshare -Urm true >/dev/null 2>&1 || { echo "user+mount namespaces refused: COULD NOT RUN" >&2; exit 2; }
cc -O2 -Wall -Werror -static -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>/dev/null ||
	cc -O2 -Wall -Werror -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" ||
	{ echo "fixture COMPILE FAILED: COULD NOT RUN" >&2; exit 2; }
cat >"$WORK/wrap.sh" <<'EOF'
#!/bin/sh
mount -t tmpfs tmpfs /dev/pts || exit 2
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF
fx() { unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$PB" "$@"; }

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "fixture           unshare -Urm, /dev/pts hidden, chroot(2) denied"
timeout 600 "$PB" pull -q "$BUSYBOX" >"$WORK/pull.log" 2>&1 ||
	{ note "pull FAILED: COULD NOT RUN"; sed 's/^/  /' "$WORK/pull.log"; exit 2; }
note "busybox           $BUSYBOX $("$PB" inspect --format '{{.Id}}' "$BUSYBOX" 2>/dev/null)"
SRC=$("$PB" extract "$BUSYBOX" 2>/dev/null | tail -1)
[ -d "$SRC/bin" ] || { note "extract FAILED: COULD NOT RUN"; exit 2; }
mkdir -p "$WORK/img"
cp -a "$SRC/." "$WORK/img/"
mkdir -p "$WORK/img/etc"
printf 'ID=podvm413\nPRETTY_NAME="the image of experiment 413"\n' >"$WORK/img/etc/os-release"
tar -C "$WORK/img" -cf "$WORK/img.tar" . || exit 2
"$PB" import "$WORK/img.tar" "$IMG" >/dev/null 2>&1 || { note "import FAILED: COULD NOT RUN"; exit 2; }
note "image             $IMG (busybox:musl and an /etc/os-release of its own)"
note ""

note "== one run: clauses 1 to 5"
fx run --rm --verbose --podvm-tier=syscall -v "$WORK/ws:/workspace" "$IMG" sh -c '
cat /etc/os-release
grep TracerPid /proc/self/status
echo written-by-413 > /workspace/out.txt
uname -r
' >"$WORK/run.out" 2>"$WORK/run.err" &
RUN=$!
sleep 0.5
qemu=$(pgrep -c qemu 2>/dev/null); [ -n "$qemu" ] || qemu=0
wait "$RUN"
rc=$?
note "  exit $rc  (want 0)"
sed 's/^/    /' "$WORK/run.out" | tee -a "$OUT"
[ "$rc" -eq 0 ] || { bad "the run exited $rc"; tail -5 "$WORK/run.err" | sed 's/^/      /' | tee -a "$OUT"; }
note "== clause 1  /etc/os-release of the image"
grep -qx 'ID=podvm413' "$WORK/run.out" || bad "the payload did not read the image's /etc/os-release"
grep -qx 'ID=podvm413' /etc/os-release 2>/dev/null && bad "the host has the same file, so the clause proves nothing"
note "== clause 2  no ptrace, no qemu"
grep -q '^TracerPid:[[:space:]]*0$' "$WORK/run.out" || bad "TracerPid is not 0"
note "  qemu processes during the run: $qemu"
[ "$qemu" = 0 ] || bad "a qemu process ran"
note "  rung named by the banner: $(grep -o 'entering on the syscall tier' "$WORK/run.err" | head -1)"
grep -q 'entering on the syscall tier' "$WORK/run.err" || bad "the banner does not name the syscall tier"
note "== clause 3  /workspace writes through"
note "  host file: $(cat "$WORK/ws/out.txt" 2>/dev/null || echo absent)"
[ "$(cat "$WORK/ws/out.txt" 2>/dev/null)" = written-by-413 ] || bad "the write did not reach the host"
note "== clause 4  uname -r"
note "  release: $(tail -1 "$WORK/run.out")"
[ "$(tail -1 "$WORK/run.out")" = "6.1.0-podvm-syscall" ] || bad "uname -r does not name the tier"
note "== clause 5  the banner"
grep -q "it is a compatibility layer, not a boundary" "$WORK/run.err" || bad "the banner does not say that the layer is not a boundary"
note "  $(grep -o 'The handler shares[^(]*' "$WORK/run.err" | head -1)"
note ""

note "== clause 6  the cap of /tmp"
fx run --rm --podvm-tier=syscall --tmpfs /tmp:size=1m "$IMG" sh -c '
dd if=/dev/zero of=/tmp/big bs=4096 count=512 2>&1 | grep -i -m1 "space\|copied"
ls -l /tmp/big | awk "{print \$5}"
' >"$WORK/cap.out" 2>"$WORK/cap.err"
rc=$?
note "  exit $rc"
sed 's/^/    /' "$WORK/cap.out" | tee -a "$OUT"
grep -qi "no space left on device" "$WORK/cap.out" || bad "a write over the cap did not fail with ENOSPC"
[ "$(tail -1 "$WORK/cap.out")" = 1048576 ] || bad "the file under /tmp is not 1 MiB"
left=$(find "$PODVM_STORE/tmpfs" -mindepth 1 2>/dev/null | head -3)
note "  directories of /tmp left in the store: ${left:-none}"
[ -z "$left" ] || bad "the /tmp of the run stayed"
note ""

note "== clause 7  refusals by name"
timeout 600 "$PB" pull -q "$ALPINE" >/dev/null 2>&1
fx run --rm --podvm-tier=syscall "$ALPINE" true >"$WORK/dyn.out" 2>&1
rc=$?
note "  dynamic: exit $rc: $(grep -m1 'syscall' "$WORK/dyn.out")"
[ "$rc" -eq 125 ] || bad "a dynamic program exited $rc"
grep -q "statically linked" "$WORK/dyn.out" || bad "the refusal does not name the static program"
fx run -d --podvm-tier=syscall "$IMG" true >"$WORK/det.out" 2>&1
rc=$?
note "  run -d:  exit $rc: $(head -1 "$WORK/det.out")"
[ "$rc" -eq 125 ] || bad "run -d exited $rc"
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: THE SYSCALL TIER GIVES THE VIEW AND SAYS IT IS NOT A BOUNDARY"
else
	note "verdict: A CLAUSE DISAGREED"
fi
sed -i "s#$WORK#WORK#g" "$OUT"
rm -rf "$WORK"
exit "$fail"
