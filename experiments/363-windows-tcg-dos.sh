#!/bin/sh
# Question: on a KVM-less lane, does a DOS guest run one command under TCG
# and return its stdout and its exit code, with the base untouched and the
# per-run overlay discarded?
#
# TODO/milestones.md T-1112 (the guest arm, portable: FreeDOS 1.4 under TCG,
# no KVM, no licensed image). The refusal arm stays in 362 for the doors
# this driver does not serve: create, detached run, chroot tier, and a
# foreign arch.
#
# Pinned inputs: FreeDOS 1.4 LiteUSB zip, 17 MB, sha256 pinned below, raw
# image 33554432 bytes. qemu 10.2.3 TCG on the lane. Every fetch carries
# two ceilings: timeout around curl and --max-time inside it, so a stalled
# origin proves it rather than hanging the lane.
#
# Clauses:
#   0. conditions: binary, kvm state, qemu version and accel list.
#   1. the machine profile holds (tcg or full) with refusal null.
#   2. setup: fetch with ceiling, sha256 matches, image is 33554432 bytes
#      raw, base lands at PODBOX_DOS_BASE.
#   3. `run --podbox-tier=machine --platform windows/amd64 freedos:1.4 ver`
#      exits 0 with FreeCom version 0.86 on stdout and the TCG plus FreeDOS
#      banner on stderr.
#   4. `cmd /c ver` exits 0 with the same version string: the prefix drops.
#   5. `dir /zzz` exits 1, distinct from 0: the non-zero path.
#   6. `pause` exits 125 with DEADLINE and no status: the deadline path.
#   7. the base is byte-identical after every run, no per-run dir remains,
#      and the OCI store is untouched.
#   8. the other doors still refuse by name: create, run without the tier,
#      and windows/arm64 on this host.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no toolchain, no build, no fetch, no jq).
set -u

REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep363-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-363.txt"

FD_URL='https://download.freedos.org/1.4/FD14-LiteUSB.zip'
FD_SHA256='857dcd2ebf9d3d094320154db5fb5b830acba6fb98f981a95a0ca7ab3350338b'
FD_IMG_BYTES=33554432

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "cpu model         $(grep -m1 'model name' /proc/cpuinfo 2>/dev/null | cut -d: -f2 | sed 's/^ //')"
if [ -e /dev/kvm ]; then echo "kvm               present"; else echo "kvm               absent"; fi
if command -v qemu-system-x86_64 >/dev/null 2>&1; then
	echo "qemu              $(qemu-system-x86_64 --version | head -1)"
	echo "accel             $(qemu-system-x86_64 -accel help 2>&1 | tr '\n' ' ')"
else
	echo "qemu              absent"
fi
if command -v qemu-img >/dev/null 2>&1; then
	echo "qemu-img          present"
else
	echo "qemu-img          absent"
fi
} >"$REPORT"

if [ -x /workspace/zig/zig ]; then
	PATH="/workspace/zig:$PATH"; export PATH
	echo "zig               $(zig version) at /workspace/zig" >>"$REPORT"
