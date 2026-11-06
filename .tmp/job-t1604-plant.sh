#!/usr/bin/env sh
# T-1604 PLANT: a deliberately failing test in the SAME binary must not poison
# its neighbours.
#
# The job appends one panicking test to crates/podbox-cli/tests/detached_stdio.rs
# (temporarily, inside the guest copy), runs the suite, and asserts the two
# neighbours still report THEIR OWN results rather than a PoisonError. The
# appended test is removed again before the job exits, so the returned artifacts
# describe the tree as it was.
#
# EXPECTED: this job exits NON-zero. The plant test is supposed to fail. What is
# checked here is the neighbours' lines, not the suite's exit code.
set -u

# ⛔ Flat in /out and ending in `.log`: dev-lane.sh asserts on "$ARTIFACTS"/*.log,
# so the plant transcript has to land where its assertion can read it. This job
# is EXPECTED to exit 0 while the suite inside it exits non-zero, because the
# plant test is supposed to fail; what is checked is the neighbours' lines.
OUT=/out
mkdir -p "$OUT" || exit 2

./scripts/common/bootstrap-env.sh rust cc zig tools || exit 1

TARGET=crates/podbox-cli/tests/detached_stdio.rs
cp "$TARGET" "$OUT/t1604-plant-before.rs" || exit 2

cleanup() {
	cp "$OUT/t1604-plant-before.rs" "$TARGET" 2>/dev/null
}
trap cleanup EXIT INT TERM

cat >>"$TARGET" <<'PLANT'

/// T-1604 PLANT (temporary). Panics while holding SERIAL. The two hermetic
/// tests in this binary must still report their own results afterwards.
#[test]
fn t1604_plant_panics_while_holding_the_serial_guard() {
	let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
	panic!("T-1604 plant: a deliberate failure while holding SERIAL");
}
PLANT

LOG="$OUT/t1604-plant-run.log"
cargo test --target x86_64-unknown-linux-musl -p podbox-cli --test detached_stdio \
	>"$LOG" 2>&1
rc=$?

printf '%s\n' "== T-1604 PLANT: plant exit (non-zero is EXPECTED and correct)"
printf 'exit %s\n' "$rc"
printf '\n'
printf '%s\n' "== every test line, so the neighbours' own results are visible"
grep -E '^test |^running |^test result:|PoisonError|panicked at' "$LOG" || printf '  (no lines)\n'
printf '\n'

# ⛔ THE ASSERTION. The two hermetic neighbours must appear as their own results.
# A neighbour that reads `... PoisonError ...` instead of `ok` is the defect the
# mutex fix removes, so its ABSENCE is the plant passing and its presence is the
# plant failing. Naming them keeps this from passing on a suite that ran nothing.
for neighbour in run_help_names_the_detach_flag detached_run_without_an_image_is_a_cli_error; do
	if ! grep -qE "test $neighbour .*\.\.\. ok" "$LOG"; then
		printf 'PLANT FAILED: %s did not report ok; the cascade reached it\n' "$neighbour"
		printf '%s\n' "T-1604 PLANT-CASCADE-PRESENT"
		exit 1
	fi
	printf 'neighbour reported its own result: %s\n' "$neighbour"
done

# ⛔ And the plant itself must have actually run and actually failed, or this
# proves nothing: a filtered-out suite would report the same neighbours as ok.
if ! grep -q 't1604_plant_panics_while_holding_the_serial_guard' "$LOG"; then
	printf '%s\n' "PLANT FAILED: the plant test never ran"
	printf '%s\n' "T-1604 PLANT-NOT-RUN"
	exit 1
fi

printf '\n%s\n' "T-1604 PLANT-NO-CASCADE"
exit 0