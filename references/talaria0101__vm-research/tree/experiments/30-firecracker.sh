#!/usr/bin/env bash
# Question: firecracker-microvm/firecracker v1.16.1 — the released binary
# starts and serves its API here, but where exactly does a microVM launch die:
# at open("/dev/kvm"), at a seccomp-filtered syscall, or somewhere earlier?
# The decisive evidence is the API error for PUT /actions InstanceStart (and
# the firecracker log line around it).
#
# Exit codes: 0 VM started (not expected here), 1 VM launch blocked, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/30-firecracker.log"
conditions firecracker > "$LOG"

FC_DIR="$VR_WORK/firecracker-1.16.1"
FC="$FC_DIR/firecracker-v1.16.1-x86_64"
if [ ! -x "$FC" ]; then
  mkdir -p "$FC_DIR"
  curl -sL --max-time 300 -o "$FC_DIR/fc.tgz" \
    "https://github.com/firecracker-microvm/firecracker/releases/download/v1.16.1/firecracker-v1.16.1-x86_64.tgz" || exit 2
  tar --no-same-owner -xzf "$FC_DIR/fc.tgz" -C "$FC_DIR" || exit 2
fi
FC=$(find "$FC_DIR" -name 'firecracker-v1.16.1-x86_64' -type f | head -1)
[ -x "$FC" ] || exit 2
echo "binary: $FC" >> "$LOG"
"$FC" --version >> "$LOG" 2>&1

# Use the pinned Alpine kernel from lib.sh as boot-source payload — it never
# gets to load, but the request is realistic.
vfetch vmlinuz-virt "$PIN_VMLINUZ_VIRT_URL" "$PIN_VMLINUZ_VIRT_SHA" || exit 2

rm -f /tmp/fc.sock /tmp/fc.log
"$FC" --api-sock /tmp/fc.sock >> "$LOG" 2>&1 &
FCPID=$!
for i in $(seq 1 100); do [ -S /tmp/fc.sock ] && break; sleep 0.1; done
[ -S /tmp/fc.sock ] || { echo "no api socket" >> "$LOG"; kill $FCPID 2>/dev/null; exit 1; }

api() { curl -s --max-time 10 --unix-socket /tmp/fc.sock -X PUT "http://localhost/$1" -H 'Accept: application/json' -d "$2"; echo; }
{
  echo "--- PUT /logger";  api logger   '{"log_path":"/tmp/fc.log","level":"Debug","show_level":true,"show_log_origin":true}'
  echo "--- PUT /boot-source"; api boot-source "{\"kernel_image_path\":\"$VR_WORK/vmlinuz-virt\",\"boot_args\":\"console=ttyS0 reboot=k panic=1\"}"
  echo "--- PUT /machine-config"; api machine-config '{"vcpu_count":2,"mem_size_mib":512}'
  echo "--- PUT /actions InstanceStart"; api actions '{"action_type":"InstanceStart"}'
} >> "$LOG" 2>&1

sleep 1
echo "--- firecracker.log tail (its own account)" >> "$LOG"
tail -6 /tmp/fc.log >> "$LOG" 2>&1
kill $FCPID 2>/dev/null; wait $FCPID 2>/dev/null
echo "--- decision:" >> "$LOG"
grep -oE '"fault_message": "[^"]*"' "$LOG" | tail -2 >> "$LOG"
if grep -q 'InstanceStart' "$LOG" && grep -qiE 'fault_message|Error' "$LOG"; then
  tail -20 "$LOG"; exit 1
fi
exit 1
