#!/usr/bin/env bash
# Question: does the Windows wrapper run two distinct jobs, restore tracked
# executable modes, give a caller the right checkout, and leave no kept
# job after job-specific collection?
#
# The image digest fixes the job input. The caller path check expects exit 2
# because this small image has no podbox binary; its error must name /work.
# The report prints the host, version, image, commit, and each result.
# Exit 0 matched, 1 did not, 2 could not run.
set -u

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 2
ROOT=$(CDPATH= cd -- "$HERE/.." && pwd) || exit 2
OUT="$ROOT/experiments/results/windows-lane-v6.txt"
IMAGE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'

if ! command -v wsl-toolkit >/dev/null 2>&1 ||
   ! command -v jq >/dev/null 2>&1 ||
   ! command -v timeout >/dev/null 2>&1; then
  echo 'windows-lane-v6: wsl-toolkit, jq, and timeout are required' >&2
  exit 2
fi
work=$(mktemp -d) || exit 2
trap 'rm -rf "$work"' EXIT HUP INT TERM

timeout 3m wsl-toolkit --instance podbox gc --json >"$work/initial.json" 2>"$work/initial.err" || exit 2
jq -e '[.containers[]?, .guest_dirs[]?, .host_dirs[]?, .sessions[]?] | length == 0' "$work/initial.json" >/dev/null || {
  echo 'windows-lane-v6: clean the podbox job ledger before this drive' >&2
  exit 2
}

printf 'test -x /work/scripts/common/bootstrap-env.sh || exit 1\nprintf "job=one\\n"\n' >"$work/one.sh"
printf 'test -x /work/scripts/common/bootstrap-env.sh || exit 1\nprintf "job=two\\n"\n' >"$work/two.sh"

mkdir -p "$ROOT/experiments/results" || exit 2
{
  echo '== conditions'
  date -u +%Y-%m-%dT%H:%M:%SZ
  uname -sr
  timeout 3m wsl-toolkit --version
  git -C "$ROOT" rev-parse --short HEAD
  printf 'image=%s\n' "$IMAGE"
  echo 'instance=podbox'
  echo
  echo '== concurrent jobs and caller path'
} >"$OUT"

cd "$ROOT" || exit 2
PODBOX_IMAGE="$IMAGE" PODBOX_JOB_TIMEOUT=10m timeout 12m sh scripts/windows/run-in-base.sh "$work/one.sh" >"$work/one.log" 2>&1 &
one_pid=$!
PODBOX_IMAGE="$IMAGE" PODBOX_JOB_TIMEOUT=10m timeout 12m sh scripts/windows/run-in-base.sh "$work/two.sh" >"$work/two.log" 2>&1 &
two_pid=$!
wait "$one_pid"; one_rc=$?
wait "$two_pid"; two_rc=$?
PODBOX_IMAGE="$IMAGE" PODBOX_JOB_TIMEOUT=10m timeout 12m sh scripts/windows/run-in-base.sh experiments/352-ascii-output.sh >"$work/caller.log" 2>&1
caller_rc=$?

failed=0
[ "$one_rc" -eq 0 ] || failed=1
[ "$two_rc" -eq 0 ] || failed=1
[ "$caller_rc" -eq 2 ] || failed=1
grep -qx 'job=one' "$work/one.log" || failed=1
grep -qx 'job=two' "$work/two.log" || failed=1
grep -Fxq 'NO-BINARY: /work/target/x86_64-unknown-linux-musl/release/podbox' "$work/caller.log" || failed=1
if grep -qx 'job=two' "$work/one.log" || grep -qx 'job=one' "$work/two.log"; then failed=1; fi
[ ! -e "$ROOT/.podbox-job.sh" ] || failed=1

timeout 3m wsl-toolkit --instance podbox gc --json >"$work/before-gc.json" 2>"$work/gc.err"
gc_rc=$?
[ "$gc_rc" -eq 0 ] || failed=1
one_id=$(sed -n 's/^  job \([0-9a-f]*\).*/\1/p' "$work/one.log" | sed -n '1p')
two_id=$(sed -n 's/^  job \([0-9a-f]*\).*/\1/p' "$work/two.log" | sed -n '1p')
caller_id=$(sed -n 's/^  job \([0-9a-f]*\).*/\1/p' "$work/caller.log" | sed -n '1p')
[ -n "$one_id" ] && [ -n "$two_id" ] && [ -n "$caller_id" ] &&
  [ "$one_id" != "$two_id" ] && [ "$one_id" != "$caller_id" ] &&
  [ "$two_id" != "$caller_id" ] || failed=1
if [ -n "$one_id" ]; then
  timeout 3m wsl-toolkit --instance podbox gc --job "$one_id" --apply >"$work/gc-one.log" 2>&1 || failed=1
fi
if [ -n "$two_id" ]; then
  timeout 3m wsl-toolkit --instance podbox gc --job "$two_id" --apply >"$work/gc-two.log" 2>&1 || failed=1
fi
if [ -n "$caller_id" ]; then
  timeout 3m wsl-toolkit --instance podbox gc --job "$caller_id" --apply >"$work/gc-caller.log" 2>&1 || failed=1
fi
timeout 3m wsl-toolkit --instance podbox gc --json >"$work/after-gc.json" 2>>"$work/gc.err"
after_gc_rc=$?
[ "$after_gc_rc" -eq 0 ] || failed=1
if [ "$after_gc_rc" -eq 0 ]; then
  jq -e '[.containers[]?, .guest_dirs[]?, .host_dirs[]?, .sessions[]?] | length == 0' "$work/after-gc.json" >/dev/null || failed=1
fi

{
  printf 'one_exit=%s\n' "$one_rc"
  printf 'two_exit=%s\n' "$two_rc"
  printf 'caller_path_exit=%s\n' "$caller_rc"
  printf 'caller_path=%s\n' "$(if grep -Fxq 'NO-BINARY: /work/target/x86_64-unknown-linux-musl/release/podbox' "$work/caller.log"; then echo matched; else echo mismatch; fi)"
  printf 'one_payload=%s\n' "$(grep -c '^job=one$' "$work/one.log")"
  printf 'two_payload=%s\n' "$(grep -c '^job=two$' "$work/two.log")"
  printf 'gc_exit=%s\n' "$gc_rc"
  printf 'retained_before_gc=%s\n' "$(if [ "$gc_rc" -eq 0 ]; then jq -r '[.containers[]?, .guest_dirs[]?, .host_dirs[]?, .sessions[]?] | length' "$work/before-gc.json"; else echo unknown; fi)"
  printf 'retained_after_gc=%s\n' "$(if [ "$after_gc_rc" -eq 0 ]; then jq -r '[.containers[]?, .guest_dirs[]?, .host_dirs[]?, .sessions[]?] | length' "$work/after-gc.json"; else echo unknown; fi)"
  printf 'verdict=%s\n' "$(if [ "$failed" -eq 0 ]; then echo matched; else echo mismatch; fi)"
} >>"$OUT"
cat "$OUT"
if [ "$failed" -ne 0 ]; then
  echo 'windows-lane-v6: job logs follow for diagnosis' >&2
  cat "$work/one.log" "$work/two.log" "$work/caller.log" "$work/gc.err" >&2
  exit 1
fi
exit 0
