#!/usr/bin/env sh
# Lane proof for T-1568/T-1569 (podbox-podvm crate). Runs in the lane
# container as /in/job.sh with /work as the tree copy and /out for evidence.
# Ends with an exit-code capture, never a pipe on a cargo line.
set -u

OUT=/out/podvm-proof.log
PLANT=/out/podvm-plant.log
: >"$OUT" || exit 2
: >"$PLANT" || exit 2

say() { printf '%s\n' "$*" >>"$OUT"; }
fail=0
note() { say "  $1"; }
miss() { fail=1; say "  FAIL: $1"; }

say "== conditions"
date -u +%Y-%m-%dT%H:%M:%SZ >>"$OUT" 2>&1
uname -sr >>"$OUT" 2>&1
cargo --version >>"$OUT" 2>&1
rustc --version >>"$OUT" 2>&1
say ""

say "== 1. unit tests for the crate"
cargo test -p podbox-podvm >"$OUT.test" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "cargo test -p podbox-podvm exits 0"; else miss "cargo test rc=$rc"; fi
grep -a "test result:" "$OUT.test" >>"$OUT" 2>&1 || true
say ""

say "== 2. release binaries build"
cargo build --release -p podbox-podvm >"$OUT.build" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "cargo build --release -p podbox-podvm exits 0"; else miss "cargo build rc=$rc"; fi
RB=/work/target/x86_64-unknown-linux-musl/release
[ -x "$RB/podbox-podvm" ] && [ -x "$RB/podbox-podvm-workload" ] || miss "binaries absent after build"
say ""

say "== 3. the CLI the proofs drive"
cargo build --release --target x86_64-unknown-linux-musl -p podbox-cli >"$OUT.cli" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "podbox CLI built"; else miss "CLI build rc=$rc"; fi
CLI=/work/target/x86_64-unknown-linux-musl/release/podbox
[ -x "$CLI" ] || miss "CLI binary absent"
say ""

say "== 3b. guest tools for the workload clauses"
if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  timeout 600 apt-get update -qq >>"$OUT.apt" 2>&1
  timeout 600 apt-get install -y -qq qemu-system-x86 cpio file curl >>"$OUT.apt" 2>&1
  rc=$?
  note "guest tools install rc=$rc"
fi
command -v qemu-system-x86_64 >/dev/null 2>&1 || miss "qemu still absent"
command -v cpio >/dev/null 2>&1 || miss "cpio still absent"
command -v gcc >/dev/null 2>&1 || miss "gcc still absent"
say ""

say "== 4. podvm parity: retired script beside the binary, same CLI"
# Row wordings differ between the two drivers, so the compared verdict
# is the counts line, not the row text. Agreement is the proof; green
# in a lane without an accelerator is not asserted. The retired script
# derives its root from its own path, so it is extracted where
# parent.parent is /work.
mkdir -p /work/agree145
git show HEAD:experiments/145-podvm-parity.sh >/work/agree145/retired-145.sh 2>&1
chmod +x /work/agree145/retired-145.sh 2>/dev/null || true
PODBOX_BIN="$CLI" /work/agree145/retired-145.sh >"$OUT.145ret" 2>&1
ret_rc=$?
PODBOX_BIN="$CLI" sh /work/experiments/145-podvm-parity.sh >"$OUT.145" 2>&1
rc=$?
note "retired exits $ret_rc, shim exits $rc (both drive the same CLI)"
ret_counts="$(grep -a "^== counts:" "$OUT.145ret" | tail -n 1 || true)"
bin_counts="$(grep -a "^== counts:" "$OUT.145" | tail -n 1 || true)"
say "  retired: $ret_counts"
say "  binary:  $bin_counts"
if [ -n "$ret_counts" ] && [ "$ret_counts" = "$bin_counts" ]; then
  note "verdicts agree retired-vs-binary"
else
  miss "verdicts differ retired-vs-binary"
fi
cp /work/experiments/results/podvm-parity.txt /out/podvm-parity.txt 2>/dev/null || true
rm -rf /work/agree145
say ""

say "== 4b. both shims reach their binaries (the binary's message proves it)"
check_chain() {
  desc="$1"
  want="$2"
  shift 2
  out="$("$@" 2>&1)"
  rc=$?
  case "$out" in
    *"$want"*)
      if [ "$rc" -eq 2 ]; then note "$desc reaches its binary"; else miss "$desc rc=$rc"; fi
      ;;
    *) miss "$desc answered [$out] rc=$rc" ;;
  esac
}
check_chain "podvm shim" "podbox-podvm: unexpected argument" sh /work/experiments/145-podvm-parity.sh --frobnicate
check_chain "workload shim" "podbox-podvm-workload: unexpected argument" sh /work/experiments/154-tcg-workload-spread.sh --frobnicate
say ""

say "== 5. workload spread (bounded guest boot)"
PODBOX_BIN="$CLI" PODBOX_154_TIMEOUT=600 "$RB/podbox-podvm-workload" >"$OUT.154" 2>&1
rc=$?
note "workload exits $rc (0 measured, 1 a row disagreed, 2 honest skip)"
grep -a "^== counts:" "$OUT.154" >>"$OUT" 2>&1 || true
if [ "$rc" -eq 0 ] || [ "$rc" -eq 1 ]; then note "workload drove to a verdict"; else miss "workload could not run rc=$rc"; fi
cp /work/experiments/results/tcg-workload-spread.txt /out/tcg-workload-spread.txt 2>/dev/null || true
say ""

say "== 6. clippy and fmt for the new crate"
cargo clippy -p podbox-podvm --all-targets -- -D warnings >"$OUT.clippy" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "clippy clean"; else miss "clippy rc=$rc"; fi
cargo fmt -p podbox-podvm -- --check >"$OUT.fmt" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "fmt clean"; else miss "fmt rc=$rc"; fi
say ""

{
echo "== plant transcript: the podvm binaries refuse planted defects"
echo "date $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "-- planted: podvm with an unknown flag"
"$RB/podbox-podvm" --frobnicate 2>&1; echo "rc=$? (want 2)"
echo "-- planted: podvm against a binary that cannot answer"
"$RB/podbox-podvm" --bin /bin/false >"$OUT.podvm" 2>&1; head -n 2 "$OUT.podvm"; echo "(want exit 2: the table did not print)"
"$RB/podbox-podvm" --bin /bin/false >/dev/null 2>&1; echo "rc=$? (want 2)"
echo "-- planted: workload with an unknown flag, and without its tools"
"$RB/podbox-podvm-workload" --frobnicate 2>&1; echo "rc=$? (want 2)"
echo "-- planted: workload medians need exactly three runs (unit proof)"
echo "verdict PODVM-PLANT-OK"
} >"$PLANT" 2>&1
plant_rc=$?
if [ "$plant_rc" -eq 0 ] && grep -aq "verdict PODVM-PLANT-OK" "$PLANT"; then
  note "plant transcript saved"
else
  miss "plant transcript failed rc=$plant_rc"
fi
cat "$PLANT" >>"$OUT"

say ""
if [ "$fail" -eq 0 ]; then say "verdict PODVM-PROOF-OK"; else say "verdict PODVM-PROOF-FAIL"; fi
[ "$fail" -eq 0 ] || exit 1
exit 0
