#!/bin/sh
# T-1309 compat audit input (ignored, not committed). Bare container dumps
# the dynamic symbol table of the payload libc for host-side analysis.
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
W="C:/Users/AjamX/Downloads/podbox/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
echo "== dump payload libc dynsym"
timeout 600 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "command -v objdump >/dev/null || dnf install -y -q binutils 2>&1 | tail -1
   objdump -T /lib64/libc.so.6 > /w/libc.dynsym 2>&1 && echo DYNSYM_OK && wc -l /w/libc.dynsym" \
  >"$W/v13audit.out" 2>&1
echo "OUTER_RC=$?"
cat "$W/v13audit.out"
