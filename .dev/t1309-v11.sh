#!/bin/sh
# T-1309 version-dispatch check (ignored, not committed). Bare container:
# 1. which realpath version librepo/libdnf reference,
# 2. full libc disassembly to /w for host-side comparison of both bodies,
# 3. bare fixed-URL makecache from a cold cache with the resulting file
#    list (proves the loop runs and succeeds bare).
set -u
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
W="C:/Users/AjamX/Downloads/podbox/.dev/t1309-w"
export MSYS_NO_PATHCONV=1
FIX="--disablerepo=* --enablerepo=baseos --setopt=baseos.baseurl=https://dl.rockylinux.org/pub/rocky/9/BaseOS/x86_64/os/ --setopt=baseos.metalink="
echo "== version refs + libc asm + bare cold-cache proof"
timeout 1200 podman run --rm -v "$W:/w:rw" "$ROCKY" /bin/sh -c \
  "command -v objdump >/dev/null || dnf install -y -q binutils 2>&1 | tail -1
   echo '-- librepo realpath ref:'; objdump -T /usr/lib64/librepo.so.0 | grep -i realpath
   echo '-- libdnf realpath ref:'; objdump -T /usr/lib64/libdnf.so.2 | grep -i realpath
   echo '-- libpython realpath refs:'; objdump -T /lib64/libpython3.9.so.1.0 | grep -i realpath
   objdump -d /lib64/libc.so.6 > /w/libc.asm 2>&1 && echo LIBC_ASM_OK && wc -c /w/libc.asm
   echo '-- bare cold-cache makecache:'; rm -rf /var/cache/dnf
   dnf makecache $FIX >/tmp/bare.log 2>&1; echo BARE_RC=\$?
   grep -E 'Metadata|Error|Invalid' /tmp/bare.log | head -2
   echo '-- cache files:'; find /var/cache/dnf -name '*primary*' | head -5" \
  >"$W/v11ver.out" 2>&1
echo "OUTER_RC=$?"
cat "$W/v11ver.out"
