#!/usr/bin/env bash
# Question: does the wsl-toolkit base expose a functional /dev/kvm, opened
# and driven, rather than a node that only stats?
#
# TODO/milestones.md T-1112. A node that exists and refuses every ioctl is
# not acceleration. This drive opens the node O_RDWR, issues
# KVM_GET_API_VERSION with a null argument, and creates a virtual machine.
# The null argument is load-bearing: passing a buffer to that _IO ioctl
# returns EINVAL on this kernel while the null shape returns 12.
# Exit 0 every clause held, 1 a clause disagreed, 2 could not run.
set -u

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 2
ROOT=$(CDPATH= cd -- "$HERE/.." && pwd) || exit 2
OUT="$ROOT/experiments/results/kvm-open.txt"

if ! command -v wsl-toolkit >/dev/null 2>&1 ||
   ! command -v timeout >/dev/null 2>&1; then
  echo 'kvm-open: wsl-toolkit and timeout are required' >&2
  exit 2
fi
work=$(mktemp -d) || exit 2
trap 'rm -rf "$work"' EXIT HUP INT TERM

cat >"$work/probe.sh" <<'PROBE'
#!/bin/sh
ls -l /dev/kvm || exit 1
python3 - <<'PY' || exit 1
import fcntl, os
fd = os.open('/dev/kvm', os.O_RDWR)
print('open O_RDWR ok')
api = fcntl.ioctl(fd, 0xAE00, 0)
print('KVM_GET_API_VERSION', api)
assert api == 12, api
vm = fcntl.ioctl(fd, 0xAE01, 0)
print('KVM_CREATE_VM fd', vm)
assert vm >= 0, vm
PY
PROBE

mkdir -p "$ROOT/experiments/results" || exit 2
{
  echo '== conditions'
  date -u +%Y-%m-%dT%H:%M:%SZ
  uname -sr
  timeout 1m wsl-toolkit --version
  git -C "$ROOT" rev-parse --short HEAD
  echo 'instance=podbox'
  echo
  echo '== kvm functional probe'
} >"$OUT"

timeout 5m wsl-toolkit --instance podbox base exec --script "$work/probe.sh" >"$work/probe.out" 2>&1
rc=$?
# The tool prints its own instance banner on stdout; that line names a
# home directory and carries no measurement, so it stays out of the file.
grep -av "instance podbox: distribution" "$work/probe.out" >>"$OUT" || true

{
  echo
  if [ "$rc" -eq 0 ]; then echo 'verdict           KVM OPEN HOLDS'; else echo 'verdict           KVM OPEN MISSING'; fi
} >>"$OUT"
cat "$OUT"
[ "$rc" -eq 0 ]
