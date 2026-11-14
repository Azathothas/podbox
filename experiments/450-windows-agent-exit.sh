#!/bin/sh
# Question: does a payload that ends in `exit` return its own code from a
# Windows guest, and not a timeout?
#
# TODO/machine.md T-1864. Before podcrates v0.45.0 the agent ran the
# command with `call`, so an `exit` in it ended the agent, and the run ran
# to the timeout. The agent of v0.45.0 runs it in a child `cmd /c`
# (podcrates@T-592).
#
# Pinned inputs:
#   VHDX    PODVM_450_VHDX: ValidationOS.vhdx of build 26100.9818, as
#           `podvm pull windows:validationos` stores it (sha256 a0f7900b...)
#   podvm   PODVM_450_BIN (default the release build of the workspace)
#   accel   whatever the host gives: KVM where /dev/kvm opens, TCG else
#
# Clauses:
#   1. `windows setup` provisions the disk (the agent), once;
#   2. `exit 42` gives 42; `cmd /c exit 42` gives 42; `echo out & exit 7`
#      prints `out` and gives 7; none is a timeout (each run has a bound
#      of 600 s);
#   3. no emulator stays after the runs.

set -u
ROOT=$(cd "$(dirname "$0")/.." && pwd)
BIN="${PODVM_450_BIN:-$ROOT/target/x86_64-unknown-linux-musl/release/podvm}"
VHDX="${PODVM_450_VHDX:?set PODVM_450_VHDX to the ValidationOS.vhdx}"
OUT="$ROOT/experiments/results/windows-agent-exit.txt"
PROV="$VHDX.podvm.qcow2"
fail=0
: >"$OUT"
say() { printf '%s\n' "$*" | tee -a "$OUT"; }

if [ ! -f "$PROV" ]; then
	t0=$(date +%s)
	timeout 2400 "$BIN" windows setup --image "$VHDX" --podvm-mem 6G --podvm-cpus 4 --podvm-timeout 900 >/dev/null 2>"$OUT.setup"
	rc=$?
	say "setup: rc $rc in $(( $(date +%s) - t0 )) s"
	[ $rc -eq 0 ] && [ -f "$PROV" ] || { say "FAIL: clause 1"; tail -5 "$OUT.setup" | tee -a "$OUT"; exit 1; }
fi

run() {
	want=$1; shift
	t0=$(date +%s)
	out=$(timeout 900 "$BIN" windows run --image "$PROV" --podvm-timeout 600 -- "$@" 2>"$OUT.err")
	rc=$?
	say "run [$*]: rc $rc in $(( $(date +%s) - t0 )) s, stdout [$(printf '%s' "$out" | tr -d '\r' | tr '\n' ' ')]"
	[ $rc -eq "$want" ] || { say "FAIL: [$*] wants $want"; tail -3 "$OUT.err" | tee -a "$OUT"; fail=1; }
}
run 42 exit 42
run 42 cmd /c exit 42
run 7 echo out '&' exit 7

if pgrep -f qemu-system >/dev/null; then say "FAIL: clause 3, an emulator stays"; fail=1; fi
rm -f "$OUT.err"
[ $fail -eq 0 ] && say "PASS: each clause" || say "FAIL"
exit $fail
