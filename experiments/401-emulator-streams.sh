#!/bin/sh
# Does the Windows guest driver drain or redirect the emulator's streams?
# T-1112. A fake emulator writes 256 KiB to each stream and exits.
# The current source must pass the unit test. A mutation that restores the
# unread pipes must fail it. No guest, image, or KVM device is used.
# Exit: 0 matched, 1 failed, 2 could not run.
set -u
ROOT=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
cd "$ROOT" || exit 2
[ -f crates/podbox-windows/src/lib.rs ] || { echo "cannot run: no checkout at $ROOT"; exit 2; }
command -v cargo >/dev/null 2>&1 || { echo "cannot run: cargo is absent"; exit 2; }
TEST=a_noisy_emulator_is_not_blocked_on_its_own_output
SRC=crates/podbox-windows/src/lib.rs
SAVED=$(mktemp) || exit 2
trap 'cp "$SAVED" "$SRC"; rm -f "$SAVED"' EXIT HUP INT TERM
cp "$SRC" "$SAVED" || exit 2

echo "== conditions"
date -u +%Y-%m-%dT%H:%M:%SZ
uname -sr
cargo --version

echo "== current source"
if timeout 900 cargo test -q -p podbox-windows --lib "$TEST" >current.log 2>&1; then
  echo "ok: $TEST passes"
else
  tail -n 20 current.log
  echo "verdict EMULATOR-STREAMS-FAIL: the current source fails its test"
  exit 1
fi

echo "== mutation: unread pipes restored"
sed -e 's/\.stdout(Stdio::null())$/.stdout(Stdio::piped())/' \
    -e 's/^        \.stderr(stderr);$/        .stderr(Stdio::piped());/' "$SAVED" >"$SRC"
if cmp -s "$SAVED" "$SRC"; then
  echo "cannot run: the mutation did not change the source"
  exit 2
fi
if timeout 900 cargo test -q -p podbox-windows --lib "$TEST" >mutant.log 2>&1; then
  echo "verdict EMULATOR-STREAMS-FAIL: the test passed against unread pipes"
  exit 1
fi
grep -E "blocked for|panicked" mutant.log | head -n 3
echo "ok: the mutation fails the test"
echo "verdict EMULATOR-STREAMS-OK"
