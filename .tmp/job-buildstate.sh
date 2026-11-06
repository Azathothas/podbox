#!/usr/bin/env sh
# Lane proof for T-1559/T-1560 (podbox-buildstate crate). Runs in the lane
# container as /in/job.sh with /work as the tree copy and /out for evidence.
# Ends with an exit-code capture, never a pipe on a cargo line.
set -u

OUT=/out/buildstate-proof.log
PLANT=/out/buildstate-plant.log
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
cargo test -p podbox-buildstate >"$OUT.test" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "cargo test -p podbox-buildstate exits 0"; else miss "cargo test rc=$rc"; fi
grep -a "test result:" "$OUT.test" >>"$OUT" 2>&1 || true
say ""

say "== 2. release binaries build"
cargo build --release -p podbox-buildstate >"$OUT.build" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "cargo build --release -p podbox-buildstate exits 0"; else miss "cargo build rc=$rc"; fi
BIN1=""
BIN2=""
for cand in /work/target/release/podbox-buildstate /work/target/x86_64-unknown-linux-musl/release/podbox-buildstate /work/target/debug/podbox-buildstate; do
  [ -x "$cand" ] && BIN1="$cand" && break
done
for cand in /work/target/release/podbox-release-licenses /work/target/x86_64-unknown-linux-musl/release/podbox-release-licenses /work/target/debug/podbox-release-licenses; do
  [ -x "$cand" ] && BIN2="$cand" && break
done
[ -n "$BIN1" ] && [ -n "$BIN2" ] || miss "binaries absent after build"
note "binaries: $BIN1 $BIN2"
say ""

say "== 3. agreement: retired script against the binary, same tree, same env"
git show HEAD:scripts/build-state.py >"$OUT.orig-bs.py" 2>&1
# The licenses script derives its root from its own path (no --root
# flag), so it is extracted where parent.parent is /work, like the
# document-state agreement in the release proof.
mkdir -p /work/agree-lic
git show HEAD:scripts/release-licenses.py >/work/agree-lic/orig-lic.py 2>&1
# Both drivers are pointed at the same root: the retired scripts derive it
# from their own path, so the copies are invoked with an explicit --root.
A1="$(python3 "$OUT.orig-bs.py" inputs --root /work --target x86_64-unknown-linux-musl 2>"$OUT.pyerr")"
B1="$("$BIN1" inputs --root /work --target x86_64-unknown-linux-musl 2>"$OUT.binerr")"
if [ -n "$A1" ] && [ "$A1" = "$B1" ]; then note "inputs digests agree: $B1"; else miss "inputs differ: py=[$A1] rs=[$B1]"; fi
A2="$(python3 "$OUT.orig-bs.py" commit --root /work 2>>"$OUT.pyerr")"
B2="$("$BIN1" commit --root /work 2>>"$OUT.binerr")"
if [ -n "$A2" ] && [ "$A2" = "$B2" ]; then note "commit agrees: $B2"; else miss "commit differs: py=[$A2] rs=[$B2]"; fi
say ""

say "== 4. cross-read: each writer's record reads ready under the other"
S4="$(mktemp -d)"
mkdir -p "$S4/crates/a"
echo "scratch-input" >"$S4/crates/a/f.rs"
for n in podbox node operator proxy shell; do echo "scratch-out" >"$S4/$n"; done
"$BIN1" record --root "$S4" --target t --binary "$S4/podbox" >"$OUT.rec1" 2>&1
rc=$?
[ "$rc" -eq 0 ] || miss "binary record rc=$rc"
S1="$(python3 "$OUT.orig-bs.py" status --root "$S4" --target t --binary "$S4/podbox" 2>&1)"
rc=$?
if [ "$rc" -eq 0 ] && [ "$S1" = "ready" ]; then note "python reads the binary record: ready"; else miss "python on binary record: [$S1] rc=$rc"; fi
rm -f "$S4/.dev/build-state.json"
python3 "$OUT.orig-bs.py" record --root "$S4" --target t --binary "$S4/podbox" >"$OUT.rec2" 2>&1
"$BIN1" status --root "$S4" --target t --binary "$S4/podbox" >"$OUT.st2" 2>&1
rc=$?
S2="$(cat "$OUT.st2")"
if [ "$rc" -eq 0 ] && [ "$S2" = "ready" ]; then note "binary reads the python record: ready"; else miss "binary on python record: [$S2] rc=$rc"; fi
rm -rf "$S4"
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
check_chain "build-state shim" "unknown action" python3 /work/scripts/build-state.py frobnicate
check_chain "licenses shim" "--output DIR is required" python3 /work/scripts/release-licenses.py
say ""

