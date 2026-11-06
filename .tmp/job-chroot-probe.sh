#!/usr/bin/env sh
# Focused probe: is the chroot short-count environmental (podbox emits
# fewer/no workload lines with rc=0) or a port counting defect? Drives
# `podbox run` directly, six times per class, and shows per-run rc,
# stdout bytes, stderr bytes, and workload-line counts with raw bytes.
set -u

OUT=/out/chroot-probe.log
: >"$OUT" || exit 2

say() { printf '%s\n' "$*" >>"$OUT"; }

CLI=/work/target/x86_64-unknown-linux-musl/release/podbox
if [ ! -x "$CLI" ]; then
  cargo build --release --target x86_64-unknown-linux-musl -p podbox-cli >>"$OUT" 2>&1
  rc=$?
  [ "$rc" -eq 0 ] || exit 2
fi
[ -x "$CLI" ] || { say "FAIL: no CLI"; say "verdict PROBE-FAIL"; exit 1; }

REF="public.ecr.aws/docker/library/alpine@sha256:3e9b4b680bfc9fb5269227cffbd6d42be39fbf7c0b908123913864aa4447e764"
"$CLI" pull "$REF" >>"$OUT" 2>&1 || { say "FAIL: pull rc=$?"; say "verdict PROBE-FAIL"; exit 1; }
ROOTFS="$("$CLI" extract "$REF" 2>>"$OUT")" || { say "FAIL: extract"; say "verdict PROBE-FAIL"; exit 1; }
say "rootfs: $ROOTFS"
mkdir -p "$ROOTFS/bench" || { say "FAIL: cannot stage bench dir"; say "verdict PROBE-FAIL"; exit 1; }
for w in int sys mem io; do
  gcc -O2 -static -o "$ROOTFS/bench/bench-$w" "/work/experiments/154-bench-$w.c" >>"$OUT" 2>&1 || { say "FAIL: payload $w did not build"; say "verdict PROBE-FAIL"; exit 1; }
done

for w in int sys; do
  B="/bench/bench-$w"
  ls "$ROOTFS$B" >>"$OUT" 2>&1 || say "note: payload absent in rootfs"
  for i in 1 2 3 4 5 6; do
    out="$("$CLI" run --pull=never "$REF" "$B" 2>"$OUT.err")"
    rc=$?
    errb="$(wc -c <"$OUT.err")"
    outb="$(printf '%s' "$out" | wc -c)"
    lines="$(printf '%s\n' "$out" | grep -c "^workload=$w " || true)"
    say "run $w attempt $i: rc=$rc stdout_bytes=$outb stderr_bytes=$errb workload_lines=$lines"
    if [ "$lines" -ne 1 ]; then
      say "  raw stdout bytes then:"
      printf '%s\n' "$out" | head -n 5 >>"$OUT" 2>&1 || true
      say "  raw stderr bytes then:"
      head -n 5 "$OUT.err" >>"$OUT" 2>&1 || true
    fi
  done
done
rm -f "$OUT.err"
say "verdict PROBE-OK"
say ""
say "== phase 2: the binary's exact conditions (scratch store, scratch cwd)"
S2="$(mktemp -d)"
mkdir -p "$S2/work"
export PODBOX_STORE="$S2/store"
cd "$S2/work" || exit 2
"$CLI" pull "$REF" >>"$OUT" 2>&1 || { say "FAIL: phase2 pull"; say "verdict PROBE-FAIL"; exit 1; }
R2="$("$CLI" extract "$REF" 2>>"$OUT")" || { say "FAIL: phase2 extract"; say "verdict PROBE-FAIL"; exit 1; }
mkdir -p "$R2/bench" || exit 2
for w in int sys; do
  cp "$ROOTFS/bench/bench-$w" "$R2/bench/bench-$w" || exit 2
done
for w in int sys; do
  B="/bench/bench-$w"
  for i in $(seq 1 20); do
    out="$("$CLI" run --pull=never "$REF" "$B" 2>"$OUT.err2")"
    rc=$?
    errb="$(wc -c <"$OUT.err2")"
    outb="$(printf '%s' "$out" | wc -c)"
    lines="$(printf '%s\n' "$out" | grep -c "^workload=$w " || true)"
    if [ "$lines" -ne 1 ]; then
      say "phase2 run $w attempt $i: rc=$rc stdout_bytes=$outb stderr_bytes=$errb workload_lines=$lines MISMATCH"
      say "  raw stdout:"
      printf '%s\n' "$out" | head -n 8 >>"$OUT" 2>&1 || true
      say "  raw stderr:"
      head -n 8 "$OUT.err2" >>"$OUT" 2>&1 || true
    fi
  done
  say "phase2 $w: 20 attempts done"
done
rm -f "$OUT.err2"
rm -rf "$S2"
say "verdict PROBE2-OK"
say ""
say "== phase 3: distribution data, always logged"
export PODBOX_STORE="$ROOTFS_STORE"
cd "$S2W" 2>/dev/null || cd /work
for w in int sys; do
  B="/bench/bench-$w"
  for i in $(seq 1 20); do
    out="$("$CLI" run --pull=never "$REF" "$B" 2>"$OUT.err3")"
    rc=$?
    errb="$(wc -c <"$OUT.err3")"
    outb="$(printf '%s' "$out" | wc -c)"
    lines="$(printf '%s\n' "$out" | grep -c "^workload=$w " || true)"
    say "phase3 run $w attempt $i: rc=$rc stdout_bytes=$outb stderr_bytes=$errb workload_lines=$lines"
  done
done
rm -f "$OUT.err3"
say "verdict PROBE3-OK"
exit 0
