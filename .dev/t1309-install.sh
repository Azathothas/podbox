#!/bin/sh
# Scratch install-repeat for T-1309 (ignored). Two dnf installs under the
# interposer; the second verbose with its log kept on /w.
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
PB="C:/Users/AjamX/Downloads/podbox/.dev/podbox-interpose-bin"
W="$PWD/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
mkdir -p "$W"
echo "== run A: repeat of the row install"
timeout 1200 podman run --rm -v "$PB:/pb:ro" -e PODBOX_STORE=/tmp/pbstore \
	"$ROCKY" /pb run --rm "$ROCKY" dnf install -y --setopt=install_weak_deps=False gcc glibc-devel >"$W/install-a.log" 2>&1
echo "A_RC=$?"; tail -4 "$W/install-a.log"
echo "== run B: verbose, log kept"
timeout 1200 podman run --rm -v "$PB:/pb:ro" -e PODBOX_STORE=/tmp/pbstore \
	"$ROCKY" /pb run --rm "$ROCKY" dnf -v install -y --setopt=install_weak_deps=False gcc glibc-devel >"$W/install-b.log" 2>&1
echo "B_RC=$?"; tail -4 "$W/install-b.log"
