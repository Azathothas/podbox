#!/bin/sh
# Question: on a KVM-less lane, does a Windows guest refuse by name on
# every entry path before anything is fetched or mutated, and does the
# machine tier name the platform rather than its legs?
#
# TODO/milestones.md T-1112 (the refusal arm; the guest arm needs a KVM
# host with a licensed image, which no reachable machine is).
#
# The fixture is the lane itself beside T-1317's: /dev/kvm absent,
# qemu below the 11 the entry prerequisites, tun absent. The drive
# asserts the absence first (clause 1), so a future lane with KVM
# fails out loud rather than drifting.
#
# Clauses:
#   0. conditions: binary, /dev/kvm presence, qemu version.
#   1. the fixture: probe JSON reads open(/dev/kvm) denied ENOENT.
#   2. run --platform windows/amd64 exits 125 naming windows/amd64
#      and the missing support, with the store byte-identical after.
#   3. run --podbox-tier=machine --platform windows/amd64 exits 125
#      naming windows/amd64: the OS gate sits before the tier
#      dispatch, so a Windows request never reads as a Linux guest
#      the driver has not arrived for.
#   4. create --platform windows/amd64 exits 125 naming windows/amd64
#      with the store byte-identical after.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no toolchain, no build, no jq).
set -u

# The job travels as /work/.podbox-job.sh: take the checkout from the
# working directory, never from $0 (see 353 for the measurement).
REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep362-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-362.txt"

ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
if [ -e /dev/kvm ]; then echo "kvm               present"; else echo "kvm               absent"; fi
if command -v qemu-system-x86_64 >/dev/null 2>&1; then
	echo "qemu              $(qemu-system-x86_64 --version | head -1)"
else
	echo "qemu              absent"
fi
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

# Clause 1: the fixture reads KVM denied.
if timeout 120 "$PB" probe --json >"$WORK/probe.json" 2>/dev/null; then
	echo "probe json        ok" >>"$REPORT"
else
	echo "probe --json FAILED: COULD NOT RUN" | tee -a "$REPORT"
	exit 2
fi
if jq -e '.probes[] | select(.name == "open(/dev/kvm, O_RDWR)") | .verdict == "denied" and .errno_name == "ENOENT"' "$WORK/probe.json" >/dev/null; then
	echo "clause 1          kvm denied ENOENT on this lane" >>"$REPORT"
else
	echo "clause 1          KVM NOT DENIED HERE" >>"$REPORT"; fail=1
fi

# The store snapshot: every refusal below must leave it identical.
snap() { find "$STORE" -type f | sort >"$WORK/snap-$1.txt"; }
snap before

# Clause 2: run refuses the Windows guest by name before any fetch.
step win-run "$RUN_ERR" "$PB" run --rm --platform windows/amd64 "$ALPINE" cmd /c ver
grep -q "windows/amd64" "$WORK/out-win-run.txt" || { echo "run refusal did not name windows/amd64" >>"$REPORT"; fail=1; }
grep -q "No windows guest support exists" "$WORK/out-win-run.txt" || { echo "run refusal did not name the missing support" >>"$REPORT"; fail=1; }
snap after-run
if cmp -s "$WORK/snap-before.txt" "$WORK/snap-after-run.txt"; then
	echo "clause 2          store untouched" >>"$REPORT"
else
	echo "clause 2          STORE MUTATED" >>"$REPORT"; fail=1
fi

# Clause 3: the machine tier names the platform, not its legs.
step win-machine "$RUN_ERR" "$PB" run --rm --podbox-tier=machine --platform windows/amd64 "$ALPINE" cmd /c ver
grep -q "windows/amd64" "$WORK/out-win-machine.txt" || { echo "machine refusal did not name windows/amd64" >>"$REPORT"; fail=1; }
snap after-machine
if cmp -s "$WORK/snap-before.txt" "$WORK/snap-after-machine.txt"; then
	echo "clause 3          store untouched" >>"$REPORT"
else
	echo "clause 3          STORE MUTATED" >>"$REPORT"; fail=1
fi

# Clause 4: create refuses the same way.
step win-create "$RUN_ERR" "$PB" create --platform windows/amd64 "$ALPINE" cmd /c ver
grep -q "windows/amd64" "$WORK/out-win-create.txt" || { echo "create refusal did not name windows/amd64" >>"$REPORT"; fail=1; }
snap after-create
if cmp -s "$WORK/snap-before.txt" "$WORK/snap-after-create.txt"; then
	echo "clause 4          store untouched" >>"$REPORT"
else
	echo "clause 4          STORE MUTATED" >>"$REPORT"; fail=1
fi

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           WINDOWS REFUSAL HOLDS"; else echo "verdict           WINDOWS REFUSAL OPEN"; fi
} >>"$REPORT"

cp "$REPORT" "$REPO/experiments/results/windows-refusal.txt"
if [ -d /out ]; then cp "$REPORT" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
