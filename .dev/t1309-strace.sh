#!/bin/sh
# Scratch strace for T-1309 (ignored). Outer rocky gets strace via control
# dnf, then traces the RED (sh-parent) and GREEN (direct) shapes. Logs land
# on /w (host .dev/t1309-w).
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
PB="C:/Users/AjamX/Downloads/podbox/.dev/podbox-interpose-bin"
W="C:/Users/AjamX/Downloads/podbox/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
mkdir -p "$W"
DNF="dnf install -y --setopt=install_weak_deps=False gcc glibc-devel"
TR="open,openat,openat2,stat,lstat,statx,fstatat,rename,renameat,renameat2,unlink,unlinkat,mkstemp,mkostemp,execve,access,faccessat,chmod,fchmodat,utimensat"
echo "== outer setup: strace into a keeper image"
timeout 600 podman run --name t1309base "$ROCKY" dnf install -y strace >/dev/null 2>&1 || {
	podman rm -f t1309base >/dev/null 2>&1
	timeout 600 podman run --name t1309base "$ROCKY" dnf install -y strace >/dev/null 2>&1 || exit 2
}
timeout 60 podman commit t1309base t1309strace >/dev/null 2>&1
podman rm -f t1309base >/dev/null 2>&1
echo "== RED trace: sh-parent"
timeout 1200 podman run --rm -v "$PB:/pb:ro" -v "$W:/w:rw" -e PODBOX_STORE=/tmp/pbstore \
	t1309strace strace -f -e trace="$TR" -o /w/red.tr \
	/pb run --rm "$ROCKY" /bin/sh -c "$DNF" >"$W/red.out" 2>&1
echo "RED_RC=$?"
grep -E "Complete!|Error|Invalid" "$W/red.out" | head -2
echo "== GREEN trace: direct"
timeout 1200 podman run --rm -v "$PB:/pb:ro" -v "$W:/w:rw" -e PODBOX_STORE=/tmp/pbstore \
	t1309strace strace -f -e trace="$TR" -o /w/green.tr \
	/pb run --rm "$ROCKY" $DNF >"$W/green.out" 2>&1
echo "GREEN_RC=$?"
grep -E "Complete!|Error|Invalid" "$W/green.out" | head -2
ls -la "$W"/*.tr
