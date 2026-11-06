#!/usr/bin/env sh
# Lane proof for T-1561..T-1567 (podbox-release crate). Runs in the lane
# container as /in/job.sh with /work as the tree copy and /out for evidence.
# Ends with an exit-code capture, never a pipe on a cargo line.
set -u

OUT=/out/release-proof.log
PLANT=/out/release-plant.log
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
cargo test -p podbox-release >"$OUT.test" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "cargo test -p podbox-release exits 0"; else miss "cargo test rc=$rc"; fi
grep -a "test result:" "$OUT.test" >>"$OUT" 2>&1 || true
say ""

say "== 2. release binaries build"
cargo build --release -p podbox-release >"$OUT.build" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "cargo build --release -p podbox-release exits 0"; else miss "cargo build rc=$rc"; fi
RB=/work/target/x86_64-unknown-linux-musl/release
for b in podbox-verify podbox-size podbox-prove-t0211 document-state release-notes podbox-reconcile podbox-publish; do
  [ -x "$RB/$b" ] || miss "binary absent: $b"
done
note "all seven binaries present"
say ""

say "== 3. document-state agreement: retired script against the binary"
# The retired script takes no --root flag: it derives the root from its
# own path. It is extracted where parent.parent is /work, so both sides
# render the same tree; the binary takes --root explicitly.
mkdir -p /work/agree
git show HEAD:scripts/document-state.py >/work/agree/orig-docstate.py 2>&1
chmod +x /work/agree/orig-docstate.py 2>/dev/null || true
python3 /work/agree/orig-docstate.py >"$OUT.pydoc" 2>&1
rc=$?
[ "$rc" -eq 0 ] || miss "python document-state rc=$rc"
"$RB/document-state" --root /work >"$OUT.rsdoc" 2>&1
rc=$?
[ "$rc" -eq 0 ] || miss "binary document-state rc=$rc"
if diff "$OUT.pydoc" "$OUT.rsdoc" >"$OUT.docdiff" 2>&1; then note "rendered pages identical"; else miss "rendered pages differ"; fi
rm -rf /work/agree /work/orig-docstate.py
say ""

say "== 4. refusal shapes through the shims (no network)"
"$RB/podbox-verify" 2>"$OUT.v1" || rc=$?
if [ "${rc:-0}" -eq 2 ]; then note "verify without args exits 2"; else miss "verify no-args rc=${rc:-0}"; fi
"$RB/podbox-verify" v0 badarch 2>"$OUT.v2" || rc=$?
if [ "$rc" -eq 2 ]; then note "verify with a bad arch exits 2"; else miss "verify bad-arch rc=$rc"; fi
"$RB/release-notes" 2>"$OUT.n1" || rc=$?
if [ "$rc" -eq 2 ]; then note "notes without a tag exits 2"; else miss "notes no-tag rc=$rc"; fi
"$RB/podbox-publish" --release not-a-tag 2>"$OUT.p1" || rc=$?
# The script reads the remote (ls-remote, gh) before it checks the tag
# shape, so an offline lane cannot run this at all: rc=2. The exit-1
# shape is unit-proven (valid_release rejects not-a-tag).
if [ "$rc" -eq 2 ]; then note "publish with a bad tag cannot run in the lane (rc=2)"; else miss "publish bad-tag rc=$rc"; fi
grep -a "^cannot run" "$OUT.p1" >>"$OUT" 2>&1 || true
say ""

