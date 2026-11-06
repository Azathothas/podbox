#!/bin/sh
# Scratch repro for T-1309 (ignored, not committed). Host podman only.
# Leg 1 (control): dnf makecache straight in rocky:9.
# Leg 2 (podbox): the same command under /pb run with the interposer.
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
# winpath spelling: Git Bash hands /c/... to a Windows binary as garbage,
# so the drive root is spelled C:/... and MSYS conversion stays off.
PB="C:/Users/AjamX/Downloads/podbox/.dev/podbox-interpose-bin"
export MSYS_NO_PATHCONV=1
[ -r "$PB" ] || { echo "no staged binary"; exit 2; }
podman pull "$ROCKY" >/dev/null 2>&1 || { echo "pull failed"; exit 2; }

echo "== leg 1: control, dnf makecache in rocky directly"
timeout 600 podman run --rm "$ROCKY" dnf makecache 2>&1 | tail -5
echo "LEG1_RC=$?"

echo "== leg 2: the same under podbox with the interposer"
timeout 600 podman run --rm \
  -v "$PB:/pb:ro" \
  -e PODBOX_STORE=/tmp/pbstore \
  "$ROCKY" /pb run --rm "$ROCKY" dnf makecache 2>&1 | tail -8
echo "LEG2_RC=$?"
