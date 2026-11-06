#!/bin/sh
# Scratch bisect for T-1309 (ignored): shell parent vs file redirect vs probes.
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
PB="C:/Users/AjamX/Downloads/podbox/.dev/podbox-interpose-bin"
export MSYS_NO_PATHCONV=1
DNF="dnf install -y --setopt=install_weak_deps=False gcc glibc-devel"
leg() {
	echo "== $1"
	timeout 900 podman run --rm -v "$PB:/pb:ro" -e PODBOX_STORE=/tmp/pbstore \
		"$ROCKY" /bin/sh -c "/pb run --rm \"$ROCKY\" /bin/sh -c '$2'" 2>&1 | \
		grep -E "Complete!|Error|Invalid|Failed" | head -3
	echo "RC=$?"
}
leg "S1 shell-parent plus file" "$DNF >/tmp/x.log 2>&1; tail -2 /tmp/x.log"
leg "S2 exec direct-child plus file" "exec $DNF >/tmp/x.log 2>&1"
leg "S3 shell-parent plus pipe" "$DNF"
