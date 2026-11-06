#!/usr/bin/env sh
# T-1604: the detached-stdio serial mutex and the 1-of-3 detached start failure.
#
# Runs `cargo test --test detached_stdio` TWICE, as the entry's Prove clause
# requires, and captures each exit code from the PROCESS rather than from a pipe:
# a `cargo ... | tee` reports the pipe's status and a red suite reads green,
# which is why dev-lane.sh refuses such a job.
#
# The stdout of each run goes to its own file under /out and the `test result:`
# lines are printed here, so the returned log carries what actually ran.
set -u

# ⛔ These two files sit FLAT in the artifacts directory and end in `.log`:
# dev-lane.sh's own assertion greps "$ARTIFACTS"/*.log, so a log written into a
# subdirectory is a log it cannot see, and it answers "no test result reached".
# That is not a proof failure, but it is a harness refusal and it reads as one.
OUT=/out
mkdir -p "$OUT" || exit 2

./scripts/common/bootstrap-env.sh rust cc zig tools || exit 1

RUN1="$OUT/t1604-run1.log"
RUN2="$OUT/t1604-run2.log"

cargo test --target x86_64-unknown-linux-musl -p podbox-cli --test detached_stdio \
	>"$RUN1" 2>&1
rc1=$?

cargo test --target x86_64-unknown-linux-musl -p podbox-cli --test detached_stdio \
	>"$RUN2" 2>&1
rc2=$?

printf '%s\n' "== T-1604 detached_stdio, run twice"
printf 'run 1 exit %s\n' "$rc1"
printf 'run 2 exit %s\n' "$rc2"
printf '\n'
printf '%s\n' "== run 1: every test line and result"
grep -E '^test |^running |^test result:|^SKIP|panicked' "$RUN1" || printf '  (no lines)\n'
printf '\n'
printf '%s\n' "== run 2: every test line and result"
grep -E '^test |^running |^test result:|^SKIP|panicked' "$RUN2" || printf '  (no lines)\n'
printf '\n'

# ⛔ Both runs must have run the tests, not zero of them. `test result: ok.
# 0 passed` beside a real pass is fine; the ABSENCE of any pass is not, and it
# is the failure dev-lane.sh's artifact assertion exists to catch.
p1="$(grep -c 'test result: ok\. [1-9][0-9]* passed' "$RUN1" || true)"
p2="$(grep -c 'test result: ok\. [1-9][0-9]* passed' "$RUN2" || true)"
printf 'runs reporting a nonzero pass count: %s and %s\n' "$p1" "$p2"

if [ "$rc1" -ne 0 ] || [ "$rc2" -ne 0 ]; then
	printf '\n%s\n' "T-1604 LANE-RED"
	exit 1
fi
if [ "${p1:-0}" -eq 0 ] || [ "${p2:-0}" -eq 0 ]; then
	printf '\n%s\n' "T-1604 NO-TESTS-RAN"
	exit 1
fi

printf '\n%s\n' "T-1604 LANE-GREEN"
exit 0