say "== 5. licences: identical inventories over the same lock"
rm -rf "$OUT.lic-py" "$OUT.lic-rs"
# Warm the registry index first: a cold `cargo metadata` fails on the
# script side while the binary succeeds, which reads as a port defect
# instead of an empty cache. Bounded, both manifests the script reads.
cargo metadata --locked --format-version 1 --manifest-path /work/Cargo.toml >/dev/null 2>&1
cargo metadata --locked --format-version 1 --manifest-path /work/crates/podbox-interpose/Cargo.toml >/dev/null 2>&1
python3 /work/agree-lic/orig-lic.py --output "$OUT.lic-py" >"$OUT.lic1" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "python licences exits 0"; else miss "python licences rc=$rc"; fi
if [ "$rc" -ne 0 ]; then
  say "  the script's own output then:"
  tail -n 20 "$OUT.lic1" >>"$OUT" 2>&1 || true
fi
rm -rf /work/agree-lic
"$BIN2" --output "$OUT.lic-rs" >"$OUT.lic2" 2>&1
rc=$?
[ "$rc" -eq 0 ] || miss "binary licences rc=$rc"
if diff -r "$OUT.lic-py" "$OUT.lic-rs" >"$OUT.licdiff" 2>&1; then note "inventories identical"; else miss "inventories differ"; fi
say ""

say "== 6. freshness fixture through the binary (T-1349's proof, re-driven)"
python3 /work/experiments/393-build-freshness.py >"$OUT.393" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "393 exits 0 against the binary"; else miss "393 rc=$rc"; fi
grep -a "^ok:\|^verdict" "$OUT.393" >>"$OUT" 2>&1 || true
say ""

say "== 7. clippy and fmt for the new crate"
cargo clippy -p podbox-buildstate --all-targets -- -D warnings >"$OUT.clippy" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "clippy clean"; else miss "clippy rc=$rc"; fi
cargo fmt -p podbox-buildstate -- --check >"$OUT.fmt" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "fmt clean"; else miss "fmt rc=$rc"; fi
say ""

{
echo "== plant transcript: podbox-buildstate refuses planted defects"
echo "date $(date -u +%Y-%m-%dT%H:%M:%SZ)"
WORK="$(mktemp -d)"
mkdir -p "$WORK/crates/a"
echo "fixture" >"$WORK/crates/a/f.rs"
echo "build-bytes" >"$WORK/podbox"
for n in node operator proxy shell; do echo "helper" >"$WORK/$n"; done
echo "-- record then status: ready"
"$BIN1" record --root "$WORK" --target t --binary "$WORK/podbox"
"$BIN1" status --root "$WORK" --target t --binary "$WORK/podbox"; echo "rc=$? (want 0 ready)"
echo "-- planted: output byte flipped"
echo "changed" >"$WORK/podbox"
"$BIN1" status --root "$WORK" --target t --binary "$WORK/podbox"; echo "rc=$? (want 1 stale: binary bytes changed)"
echo "build-bytes" >"$WORK/podbox"
echo "-- planted: input byte flipped, timestamp restored"
f="$WORK/crates/a/f.rs"; st="$(stat -c %y "$f")"
echo "fixture2" >"$f"; touch -d "$st" "$f"
"$BIN1" status --root "$WORK" --target t --binary "$WORK/podbox"; echo "rc=$? (want 1 stale: build inputs changed)"
echo "fixture" >"$WORK/crates/a/f.rs"
echo "-- planted: record schema rewritten"
sed -i 's/podbox-build\/1/other\/9/' "$WORK/.dev/build-state.json"
"$BIN1" status --root "$WORK" --target t --binary "$WORK/podbox"; echo "rc=$? (want 1 stale: build record schema differs)"
echo "-- planted: record corrupted"
echo "not json" >"$WORK/.dev/build-state.json"
"$BIN1" status --root "$WORK" --target t --binary "$WORK/podbox"; echo "rc=$? (want 1 stale: unreadable build record)"
rm -f "$WORK/.dev/build-state.json"
echo "-- planted: binary removed"
rm -f "$WORK/podbox"
"$BIN1" status --root "$WORK" --target t --binary "$WORK/podbox"; echo "rc=$? (want 2 absent)"
echo "-- planted: licences with no output flag, and an unwritable output"
"$BIN2" 2>&1; echo "rc=$? (want 2)"
"$BIN2" --output /proc/cannot-write-here 2>&1; echo "rc=$? (want 2)"
rm -rf "$WORK"
echo "verdict BUILDSTATE-PLANT-OK"
} >"$PLANT" 2>&1
plant_rc=$?
if [ "$plant_rc" -eq 0 ] && grep -aq "verdict BUILDSTATE-PLANT-OK" "$PLANT"; then
  note "plant transcript saved"
else
  miss "plant transcript failed rc=$plant_rc"
fi
cat "$PLANT" >>"$OUT"

say ""
if [ "$fail" -eq 0 ]; then say "verdict BUILDSTATE-PROOF-OK"; else say "verdict BUILDSTATE-PROOF-FAIL"; fi
[ "$fail" -eq 0 ] || exit 1
exit 0
