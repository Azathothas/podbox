#!/bin/sh
# Scratch consistency probe for T-1309 (ignored). Two exact 240 rocky rows
# plus one fixed-URL install under the interposer.
set -u
export MSYS_NO_PATHCONV=1
export PODBOX_BIN="C:/Users/AjamX/Downloads/podbox/.dev/podbox-interpose-bin"
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
PB="$PODBOX_BIN"
echo "== row repeat 1"
sh experiments/240-distro-sweep.sh rocky 2>&1 | grep -E "^rocky |INSTALL_RC|no-compiler|42" | head -5
echo "== row repeat 2"
sh experiments/240-distro-sweep.sh rocky 2>&1 | grep -E "^rocky |INSTALL_RC|no-compiler|42" | head -5
echo "== fixed-URL install under preload"
timeout 600 podman run --rm -v "$PB:/pb:ro" -e PODBOX_STORE=/tmp/pbstore "$ROCKY" \
	/pb run --rm "$ROCKY" dnf install -y --setopt=install_weak_deps=False \
	--disablerepo='*' --enablerepo=baseos \
	--setopt=baseos.baseurl=https://dl.rockylinux.org/pub/rocky/9/BaseOS/x86_64/os/ \
	--setopt=baseos.metalink= gcc 2>&1 | grep -E "Complete|Error|Invalid|Failed" | head -4
echo "FIXED_RC=$?"