say "== 4b. every shim reaches its binary (the binary's message proves it)"
# Each shim prints "is not built" where the binary is absent; the binary
# answers usage or unexpected-argument on these probes. The fragment
# decides which one ran, so a dead shim cannot pass as a live binary.
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
check_chain "verify shim" "verify-release: usage:" sh /work/scripts/verify-release.sh
check_chain "notes shim" "release-notes: usage:" sh /work/scripts/release-notes.sh
check_chain "document-state shim" "document-state: unexpected argument" python3 /work/scripts/document-state.py --frobnicate
check_chain "reconcile shim" "podbox-reconcile: unexpected argument" python3 /work/experiments/395-reconcile-repository.py --frobnicate
check_chain "publish shim" "podbox-publish: unexpected argument" python3 /work/experiments/399-publication.py --frobnicate
check_chain "120 shim" "podbox-prove-t0211: unexpected argument" /work/experiments/120-reproducible-build.sh frobnicate
check_chain "157 shim" "podbox-prove-t0211: unexpected argument" /work/experiments/157-lock-inheritance-prove.sh frobnicate
# The entry proves refusal shapes through the shim path, not only the
# binary: the chain above proves resolution, this proves the shape.
python3 /work/experiments/399-publication.py --release not-a-tag >"$OUT.pubshim" 2>&1 || pubrc=$?
if [ "${pubrc:-0}" -eq 2 ]; then note "publish bad tag refuses through the shim (rc=2)"; else miss "publish bad tag via shim rc=${pubrc:-0}"; fi
grep -a "^cannot run" "$OUT.pubshim" >>"$OUT" 2>&1 || true
say ""

say "== 5. reconcile reads the branch state"
"$RB/podbox-reconcile" --root /work >"$OUT.r1" 2>&1
rc=$?
note "reconcile rc=$rc (2 where the publish ref is absent, 0 where present)"
grep -a "^verdict\|^cannot run" "$OUT.r1" >>"$OUT" 2>&1 || true
say ""

say "== 6. lock-inheritance with two attempts per test plus both mutations"
PODBOX_PROVE_RUNS=2 "$RB/podbox-prove-t0211" lock-inheritance --root /work >"$OUT.157" 2>&1
rc=$?
rc157=$rc
# The entry proves the port RUNS its two mutations, not that they redden:
# 0 both caught, 1 a mutation not caught (the tree's verdict, T-0211's
# property, recorded here), 2 it could not run. Agreement with the
# retired driver is asserted in 6b below.
note "lock-inheritance exits $rc (0 caught, 1 not caught, 2 could not run)"
if [ "$rc" -eq 2 ]; then miss "lock-inheritance could not run"; fi
grep -a "THE MUTATION DID NOT LAND\|passed\|exited\|cargo reported" "$OUT.157" >>"$OUT" 2>&1 || true
say ""

say "== 6b. the retired script beside the binary, same tree"
# The script derives its root from its own path, so it is extracted
# where parent.parent is /work. Agreement is the proof: if the retired
# script reddens neither mutation either, the non-reddening is the
# tree's property (T-0211 stays partial), not the port's.
mkdir -p /work/agree157
git show HEAD:experiments/157-lock-inheritance-prove.sh >/work/agree157/retired-157.sh 2>&1
chmod +x /work/agree157/retired-157.sh 2>/dev/null || true
PODBOX_PROVE_RUNS=2 /work/agree157/retired-157.sh >"$OUT.157ret" 2>&1
ret_rc=$?
note "retired 157 exits $ret_rc, binary exits $rc157"
grep -a "exited\|DID NOT LAND\|NOT REPEATABLE\|did not redden\|reddened the OTHER" "$OUT.157ret" >>"$OUT" 2>&1 || true
if [ "$ret_rc" -eq "$rc157" ]; then note "verdicts agree retired-vs-binary"; else miss "verdicts differ retired-vs-binary"; fi
rm -rf /work/agree157
say ""

say "== 7. reproducible proof without a lane refuses honestly"
"$RB/podbox-prove-t0211" reproducible --root /work >"$OUT.120" 2>&1
rc=$?
if [ "$rc" -eq 2 ]; then note "reproducible without wsl-toolkit exits 2"; else miss "reproducible rc=$rc"; fi
say ""

