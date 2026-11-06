#!/bin/sh
# Scratch bisect + disassembly for T-1309 (ignored, not committed).
# 1. bare: install binutils, dump realpath disassembly to /w.
# 2. build S3 (dlerror-before) and S4 (fprintf-before), makecache each.
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
W="C:/Users/AjamX/Downloads/podbox/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
echo "== disassembly of rocky realpath (bare)"
timeout 900 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "command -v objdump >/dev/null || dnf install -y -q binutils 2>&1 | tail -1
   objdump -d --disassemble=realpath /lib64/libc.so.6 > /w/realpath.asm 2>&1 && echo ASM_OK && wc -l /w/realpath.asm" \
  >"$W/v8asm.out" 2>&1
echo "ASM_OUTER_RC=$?"
cat "$W/v8asm.out"
echo "== build S3 S4, run each"
timeout 1200 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "command -v gcc >/dev/null || dnf install -y -q gcc 2>&1 | tail -1
   gcc -shared -fPIC -o /w/s3.so /w/s3shim.c && gcc -shared -fPIC -o /w/s4.so /w/s4shim.c && echo S34_BUILD_OK
   echo '-- S3 (dlerror-before):'; LD_PRELOAD=/w/s3.so dnf makecache >/tmp/s3.log 2>&1; echo S3_RC=\$?
   grep -E 'Metadata|Error|Invalid' /tmp/s3.log | head -2; grep -h 's3(' /tmp/s3.log | head -3
   echo '-- S4 (fprintf-before):'; LD_PRELOAD=/w/s4.so dnf makecache >/tmp/s4.log 2>&1; echo S4_RC=\$?
   grep -E 'Metadata|Error|Invalid' /tmp/s4.log | head -2; grep -h 's4(' /tmp/s4.log | head -3" \
  >"$W/v8bis.out" 2>&1
echo "BIS_OUTER_RC=$?"
cat "$W/v8bis.out"
