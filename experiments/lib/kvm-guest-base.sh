#!/bin/sh
# T-1112 KVM guest run, executed in the wsl-toolkit base (the KVM host).
set -u

: "${PODBOX_BIN_SRC:?the host driver supplies the binary}"
KVM=$(mktemp -d "$HOME/podbox-kvm.XXXXXXXX") || exit 2
cleanup() {
  stop_owned_emulators
  rm -rf "${KVM:?}"
}
trap cleanup EXIT HUP INT TERM
PODBOX="$KVM/podbox"
: "${IMG_HOST:?the host driver supplies the image}"
VHDX_LEN=910163968
VHDX_SHA256="063442aa9f71f2faeebf49cd960003ce315abd556b52c696e1b994ec5a80fe7f"
CODE="/usr/share/edk2-ovmf/x64/OVMF_CODE.4m.fd"
VARS="/usr/share/edk2-ovmf/x64/OVMF_VARS.4m.fd"

fail=0
say() { printf '%s\n' "$*"; }
miss() { fail=1; say "FAIL: $1"; }
ok() { say "ok: $1"; }

# Reserve the image length twice for the copy and provision, plus 16 inodes.
available=$(df -Pk "$KVM" | awk 'END {print $4 * 1024}')
inodes=$(df -Pi "$KVM" | awk 'END {print $4}')
if ! awk "BEGIN {exit !($available >= 2 * $VHDX_LEN && $inodes >= 16)}"; then
  say "cannot run: insufficient free blocks or inodes for image and provision"
  exit 2
fi

# The 2026-09-30 runs left a 4 GiB KVM emulator that SIGKILL did not remove,
# and the Windows host then failed. The base enforces no memory limit, so
# this driver refuses to add a guest beside another emulator or into memory
# that cannot hold the guest and a 2 GiB margin.
if ps -eo args | awk '$1 ~ /(^|\/)qemu-system-/ {found=1} END {exit !found}'; then
  say "cannot run: an emulator already runs in the base; stop it first"
  ps -eo pid,stat,args | awk '$3 ~ /(^|\/)qemu-system-/'
  exit 2
fi
avail_mib=$(awk '/^MemAvailable:/ {print int($2 / 1024)}' /proc/meminfo)
if [ "${avail_mib:-0}" -lt 6144 ]; then
  say "cannot run: ${avail_mib:-unknown} MiB available; the guest needs 4096 MiB plus a 2048 MiB margin"
  exit 2
fi

mkdir -p "$KVM" || { miss "cannot create $KVM"; printf 'verdict KVM-GUEST-FAIL\n'; exit 1; }
cp "$PODBOX_BIN_SRC" "$PODBOX" && chmod +x "$PODBOX" \
  || { miss "cannot stage the podbox binary"; printf 'verdict KVM-GUEST-FAIL\n'; exit 1; }
[ -x "$PODBOX" ] || { miss "staged binary not executable"; printf 'verdict KVM-GUEST-FAIL\n'; exit 1; }
export PODBOX_STORE="$KVM/store"
export XDG_RUNTIME_DIR="$KVM/runtime"
export PODBOX_OVMF_CODE="$CODE" PODBOX_OVMF_VARS="$VARS"
mkdir -m 700 -p "$PODBOX_STORE" "$XDG_RUNTIME_DIR" || exit 1

say "== conditions"
date -u +%Y-%m-%dT%H:%M:%SZ
uname -sr
qemu-system-x86_64 --version | head -n 1
qemu-system-x86_64 -accel help 2>/dev/null | tr '\n' ' '
echo
"$PODBOX" --version 2>/dev/null || "$PODBOX" version 2>/dev/null || echo "podbox binary answers"
sha256sum "$PODBOX" | awk '{print "binary SHA256: " $1}'
free -m | head -n 2
ls -l /dev/kvm /dev/net/tun
echo

say "== image"
if [ -f "$IMG_HOST" ]; then
  say "host image bytes: $(stat -c %s "$IMG_HOST" 2>/dev/null || stat -f %z "$IMG_HOST")"
else
  miss "host image absent at $IMG_HOST"
fi
if [ "$fail" -eq 0 ]; then
  cp "$IMG_HOST" "$KVM/base.vhdx" || miss "image copy failed"
  got=$(stat -c %s "$KVM/base.vhdx" 2>/dev/null || stat -f %z "$KVM/base.vhdx")
  [ "$got" = "$VHDX_LEN" ] || miss "image length $got, not $VHDX_LEN"
  sum=$(sha256sum "$KVM/base.vhdx" 2>/dev/null | cut -d' ' -f1)
  [ "$sum" = "$VHDX_SHA256" ] || miss "image digest mismatch"
  [ "$fail" -eq 0 ] && ok "installed disk copied with the pinned digest"
fi

say "== doctor"
if [ "$fail" -eq 0 ]; then
  if timeout 120 "$PODBOX" windows doctor >"$KVM/doctor.txt" 2>&1; then
    cat "$KVM/doctor.txt"
    if grep -qi "accelerator: kvm" "$KVM/doctor.txt"; then
      ok "doctor names the kvm accelerator"
    else
      miss "doctor does not name kvm"
    fi
  else
    miss "doctor refused"; cat "$KVM/doctor.txt"
  fi
fi

