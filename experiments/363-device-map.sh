#!/bin/sh
# Question: does `--device HOST[:GUEST[:PERMS]]` map a host path into the
# payload, and do the refusal arms name their reason?
#
# TODO/enter.md T-0501. `mknod` is denied and `mount` is denied, so the
# entry opens the host path before the chroot and the interposer serves
# the exact guest spelling as a duplicate of that descriptor. The binary
# is `$PODBOX_BIN`, lane-built with the interposer objects embedded (the
# serve holds only where the preload holds, which clause 6 shows).
#
# Clauses:
#   0. conditions, pulls and the pinned inputs; the opener (dynamic and
#      static) built on the lane and staged into the debian rootfs;
#   1. a host file maps and reads back byte-identical, the banner naming
#      the mapping on stderr while the payload owns stdout;
#   2. `/dev/zero` maps and reads back zeros;
#   3. the opener matrix: exact numeric flags against r, w and rw rows,
#      EACCES where the row does not grant, a descriptor where it does;
#   3b. creation where the image cannot satisfy it (a missing parent):
#      the serve answers with the node's own semantics, creation as a
#      no-op on a device and EEXIST for exclusive creation;
#   3c. the boundary, pinned: a creating open the image CAN satisfy
#      lands in the image (proven by reading the bytes back), because
#      the serve only ever answers failures;
#   3d. `stat` names the image (unhooked, honest ENOENT);
#   4. a missing host path refuses at 125 naming it;
#   5. the machine tier refuses `--device` at 125 naming the chroot tier;
#   6. the musl payload is served (both objects, both libcs) while the
#      lane-built static opener fails honestly with ENOENT, declined
#      where no loader reads LD_PRELOAD;
#   7. `create` then `start` serves the same mapping through the launcher,
#      which re-opens from the record rather than inheriting;
#   8. `exec` re-serves the container's mapping with no flag of its own.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no binary, no pull).
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/device-map.txt"
WORK="$REPO/experiments/.sweep363-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2

DEBIAN='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'

fail=0
pass() { echo "  ok: $1" >>"$WORK/report"; }
miss() { echo "  FAIL: $1" >>"$WORK/report"; fail=1; }

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "podbox            $($BIN version 2>/dev/null || echo MISSING)"
echo "debian            $DEBIAN"
echo "alpine            $ALPINE"
} >"$WORK/report"

