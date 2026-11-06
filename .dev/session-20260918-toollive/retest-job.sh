#!/bin/sh
# Control for the 5 store-test failures seen in the 2026-09-18 gate run.
# Bootstraps first (a user job does not get the default wrapper's bootstrap),
# then runs the podbox-image lib suite twice: default parallelism, then serial.
# No pipes: every exit code is read from the process that produced it.
set -u
cd /work || exit 2
./scripts/common/bootstrap-env.sh rust cc zig tools >job-bootstrap.log 2>&1
rc=$?
echo "bootstrap_exit=$rc"
if [ "$rc" != 0 ]; then echo "BOOTSTRAP_FAILED"; exit 2; fi
echo "== run 1: default parallelism"
cargo test -p podbox-image --lib >job-run1.log 2>&1
rc1=$?
echo "run1_exit=$rc1"
grep -E "^test result|FAILED|failures:$" job-run1.log
echo "== run 2: one thread"
cargo test -p podbox-image --lib -- --test-threads=1 >job-run2.log 2>&1
rc2=$?
echo "run2_exit=$rc2"
grep -E "^test result|FAILED|failures:$" job-run2.log
echo "== done"
if [ "$rc1" != 0 ] || [ "$rc2" != 0 ]; then exit 1; fi
exit 0