say "== setup"
PROV="$KVM/base.podbox.qcow2"
# A re-drive provisions again from the verified base rather than
# reusing the last run's image: the provision is part of the proof.
rm -f "$PROV"
if [ "$fail" -eq 0 ]; then
  # Provision waits once before typing and once for shutdown. The outer
  # bound includes both waits and the QMP connection deadline.
  if timeout 1500 "$PODBOX" windows setup --image "$KVM/base.vhdx" --podbox-timeout 600 >"$KVM/setup.txt" 2>&1; then
    tail -n 4 "$KVM/setup.txt"
    [ -f "$PROV" ] && ok "provisioned image beside the base" || miss "provisioned image absent"
  else
    setup_rc=$?
    miss "setup failed with exit $setup_rc"; tail -n 10 "$KVM/setup.txt"
  fi
fi

say "== run ver with the emulator command line watched"
if [ "$fail" -eq 0 ]; then
  (while :; do
    for d in "$XDG_RUNTIME_DIR"/podbox-windows-*-run; do
      [ -f "$d/serial.log" ] && cp "$d/serial.log" "$KVM/ver-serial.log" 2>/dev/null
    done
    sleep 2
  done) &
  watchpid=$!
  timeout 600 "$PODBOX" windows run --image "$PROV" --podbox-timeout 540 -- ver >"$KVM/ver.txt" 2>"$KVM/ver.err" &
  runpid=$!
  qemu_cmd=""
  waited=0
  while [ "$waited" -lt 540 ]; do
    if ! kill -0 "$runpid" 2>/dev/null; then break; fi
    qemu_cmd=$(ps -ef 2>/dev/null | grep "[q]emu-system-x86_64" | grep -F "$KVM" | head -n 1)
    [ -n "$qemu_cmd" ] && break
    sleep 2
    waited=$((waited + 2))
  done
  wait "$runpid"
  vcode=$?
  kill "$watchpid" 2>/dev/null || true
  wait "$watchpid" 2>/dev/null || true
  say "run exit: $vcode"
  # The transcript stays reviewable: cmd.exe writes CRLF and a raw byte
  # would make this file invisible to grep, so the asserts above read
  # the raw capture and only the printed copy is scrubbed.
  tr -d '\r' <"$KVM/ver.txt"
  say "emulator: $qemu_cmd"
  if [ "$vcode" -eq 0 ] && grep -q "Microsoft Windows" "$KVM/ver.txt"; then
    ok "guest version string with exit 0"
  else
    miss "ver run failed"; tail -n 5 "$KVM/ver.err"
    [ ! -f "$KVM/ver-serial.log" ] || tr -d '\r' <"$KVM/ver-serial.log"
  fi
  case "$qemu_cmd" in
    *"-accel kvm"*"-cpu host"*) ok "outside observer: the guest ran under -accel kvm -cpu host" ;;
    *) miss "emulator command line does not show kvm" ;;
  esac
fi

say "== run exit 42"
if [ "$fail" -eq 0 ]; then
  # A serial watcher runs beside the guest: one 540 s hang on
  # 2026-09-29 never reproduced, and the driver's scratch cleanup
  # takes the serial log with it, so the watch copy survives to say
  # where a future hang stops.
  (n=0; while [ "$n" -lt 55 ]; do sleep 10; n=$((n + 1));
    for d in "$XDG_RUNTIME_DIR"/podbox-windows-*-run; do
      [ -f "$d/serial.log" ] && cp "$d/serial.log" "$KVM/serial-watch.log" 2>/dev/null
    done; done) &
  watchpid=$!
  timeout 600 "$PODBOX" windows run --image "$PROV" --podbox-timeout 540 -- ver '>nul' '&' cmd /c exit 42 >"$KVM/e42.txt" 2>"$KVM/e42.err"
  ecode=$?
  kill "$watchpid" 2>/dev/null || true
  wait "$watchpid" 2>/dev/null || true
  say "run exit: $ecode"
  if [ "$ecode" -eq 42 ]; then
    ok "the guest's own exit 42 passed through"
  else
    miss "exit 42 failed with $ecode"; tail -n 5 "$KVM/e42.err"
    [ ! -f "$KVM/serial-watch.log" ] || tr -d '\r' <"$KVM/serial-watch.log"
  fi
fi

say "== the podbox run seam"
if [ "$fail" -eq 0 ]; then
  if timeout 600 "$PODBOX" run --podbox-tier=machine --platform windows/amd64 "$PROV" cmd /c ver >"$KVM/seam.txt" 2>"$KVM/seam.err"; then
    scode=$?
    tr -d '\r' <"$KVM/seam.txt"
    if grep -q "Microsoft Windows" "$KVM/seam.txt"; then
      ok "podbox run reached the disk guest with exit $scode"
    else
      miss "seam run missed the version string"
    fi
  else
    miss "podbox run seam failed"; tail -n 5 "$KVM/seam.err"
  fi
fi

say "== residue"
left=$(ls -d "$XDG_RUNTIME_DIR"/podbox-windows-*-run 2>/dev/null || true)
if [ -z "$left" ]; then
  ok "no per-run scratch directory remains"
else
  miss "scratch residue: $left"
fi
if [ -n "$(owned_emulators)" ]; then
  miss "owned emulator remains after the command"
  stop_owned_emulators
fi
[ -z "$(owned_emulators)" ] && ok "no owned emulator remains" || miss "owned emulator still exists"
ls "$KVM"
rm -rf "${KVM:?}" || miss "cannot remove owned scratch directory"
[ ! -e "$KVM" ] && ok "owned scratch directory removed" || miss "owned scratch still exists"
echo
if [ "$fail" -eq 0 ]; then
  echo "verdict KVM-GUEST-OK: ValidationOS under kvm returns its version and its exit codes"
else
  echo "verdict KVM-GUEST-FAIL"
fi
exit "$fail"