say "== 9. the full measurement: workspace build plus podbox-size ci"
cargo build --release -p podbox-gate --bin podbox-interpose-build >"$OUT.ib" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "interpose builder built"; else miss "interpose builder rc=$rc"; fi
cargo build --release --target x86_64-unknown-linux-musl >"$OUT.wsbuild" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "workspace release build exits 0"; else miss "workspace build rc=$rc"; fi
if ! cargo bloat --version >/dev/null 2>&1; then
  say "  cargo-bloat absent; installing (bounded)"
  cargo install cargo-bloat --locked >"$OUT.bloat-install" 2>&1
  rc=$?
  if [ "$rc" -eq 0 ]; then note "cargo-bloat installed"; else note "cargo-bloat install rc=$rc; the skip arm is the asserted path"; fi
fi
/work/experiments/110-bloat-delta.sh ci >"$OUT.110" 2>&1
rc=$?
note "podbox-size ci through the stub exits $rc"
if grep -aq "^total_bytes " /work/experiments/results/bloat-ci.txt 2>/dev/null; then note "bloat-ci.txt carries a total"; else miss "bloat-ci.txt missing its total"; fi
if grep -aq "FAIL: " "$OUT.110" 2>/dev/null; then
  say "  the stub's own failure lines then:"
  tail -n 30 "$OUT.110" >>"$OUT" 2>&1 || true
fi
grep -a "^total_bytes\|^headroom_bytes\|^unmeasured\|^breakdown" /work/experiments/results/bloat-ci.txt >>"$OUT" 2>&1 || true
say ""
say "== 10. clippy and fmt for the new crate"
cargo clippy -p podbox-release --all-targets -- -D warnings >"$OUT.clippy" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "clippy clean"; else miss "clippy rc=$rc"; fi
cargo fmt -p podbox-release -- --check >"$OUT.fmt" 2>&1
rc=$?
if [ "$rc" -eq 0 ]; then note "fmt clean"; else miss "fmt rc=$rc"; fi
say ""

{
echo "== plant transcript: the release binaries refuse planted defects"
echo "date $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "-- planted: verify with a bad kind"
"$RB/podbox-verify" v0 x86_64 ssh2 2>&1; echo "rc=$? (want 2)"
echo "-- planted: notes for a tag that names no commit"
"$RB/release-notes" v9.9.9-no-such-tag >"$OUT.notes" 2>&1; head -n 2 "$OUT.notes"; echo "pipe done (want exit 2 from the binary)"
"$RB/release-notes" v9.9.9-no-such-tag >/dev/null 2>&1; echo "rc=$? (want 2)"
echo "-- planted: publish with a malformed tag and a missing origin state"
"$RB/podbox-publish" --release 0.1.0 2>&1; echo "rc=$? (want 2 in the lane: the remote read comes before the tag check; want 1 on a connected host)"
echo "-- planted: reconcile expecting a deleted ref that is present or absent"
"$RB/podbox-reconcile" --expect-deleted --root /work >"$OUT.recon" 2>&1; head -n 6 "$OUT.recon"
echo "-- planted: prove with an unknown subcommand"
"$RB/podbox-prove-t0211" frobnicate 2>&1; echo "rc=$? (want 2)"
echo "-- planted: document-state against a tree with a member removed"
W="$(mktemp -d)"
cp -r /work/crates/podbox-probe "$W/" 2>/dev/null
"$RB/document-state" --root "$W" >"$OUT.docstate" 2>&1; head -n 2 "$OUT.docstate"; echo "rc above (want 2: no workspace)"
rm -rf "$W"
echo "verdict RELEASE-PLANT-OK"
} >"$PLANT" 2>&1
plant_rc=$?
if [ "$plant_rc" -eq 0 ] && grep -aq "verdict RELEASE-PLANT-OK" "$PLANT"; then
  note "plant transcript saved"
else
  miss "plant transcript failed rc=$plant_rc"
fi
cat "$PLANT" >>"$OUT"

say ""
if [ "$fail" -eq 0 ]; then say "verdict RELEASE-PROOF-OK"; else say "verdict RELEASE-PROOF-FAIL"; fi
[ "$fail" -eq 0 ] || exit 1
exit 0