[ -x "$BIN" ] || { echo "binary $BIN is not executable: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
# shellcheck source=../scripts/common/exit-codes.sh
. "$REPO/scripts/common/exit-codes.sh"
podbox_exit_codes "$BIN" || { echo "cannot read podbox's exit-code table: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
FLAG=$PODBOX_EXIT_FLAG_ERROR
RUN_ERR=$PODBOX_EXIT_RUNTIME_ERROR

STORE="$WORK/store"
mkdir -p "$STORE" || exit 2
export PODBOX_STORE="$STORE"

echo "" >>"$WORK/report"
echo "== pulls" >>"$WORK/report"
timeout 600 "$BIN" pull "$DEBIAN" >>"$WORK/report" 2>&1 \
	|| { echo "debian pull FAILED: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
timeout 600 "$BIN" pull "$ALPINE" >>"$WORK/report" 2>&1 \
	|| { echo "alpine pull FAILED: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }

printf 'device-serve-ok\n' >"$WORK/seed"
SEED_SHA="$(sha256sum <"$WORK/seed" | cut -d' ' -f1)"
echo "seed sha256       $SEED_SHA" >>"$WORK/report"

echo "" >>"$WORK/report"
echo "== opener build and stage" >>"$WORK/report"
command -v gcc >/dev/null 2>&1 || { echo "gcc missing: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
gcc -O2 -o "$WORK/opener" "$HERE/363-opener.c" || { echo "opener did not build: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
gcc -O2 -static -o "$WORK/opener-static" "$HERE/363-opener.c" || { echo "static opener did not build: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
file "$WORK/opener-static" | grep -q "statically linked" || { echo "static opener is not static: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
ROOTFS="$("$BIN" extract "$DEBIAN" 2>"$WORK/extract.log")" || { echo "extract FAILED: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
[ -n "$ROOTFS" ] && [ -d "$ROOTFS" ] || { echo "no rootfs extracted: COULD NOT RUN" >>"$WORK/report"; cp "$WORK/report" "$OUT"; exit 2; }
mkdir -p "$ROOTFS/devtest" || exit 2
cp "$WORK/opener" "$WORK/opener-static" "$ROOTFS/devtest/" || exit 2
echo "opener staged     $ROOTFS/devtest (dynamic and static)" >>"$WORK/report"

echo "" >>"$WORK/report"
echo "== 1. a host file maps and reads back" >>"$WORK/report"
timeout 120 "$BIN" run --device "$WORK/seed:/dev/hostseed:r" \
	"$DEBIAN" /bin/cat /dev/hostseed >"$WORK/seed.out" 2>"$WORK/seed.err"
rc=$?
if [ "$rc" -eq 0 ] && [ "$(sha256sum <"$WORK/seed.out" | cut -d' ' -f1)" = "$SEED_SHA" ]; then
	pass "seed file read back byte-identical through /dev/hostseed"
else
	miss "seed file read back rc=$rc"
fi
if grep -q "is reachable inside as /dev/hostseed (r)" "$WORK/seed.err"; then
	pass "the banner names the mapping on stderr"
else
	miss "the banner does not name the mapping"
fi

echo "" >>"$WORK/report"
echo "== 2. /dev/zero maps and reads back zeros" >>"$WORK/report"
timeout 120 "$BIN" run --device /dev/zero:/dev/hostzero:r \
	"$DEBIAN" /bin/sh -c 'head -c 64 /dev/hostzero | od -An -tx1' >"$WORK/zero.out" 2>"$WORK/zero.err"
rc=$?
if [ "$rc" -eq 0 ] && grep -q "00" "$WORK/zero.out" \
	&& ! grep -Eq "[1-9a-f]" "$WORK/zero.out"; then
	pass "64 zero bytes through /dev/hostzero"
else
	miss "zero read rc=$rc"
fi

echo "" >>"$WORK/report"
echo "== 3. the opener matrix: exact flags, exact answers" >>"$WORK/report"
# The staged opener passes numeric flags, so each arm names the shape
# under test: reads and writes against r, w and rw rows. Octal:
# O_RDONLY 0, O_WRONLY 1, O_RDWR 2, O_CREAT 0100, O_EXCL 0200.
open_case() {
	desc="$1"; want_rc="$2"; want_text="$3"; spec="$4"; flags="$5"
	out="$WORK/open-$6.out"
	timeout 120 "$BIN" run --device "$spec" \
		"$DEBIAN" /devtest/opener "$7" "$flags" >"$out" 2>"$out.err"
	rc=$?
	if [ "$rc" -eq "$want_rc" ] && grep -q "$want_text" "$out"; then
		pass "$desc"
	else
		miss "$desc (rc=$rc)"
		cat "$out" "$out.err" >>"$WORK/report"
	fi
}
open_case "read on an r row opens" 0 "rc=" "/dev/zero:/dev/hostzero:r" 0 m1 /dev/hostzero
open_case "write on an r row is EACCES" 1 "errno=13" "/dev/zero:/dev/hostzero:r" 1 m2 /dev/hostzero
open_case "read-write on an r row is EACCES" 1 "errno=13" "/dev/zero:/dev/hostzero:r" 2 m3 /dev/hostzero
open_case "read on a w row is EACCES" 1 "errno=13" "$WORK/seed:/dev/hostw:w" 0 m4 /dev/hostw
open_case "write on a w row opens" 0 "rc=" "$WORK/seed:/dev/hostw:w" 1 m5 /dev/hostw
open_case "read-write on an rw row opens" 0 "rc=" "/dev/zero:/dev/hostzero:rw" 2 m6 /dev/hostzero

echo "" >>"$WORK/report"
echo "== 3b. creation where the image cannot satisfy it" >>"$WORK/report"
# A creating open never reaches the serve where the image satisfies it
# (the image is writable, so O_CREAT makes the file there first). Where
# the parent is missing the real call fails ENOENT and the serve answers
# with the node's own semantics: creation is a no-op on a device,
# exclusive creation fails EEXIST.
open_case "creating open over a missing parent writes the device" 0 "rc=" \
	"/dev/zero:/dev/nodir/node:rw" 0101 m7 /dev/nodir/node
open_case "exclusive creation over a mapping is EEXIST" 1 "errno=17" \
	"/dev/zero:/dev/nodir/node:rw" 0301 m8 /dev/nodir/node

echo "" >>"$WORK/report"
echo "== 3c. creating opens otherwise land in the image" >>"$WORK/report"
# The boundary of the serve, pinned rather than hidden: `>` (O_CREAT)
# on an existing parent is satisfied by the image, so the bytes land
# there. Proven by reading them back: the device would discard them.
timeout 120 "$BIN" run --device "$WORK/seed:/dev/hostshadow:rw" \
	"$DEBIAN" /bin/bash -c 'echo shadow-bytes > /dev/hostshadow; cat /dev/hostshadow' \
	>"$WORK/shadow.out" 2>"$WORK/shadow.err"
rc=$?
if [ "$rc" -eq 0 ] && grep -q "shadow-bytes" "$WORK/shadow.out"; then
	pass "creating open lands in the image and reads back (rc=0)"
else
	miss "shadow write rc=$rc"
	cat "$WORK/shadow.out" "$WORK/shadow.err" >>"$WORK/report"
fi

echo "" >>"$WORK/report"
echo "== 3d. stat names the image, reads name the row" >>"$WORK/report"
# `stat` is not hooked: the image holds no node, so it honestly says
# so. A read on a write-only row is the row's own EACCES.
timeout 120 "$BIN" run --device /dev/zero:/dev/hostzero:r \
	"$DEBIAN" /bin/ls -la /dev/hostzero >"$WORK/stat.out" 2>"$WORK/stat.err"
rc=$?
if [ "$rc" -ne 0 ] && grep -qi "no such file" "$WORK/stat.err"; then
	pass "stat of a served-only path honestly ENOENTs (rc=$rc)"
else
	miss "stat of a served-only path rc=$rc"
	cat "$WORK/stat.out" "$WORK/stat.err" >>"$WORK/report"
fi
timeout 120 "$BIN" run --device "$WORK/seed:/dev/hostseed:w" \
	"$DEBIAN" /bin/cat /dev/hostseed >"$WORK/wronly.out" 2>"$WORK/wronly.err"
rc=$?
if [ "$rc" -ne 0 ] && grep -qi "permission denied" "$WORK/wronly.out" "$WORK/wronly.err" 2>/dev/null; then
	pass "read on a write-only row refused (rc=$rc, Permission denied)"
else
	miss "read on a write-only row rc=$rc"
	cat "$WORK/wronly.out" "$WORK/wronly.err" >>"$WORK/report" 2>/dev/null
fi

echo "" >>"$WORK/report"
echo "== 4. a missing host path refuses at 125" >>"$WORK/report"
timeout 120 "$BIN" run --device "$WORK/absent:/dev/hostseed:r" \
	"$DEBIAN" /bin/cat /dev/hostseed >"$WORK/miss.out" 2>"$WORK/miss.err"
rc=$?
if [ "$rc" -eq "$RUN_ERR" ] && grep -q "absent" "$WORK/miss.err"; then
	pass "missing host path refused at $rc naming it"
else
	miss "missing host path rc=$rc (want $RUN_ERR)"
fi

echo "" >>"$WORK/report"
echo "== 5. the machine tier refuses --device at 125" >>"$WORK/report"
timeout 120 "$BIN" run --podbox-tier=machine --device /dev/zero:/dev/hostzero:r \
	"$DEBIAN" /bin/cat /dev/hostzero >"$WORK/tier.out" 2>"$WORK/tier.err"
rc=$?
if [ "$rc" -eq "$FLAG" ] && grep -q "needs the chroot tier" "$WORK/tier.err"; then
	pass "machine tier refused --device at $rc naming the chroot tier"
else
	miss "machine tier --device rc=$rc (want $FLAG)"
fi

echo "" >>"$WORK/report"
echo "== 6. the musl payload is served, the static one fails honestly" >>"$WORK/report"
# Alpine's busybox is dynamically linked (the banner below preloads the
# musl object for it), so the same mapping serves there too: both
# objects, both libcs. The lane-built static opener is what no
# interposer reaches, and its open fails with the kernel's own ENOENT.
timeout 120 "$BIN" run --device "$WORK/seed:/dev/hostseed:r" \
	"$ALPINE" cat /dev/hostseed >"$WORK/musl.out" 2>"$WORK/musl.err"
rc=$?
if [ "$rc" -eq 0 ] && [ "$(sha256sum <"$WORK/musl.out" | cut -d' ' -f1)" = "$SEED_SHA" ]; then
	pass "musl payload served byte-identical through /dev/hostseed"
else
	miss "musl payload rc=$rc"
fi
grep -i "musl object.*preloaded" "$WORK/musl.err" >>"$WORK/report" \
	|| echo "(no musl preload line)" >>"$WORK/report"
timeout 120 "$BIN" run --device "$WORK/seed:/dev/hostseed:r" \
	"$DEBIAN" /devtest/opener-static /dev/hostseed 0 >"$WORK/static.out" 2>"$WORK/static.err"
rc=$?
if [ "$rc" -ne 0 ] && grep -q "errno=2" "$WORK/static.out"; then
	pass "static payload fails honestly with ENOENT (rc=$rc, no interposer to serve)"
else
	miss "static payload rc=$rc"
	cat "$WORK/static.out" "$WORK/static.err" >>"$WORK/report" 2>/dev/null
fi
grep -i "statically linked" "$WORK/static.err" >>"$WORK/report" \
	|| echo "(no static-decline line)" >>"$WORK/report"

echo "" >>"$WORK/report"
echo "== 7. create then start serves through the launcher" >>"$WORK/report"
timeout 120 "$BIN" create --name d363 --device "$WORK/seed:/dev/hostseed:r" \
	"$DEBIAN" /bin/cat /dev/hostseed >"$WORK/create.out" 2>"$WORK/create.err"
rc=$?
if [ "$rc" -eq 0 ]; then
	timeout 120 "$BIN" start d363 >>"$WORK/report" 2>&1
	timeout 120 "$BIN" wait d363 >>"$WORK/report" 2>&1
	timeout 120 "$BIN" logs d363 >"$WORK/logs.out" 2>"$WORK/logs.err"
	if [ "$(sha256sum <"$WORK/logs.out" | cut -d' ' -f1)" = "$SEED_SHA" ]; then
		pass "launcher served the mapping: logs carry the seed bytes"
	else
		miss "launcher logs do not carry the seed bytes"
	fi
	timeout 120 "$BIN" rm d363 >>"$WORK/report" 2>&1 || miss "rm d363 failed"
else
	miss "create with --device rc=$rc"
fi

echo "" >>"$WORK/report"
echo "== 8. exec re-serves the container's mapping" >>"$WORK/report"
# `exec` takes no `--device` flag (docker parity: it re-enters), but the
# record carries the spec and the re-entry opens it again.
timeout 120 "$BIN" create --name d363e --device "$WORK/seed:/dev/hostseed:r" \
	"$DEBIAN" /bin/sleep 60 >"$WORK/create-e.out" 2>"$WORK/create-e.err"
rc=$?
if [ "$rc" -eq 0 ]; then
	timeout 120 "$BIN" start d363e >>"$WORK/report" 2>&1
	timeout 120 "$BIN" exec d363e /devtest/opener /dev/hostseed 0 >"$WORK/exec.out" 2>"$WORK/exec.err"
	rc=$?
	if [ "$rc" -eq 0 ] && grep -q "^rc=" "$WORK/exec.out"; then
		pass "exec re-served the mapping (opener rc=0)"
	else
		miss "exec opener rc=$rc"
		cat "$WORK/exec.out" "$WORK/exec.err" >>"$WORK/report"
	fi
	timeout 120 "$BIN" stop d363e >>"$WORK/report" 2>&1 || miss "stop d363e failed"
	timeout 120 "$BIN" rm d363e >>"$WORK/report" 2>&1 || miss "rm d363e failed"
else
	miss "create for exec with --device rc=$rc"
fi

echo "" >>"$WORK/report"
if [ "$fail" -eq 0 ]; then echo "verdict           DEVICE MAP SERVED" >>"$WORK/report"; else echo "verdict           DEVICE MAP OPEN" >>"$WORK/report"; fi
echo "== counts: 11 clauses, fail=$fail" >>"$WORK/report"
cp "$WORK/report" "$OUT"
[ "$fail" -eq 0 ]
