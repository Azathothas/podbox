#!/bin/sh
# Driven proof for T-0705 (ignored, not committed). Host podman, driver
# container pattern (Windows host cannot exec the musl binary). Each row
# prints its exit code; a row passes only at exit 0.
set -u
PB="C:/Users/AjamX/Downloads/podbox/.dev/artifacts-pb/podbox"
export MSYS_NO_PATHCONV=1
DRIVER="public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a"
ALPINE="public.ecr.aws/docker/library/alpine:3.20"
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
MAPS="/mapped:/etc"
run() {
  desc="$1"; shift
  out=$(timeout 300 podman run --rm -v "$PB:/pb:ro" -e PODBOX_STORE=/tmp/pbstore "$DRIVER" "$@" 2>&1)
  rc=$?
  echo "ROW $desc RC=$rc :: $(echo "$out" | tail -2 | tr '\n' '|')"
}
echo "== T-0705 driven proof, maps $MAPS"
run "pwd-virtual" /pb run --rm -e "PODBOX_MAPS=$MAPS" "$ALPINE" sh -c 'cd /mapped && test "$(pwd)" = /mapped'
run "forward-active" /pb run --rm -e "PODBOX_MAPS=$MAPS" "$ALPINE" sh -c 'test -f /mapped/os-release && head -1 /mapped/os-release'
run "realpath-null" /pb run --rm -e "PODBOX_MAPS=$MAPS" "$ALPINE" sh -c 'test "$(realpath /mapped/os-release)" = /mapped/os-release'
run "readlink-passthru" /pb run --rm -e "PODBOX_MAPS=$MAPS" "$ALPINE" sh -c 'readlink /mapped/os-release; test $? -ne 0'
run "py-getcwd" /pb run --rm -e "PODBOX_MAPS=$MAPS" "$ROCKY" python3 -c 'import os; os.chdir("/mapped"); assert os.getcwd() == "/mapped", os.getcwd(); print("getcwd ok")'
run "py-realpath" /pb run --rm -e "PODBOX_MAPS=$MAPS" "$ROCKY" python3 -c 'import os; assert os.path.realpath("/mapped/hosts") == "/mapped/hosts"; print("realpath ok")'
run "py-erange" /pb run --rm -e "PODBOX_MAPS=$MAPS" "$ROCKY" python3 -c '
import ctypes, errno, os
os.chdir("/mapped")
libc = ctypes.CDLL(None, use_errno=True)
libc.getcwd.restype = ctypes.c_void_p
b6 = ctypes.create_string_buffer(6)
assert libc.getcwd(b6, 6) is None and ctypes.get_errno() == errno.ERANGE, (b6.value, ctypes.get_errno())
b64 = ctypes.create_string_buffer(64)
assert libc.getcwd(b64, 64) is not None and b64.value == b"/mapped", b64.value
print("erange ok")'
run "py-gcdn" /pb run --rm -e "PODBOX_MAPS=$MAPS" "$ROCKY" python3 -c '
import ctypes, os
os.chdir("/mapped")
libc = ctypes.CDLL(None, use_errno=True)
libc.get_current_dir_name.restype = ctypes.c_void_p
p = libc.get_current_dir_name()
assert p is not None and ctypes.string_at(p) == b"/mapped", p
print("gcdn ok")'
echo "== drive done"
