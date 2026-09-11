#!/usr/bin/env bash
# Question: lima-vm/lima v2.2.0 — a manager on the qemu backend, so it should
# be TCG-viable here. Can limactl create AND boot a real guest (Alpine template)
# end to end without /dev/kvm, and how long does the boot take under TCG?
# Exit codes: 0 guest booted and shell command executed inside, 1 ran but failed, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/33-lima.log"
conditions lima > "$LOG"

LDIR="$VR_WORK/lima-2.2.0"
LIMACTL="$LDIR/bin/limactl"
if [ ! -x "$LIMACTL" ]; then
  mkdir -p "$LDIR"
  curl -sL --max-time 300 -o "$LDIR/lima.tgz" \
    "https://github.com/lima-vm/lima/releases/download/v2.2.0/lima-2.2.0-Linux-x86_64.tar.gz" || exit 2
  tar --no-same-owner -xzf "$LDIR/lima.tgz" -C "$LDIR" || exit 2
fi
# two environment completions (see README, patches/):
# 1. LD_PRELOAD shim serves a virtual /etc/passwd (sandbox has none, root fs ro)
# 2. limactl built from source with the uid-0 guard disabled (patches/lima-v2.2.0-root-guard.diff)
export LD_PRELOAD="$VR_WORK/libfakepasswd.so"
LIMACTL="$LDIR/bin/limactl-patched"
"$LIMACTL" --version >> "$LOG" 2>&1 || exit 2

export LIMA_HOME="$VR_WORK/lima-home"
export PATH="$LDIR/bin:$PATH"
rm -rf "$LIMA_HOME/alpine-tcg" 2>/dev/null

# create from the stock alpine template (pinned by the release tag of lima itself)
echo "--- limactl create template://alpine" >> "$LOG"
timeout 600 "$LIMACTL" create --name alpine-tcg "$VR_WORK/alpine-3.23-template.yaml" 2>&1 | tail -5 >> "$LOG"
echo "rc(create)=$?" >> "$LOG"

echo "--- limactl start alpine-tcg (TCG boot; wall time measured)" >> "$LOG"
START=$(date +%s)
timeout 900 "$LIMACTL" start alpine-tcg >> "$LOG" 2>&1
RC=$?
END=$(date +%s)
echo "rc(start)=$RC wall=$((END-START))s" >> "$LOG"

echo "--- limactl shell uname" >> "$LOG"
timeout 120 "$LIMACTL" shell alpine-tcg uname -a >> "$LOG" 2>&1
echo "rc(shell)=$?" >> "$LOG"
grep -E 'rc\(start\)|rc\(shell\)|Linux ' "$LOG" | tail -4
grep -q 'rc(shell)=0' "$LOG" && exit 0 || exit 1
