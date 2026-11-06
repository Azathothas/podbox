#!/usr/bin/env sh
# T-1604 PLANT CONTROL: the SAME plant against the PRE-FIX mutex shape.
#
# The plant job above proves the neighbours survive. This proves the plant is a
# real check rather than a green check nobody has seen refuse: the same panicking
# test, in the same binary, with the three guard takes put BACK to
# `SERIAL.lock().unwrap()`, must reproduce the CI cascade.
#
# EXPECTED: this job exits 0 having found the cascade, and prints
# T-1604 PLANT-CONTROL-CASCADE. It asserts the CASCADE IS PRESENT. If the
# cascade is absent, this job exits non-zero, because a control that cannot see
# the defect proves nothing about the fix.
set -u

OUT=/out
mkdir -p "$OUT" || exit 2

./scripts/common/bootstrap-env.sh rust cc zig tools || exit 1

TARGET=crates/podbox-cli/tests/detached_stdio.rs
cp "$TARGET" "$OUT/t1604-control-before.rs" || exit 2

cleanup() {
	cp "$OUT/t1604-control-before.rs" "$TARGET" 2>/dev/null
}
trap cleanup EXIT INT TERM

# ⛔ Put the defect back: three guard takes become unwrap() again, which is the
# exact shape CI reported at :138 as a PoisonError. `sed` in the guest copy only;
# the host tree is not touched.
sed -i 's/SERIAL\.lock()\.unwrap_or_else(|e| e\.into_inner())/SERIAL.lock().unwrap()/g' "$TARGET"
if ! grep -q 'SERIAL\.lock()\.unwrap();' "$TARGET"; then
	printf '%s\n' "T-1604 PLANT-CONTROL-NO-SED"
	exit 1
fi
printf 'guard takes put back to unwrap(): %s\n' \
	"$(grep -c 'SERIAL\.lock()\.unwrap();' "$TARGET")"

# The same panicking test, holding the guard exactly as before.
cat >>"$TARGET" <<'PLANT'

/// T-1604 PLANT CONTROL (temporary): the defect's own shape, restored.
#[test]
fn t1604_plant_panics_while_holding_the_serial_guard() {
	let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
	panic!("T-1604 plant: a deliberate failure while holding SERIAL");
}
PLANT

LOG="$OUT/t1604-control-run.log"
cargo test --target x86_64-unknown-linux-musl -p podbox-cli --test detached_stdio \
	>"$LOG" 2>&1
rc=$?

printf '%s\n' "== T-1604 PLANT CONTROL: pre-fix shape, plant exit"
printf 'exit %s\n' "$rc"
printf '\n'
printf '%s\n' "== every test line"
grep -E '^test |^running |^test result:|PoisonError|panicked at' "$LOG" || printf '  (no lines)\n'
printf '\n'

# ⛔ THE CONTROL. A neighbour must now die on the poison this plant left. If none
# does, the plant never poisoned anything and the green run above proves nothing.
if grep -q 'PoisonError' "$LOG"; then
	printf '%s\n' "cascade reproduced against the pre-fix shape"
	printf '%s\n' "T-1604 PLANT-CONTROL-CASCADE"
	exit 0
fi

printf '%s\n' "PLANT CONTROL FAILED: no PoisonError, so the plant never poisoned a"
printf '%s\n' "neighbour and the green run proves nothing"
printf '%s\n' "T-1604 PLANT-CONTROL-NO-CASCADE"
exit 1