#!/bin/sh
# Scratch matrix for T-1309 (ignored, not committed). Fixed URL from the
# entry premise vs default mirrors, preload vs env -u, fresh store per run.
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
PB="C:/Users/AjamX/Downloads/podbox/.dev/podbox-interpose-bin"
export MSYS_NO_PATHCONV=1
FIX="--disablerepo=* --enablerepo=baseos --setopt=baseos.baseurl=https://dl.rockylinux.org/pub/rocky/9/BaseOS/x86_64/os/ --setopt=baseos.metalink="
N=0
run() {
	N=$((N + 1))
	echo "== run $N: $1"
	timeout 600 podman run --rm -v "$PB:/pb:ro" -e PODBOX_STORE=/tmp/pbstore \
		"$ROCKY" /bin/sh -c "$2" 2>&1 | grep -E "Metadata|Error|Invalid|Failed" | head -4
	echo "RC=$?"
}
run "preload fixed-url" "/pb run --rm $ROCKY dnf makecache $FIX"
run "preload fixed-url again" "/pb run --rm $ROCKY dnf makecache $FIX"
run "no-preload fixed-url" "/pb run --rm $ROCKY env -u LD_PRELOAD dnf makecache $FIX"
run "preload mirrors" "/pb run --rm $ROCKY dnf makecache"
echo "== matrix done"