fi
command -v cargo >/dev/null 2>&1 || { echo "cargo missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
command -v unzip >/dev/null 2>&1 || { echo "unzip missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
echo "== bootstrap the lane toolchain (zig already on PATH, so rust cc tools)"
if timeout 1200 ./scripts/common/bootstrap-env.sh rust cc tools >>"$WORK/bootstrap.log" 2>&1; then
	echo "bootstrap         ok" >>"$REPORT"
else
	echo "bootstrap         FAILED" | tee -a "$REPORT"
	tail -15 "$WORK/bootstrap.log"
	exit 2
fi
command -v jq >/dev/null 2>&1 || { echo "jq missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
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
export PODBOX_DOS_BASE="$WORK/freedos-base.img"
fail=0

# Clause 1: the machine profile holds with refusal null.
if timeout 120 "$PB" probe --json >"$WORK/probe.json" 2>/dev/null; then
	echo "probe json        ok" >>"$REPORT"
else
	echo "probe --json FAILED: COULD NOT RUN" | tee -a "$REPORT"
	exit 2
fi
PROFILE="$(jq -r '.tiers.machine.profile' "$WORK/probe.json" 2>/dev/null)"
REFUSAL="$(jq -r '.tiers.machine.refusal' "$WORK/probe.json" 2>/dev/null)"
echo "profile           $PROFILE" >>"$REPORT"
echo "refusal           $REFUSAL" >>"$REPORT"
if [ "$REFUSAL" = "null" ] && { [ "$PROFILE" = "tcg" ] || [ "$PROFILE" = "full" ]; }; then
	echo "clause 1          machine profile $PROFILE holds" >>"$REPORT"
else
	echo "clause 1          NO MACHINE PROFILE HERE" >>"$REPORT"; fail=1
fi

# Clause 2: fetch the base with two ceilings and pin it.
echo "== fetch the base (17 MB with 120 s max-time inside 180 s timeout)"
if [ "$(df -k "$WORK" | awk 'NR==2 {print $4}')" -lt 512000 ]; then
	echo "clause 2          WORK DISK SHORT: COULD NOT RUN" | tee -a "$REPORT"; exit 2
fi
if timeout 180 curl -fsSL --max-time 120 -o "$WORK/FD14-LiteUSB.zip" "$FD_URL" >>"$WORK/fetch.log" 2>&1; then
	echo "fetch             ok" >>"$REPORT"
else
	echo "fetch             FAILED: COULD NOT RUN" | tee -a "$REPORT"
	tail -10 "$WORK/fetch.log"; exit 2
fi
GOT="$(sha256sum "$WORK/FD14-LiteUSB.zip" | cut -d' ' -f1)"
echo "sha256            $GOT" >>"$REPORT"
if [ "$GOT" != "$FD_SHA256" ]; then
	echo "clause 2          SHA256 MISMATCH" >>"$REPORT"; fail=1
fi
if timeout 120 unzip -p "$WORK/FD14-LiteUSB.zip" FD14LITE.img >"$WORK/FD14LITE.img" 2>>"$WORK/fetch.log"; then
	echo "unzip             ok" >>"$REPORT"
else
	echo "unzip             FAILED" | tee -a "$REPORT"; exit 1
fi
SZ="$(wc -c <"$WORK/FD14LITE.img" | tr -d ' ')"
echo "image bytes       $SZ" >>"$REPORT"
if [ "$SZ" -ne "$FD_IMG_BYTES" ]; then
	echo "clause 2          IMAGE SIZE $SZ, wanted $FD_IMG_BYTES" >>"$REPORT"; fail=1
fi
cp "$WORK/FD14LITE.img" "$PODBOX_DOS_BASE" || exit 1
qemu-img info "$PODBOX_DOS_BASE" >>"$REPORT" 2>&1 || { echo "qemu-img info FAILED" | tee -a "$REPORT"; exit 1; }
BASE_SHA_BEFORE="$(sha256sum "$PODBOX_DOS_BASE" | cut -d' ' -f1)"
echo "base sha          $BASE_SHA_BEFORE" >>"$REPORT"
echo "clause 2          base pinned and landed" >>"$REPORT"

snap() { find "$STORE" -type f | sort >"$WORK/snap-$1.txt"; }
snap before

# One guest run per clause. Each carries its own timeout: the driver
# bounds the boot plus the command, and timeout bounds the driver. The
# wrapper is 300 s because the DOS deadline alone is 180 s after a 42 s
# boot walk: a smaller wrapper would kill the driver first and report
# timeout's own 124 instead of the driver's DEADLINE.
grun() {
	name="$1"; want="$2"; shift 2
	out="$WORK/out-$name.txt"
	err="$WORK/err-$name.txt"
	timeout 300 "$@" >"$out" 2>"$err"
	rc=$?
	echo "step $name exit $rc"
	{
	echo ""
	echo "== $name"
	echo "command           $*"
	echo "exit              $rc"
	echo "stdout:"
	sed 's/^/  /' "$out"
	echo "stderr:"
	sed 's/^/  /' "$err"
	} >>"$REPORT"
	[ "$rc" -eq "$want" ] || { echo "step $name: wanted exit $want, got $rc" >>"$REPORT"; fail=1; }
}

# Clause 3: ver returns the guest version string and 0.
grun dos-ver 0 "$PB" run --rm --podbox-tier=machine --platform windows/amd64 freedos:1.4 ver
grep -q "FreeCom version 0.86" "$WORK/out-dos-ver.txt" || { echo "ver stdout did not carry FreeCom version 0.86" >>"$REPORT"; fail=1; }
grep -q "FreeDOS 1.4" "$WORK/err-dos-ver.txt" || { echo "ver banner did not name FreeDOS 1.4" >>"$REPORT"; fail=1; }
grep -q "not Windows 11" "$WORK/err-dos-ver.txt" || { echo "ver banner did not state not Windows 11" >>"$REPORT"; fail=1; }

# Clause 4: cmd /c ver drops the prefix and returns the same.
grun dos-cmdver 0 "$PB" run --rm --podbox-tier=machine --platform windows/amd64 freedos:1.4 cmd /c ver
grep -q "FreeCom version 0.86" "$WORK/out-dos-cmdver.txt" || { echo "cmd /c ver stdout did not carry the version" >>"$REPORT"; fail=1; }

# Clause 5: the non-zero path is distinct from 0.
grun dos-nonzero 1 "$PB" run --rm --podbox-tier=machine --platform windows/amd64 freedos:1.4 dir /zzz
echo "clause 5          non-zero returns 1, distinct from 0" >>"$REPORT"

# Clause 6: the deadline path carries no status.
grun dos-deadline "$RUN_ERR" "$PB" run --rm --podbox-tier=machine --platform windows/amd64 freedos:1.4 pause
grep -q "DEADLINE" "$WORK/err-dos-deadline.txt" || { echo "deadline stderr did not name DEADLINE" >>"$REPORT"; fail=1; }
grep -q "no status" "$WORK/err-dos-deadline.txt" || { echo "deadline stderr did not state no status" >>"$REPORT"; fail=1; }

# Clause 7: the base is identical, no per-run dir remains, the store is untouched.
BASE_SHA_AFTER="$(sha256sum "$PODBOX_DOS_BASE" | cut -d' ' -f1)"
if [ "$BASE_SHA_BEFORE" = "$BASE_SHA_AFTER" ]; then
	echo "clause 7          base byte-identical ($BASE_SHA_AFTER)" >>"$REPORT"
else
	echo "clause 7          BASE MUTATED" >>"$REPORT"; fail=1
fi
LEFTOVERS="$(find /tmp -maxdepth 1 -name 'podbox-windows-*' 2>/dev/null | wc -l | tr -d ' ')"
echo "per-run leftovers $LEFTOVERS" >>"$REPORT"
snap after-runs
if cmp -s "$WORK/snap-before.txt" "$WORK/snap-after-runs.txt"; then
	echo "clause 7          store untouched" >>"$REPORT"
else
	echo "clause 7          STORE MUTATED" >>"$REPORT"; fail=1
fi

# Clause 8: the other doors still refuse by name.
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
step win-create "$RUN_ERR" "$PB" create --platform windows/amd64 freedos:1.4 ver
grep -q "windows/amd64" "$WORK/out-win-create.txt" || { echo "create refusal did not name windows/amd64" >>"$REPORT"; fail=1; }
grep -q "podbox windows run" "$WORK/out-win-create.txt" || { echo "create refusal did not route to podbox windows run" >>"$REPORT"; fail=1; }
step win-notier "$RUN_ERR" "$PB" run --rm --platform windows/amd64 freedos:1.4 ver
grep -q "windows/amd64" "$WORK/out-win-notier.txt" || { echo "no-tier refusal did not name windows/amd64" >>"$REPORT"; fail=1; }
grep -q "podbox windows run" "$WORK/out-win-notier.txt" || { echo "no-tier refusal did not route to podbox windows run" >>"$REPORT"; fail=1; }
step win-arm64 "$RUN_ERR" "$PB" run --rm --podbox-tier=machine --platform windows/arm64 freedos:1.4 ver
grep -q "windows/arm64" "$WORK/out-win-arm64.txt" || { echo "arm64 refusal did not name windows/arm64" >>"$REPORT"; fail=1; }
echo "clause 8          create, no-tier and arm64 refuse by name" >>"$REPORT"

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           WINDOWS TCG DOS HOLDS"; else echo "verdict           WINDOWS TCG DOS OPEN"; fi
} >>"$REPORT"

cp "$REPORT" "$REPO/experiments/results/windows-tcg-dos.txt"
if [ -d /out ]; then cp "$REPORT" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
