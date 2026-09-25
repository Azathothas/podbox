#!/bin/sh
# Question: on a kvm-less host whose emulator lists tcg, does the machine
# tier establish the TCG profile (tiers.machine.refusal null) with the
# banner stating no hardware isolation, and does refusal stay where no
# accelerator at all holds?
#
# TODO/podvm.md T-1301 (TCG split).
#
# Clauses:
#   0. conditions: binary, qemu (installed here where missing), kernel.
#   1. probe document: tiers.machine.refusal null, profile tcg, the
#      accel leg listing tcg, kvm and tun denied. T-1301's Prove core.
#   2. evidence text: the `profile: tcg` line with the
#      no-hardware-isolation sentence and the tun shape.
#   3. entry arms: `run --podbox-tier=machine` and `podvm run` print the
#      TCG holds message at 125 (no guest driver yet); the mem ceiling
#      still refuses first.
#   4. refusal arm: with qemu hidden from PATH the tier refuses naming
#      the emulator at 125.
#   5. boot arm: 146 re-driven with this binary proves the initramfs
#      boots to VMR-GUEST-READY under TCG. 146 writes its own results
#      file; a backup rides in WORK and is restored where 146 fails, so
#      a failed boot never clobbers T-1303's evidence.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no toolchain, no build, no qemu, no jq, no boot).
set -u

# The job travels as /work/.podbox-job.sh: take the checkout from the
# working directory, never from $0 (see 353 for the measurement).
REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep357-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-357.txt"

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
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
# jq rides the bootstrap (its installed-tools line names it); the probe
# document clauses below need it, so its absence here is the lane, not
# the binary.
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
ln -sf "$PB" "$WORK/podvm" || exit 2
PVM="$WORK/podvm"

