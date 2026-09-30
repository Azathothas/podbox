#!/bin/sh
# Does the ValidationOS guest use KVM and return its output and exit status?
# T-1112 and T-1350. The host driver accepts a freshly built binary and image.
# The image length and digest are pinned in lib/kvm-guest-base.sh.
# Exit: 0 matched, 1 failed, 2 could not run.
set -u
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 2
ROOT=$(CDPATH= cd -- "$HERE/.." && pwd) || exit 2
if command -v py >/dev/null 2>&1; then
    exec py "$ROOT/scripts/windows/kvm-guest.py" "$@"
fi
echo "kvm-guest: the Windows Python launcher is not on PATH" >&2
exit 2