# shellcheck source=../scripts/common/exit-codes.sh
. "$REPO/scripts/common/exit-codes.sh"
podbox_exit_codes "$PB" || { echo "cannot read podbox's exit-code table: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
RUN_ERR=$PODBOX_EXIT_RUNTIME_ERROR

echo "== qemu"
if command -v qemu-system-x86_64 >/dev/null 2>&1; then
	echo "qemu              $(qemu-system-x86_64 --version | head -1) (present)" >>"$REPORT"
else
	echo "qemu              absent: installing qemu-system-x86" >>"$REPORT"
	if timeout 600 apt-get update >>"$WORK/apt.log" 2>&1 \
		&& timeout 1200 apt-get install -y qemu-system-x86 cpio >>"$WORK/apt.log" 2>&1; then
		echo "qemu              $(qemu-system-x86_64 --version | head -1) (installed)" >>"$REPORT"
	else
		echo "qemu              INSTALL FAILED: COULD NOT RUN" | tee -a "$REPORT"
		tail -15 "$WORK/apt.log"
		exit 2
	fi
fi
for t in cpio python3 curl; do
	command -v "$t" >/dev/null 2>&1 || { echo "$t missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
done
[ -c /dev/kvm ] && echo "kvm               present (full-tier lane)" >>"$REPORT" \
	|| echo "kvm               absent (TCG lane)" >>"$REPORT"

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

# Clause 1: the document carries the TCG profile with a null refusal.
"$PB" probe --json >"$WORK/probe.json" 2>"$WORK/probe-err.txt"
rc=$?
echo "probe exit        $rc" >>"$REPORT"
[ "$rc" -eq 0 ] || { echo "probe did not run: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
if jq -e '.tiers.machine.refusal == null' "$WORK/probe.json" >/dev/null; then
	echo "refusal           null" >>"$REPORT"
else
	echo "refusal NOT null: FAILED" >>"$REPORT"; fail=1
fi
if [ "$(jq -r '.tiers.machine.profile' "$WORK/probe.json")" = "tcg" ]; then
	echo "profile           tcg" >>"$REPORT"
else
	echo "profile NOT tcg: FAILED" >>"$REPORT"; fail=1
fi
if jq -e '.tiers.machine.legs[] | select(.name == "qemu-system-x86_64 -accel help") | .reason | contains("tcg")' "$WORK/probe.json" >/dev/null; then
	echo "accel lists       tcg" >>"$REPORT"
else
	echo "accel does NOT list tcg: FAILED" >>"$REPORT"; fail=1
fi
for leg in "open(/dev/kvm, O_RDWR)" "open(/dev/net/tun, O_RDWR)"; do
	if jq -e --arg n "$leg" '.tiers.machine.legs[] | select(.name == $n) | .verdict == "denied"' "$WORK/probe.json" >/dev/null; then
		echo "denied            $leg" >>"$REPORT"
	else
		echo "NOT denied: $leg: FAILED" >>"$REPORT"; fail=1
	fi
done

# Clause 2: the evidence names the profile with both honest halves.
step evidence 0 "$PB" probe
grep -q "profile: tcg" "$WORK/out-evidence.txt" || { echo "evidence missing the tcg profile" >>"$REPORT"; fail=1; }
grep -q "not hardware isolation" "$WORK/out-evidence.txt" || { echo "evidence missing the no-hardware-isolation sentence" >>"$REPORT"; fail=1; }
grep -q "user-mode networking" "$WORK/out-evidence.txt" || { echo "evidence missing the tun shape" >>"$REPORT"; fail=1; }

# Clause 3: the entry prints the TCG holds message at 125, under both names.
step tier-run "$RUN_ERR" "$PB" run --podbox-tier=machine never-pulled-357:tag /bin/true
grep -q "the machine tier runs TCG" "$WORK/out-tier-run.txt" || { echo "run missing the TCG message" >>"$REPORT"; fail=1; }
grep -q "not hardware" "$WORK/out-tier-run.txt" || { echo "run missing the boundary sentence" >>"$REPORT"; fail=1; }
step tier-pvm "$RUN_ERR" "$PVM" run never-pulled-357:tag /bin/true
grep -q "the machine tier runs TCG" "$WORK/out-tier-pvm.txt" || { echo "podvm missing the TCG message" >>"$REPORT"; fail=1; }
grep -q "invoked as \`podvm\`" "$WORK/out-tier-pvm.txt" || { echo "podvm run missing its name note" >>"$REPORT"; fail=1; }

# Clause 4: with qemu hidden the tier refuses naming the emulator. A
# fresh store rides along: the probe caches its findings per store
# (T-0111), so the same store would answer from the earlier run with
# qemu present rather than re-probing without it. The hiding is an
# empty PATH directory, not surgery on the real one: qemu may exist
# under several directories, and removing one line still resolves it.
mkdir -p "$WORK/store-noemu" "$WORK/empty-path" || exit 2
step tier-noemu "$RUN_ERR" env PATH="$WORK/empty-path" PODBOX_STORE="$WORK/store-noemu" "$PB" run --podbox-tier=machine never-pulled-357:tag /bin/true
grep -q "machine tier refused" "$WORK/out-tier-noemu.txt" || { echo "hidden-qemu run did not refuse the tier" >>"$REPORT"; fail=1; }
grep -q "no qemu-system-x86_64 on PATH" "$WORK/out-tier-noemu.txt" || { echo "hidden-qemu refusal did not name the emulator" >>"$REPORT"; fail=1; }

# Clause 5: 146 re-driven with this binary boots to the ready marker.
BAK="$WORK/podvm-initramfs.bak"
if [ -f "$REPO/experiments/results/podvm-initramfs.txt" ]; then
	cp "$REPO/experiments/results/podvm-initramfs.txt" "$BAK" || exit 2
fi
echo "== boot arm (146 with this binary)"
if PODBOX_BIN="$PB" timeout 1800 sh "$REPO/experiments/146-podvm-initramfs.sh" >>"$REPORT" 2>&1; then
	echo "boot              VMR-GUEST-READY through TCG" >>"$REPORT"
else
	echo "boot              FAILED" >>"$REPORT"; fail=1
	if [ -f "$BAK" ]; then
		cp "$BAK" "$REPO/experiments/results/podvm-initramfs.txt"
		echo "146 evidence      restored from backup" >>"$REPORT"
	fi
fi

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           TCG PROFILE RUNS"; else echo "verdict           TCG PROFILE OPEN"; fi
} >>"$REPORT"

cp "$REPORT" "$REPO/experiments/results/tcg-profile.txt"
if [ -d /out ]; then cp "$REPORT" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
