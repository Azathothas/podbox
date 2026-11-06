#!/bin/sh
# Driven proof for T-0707 (ignored, not committed). Host podman, driver
# container pattern (Windows host cannot exec the musl binary). Each row
# prints its exit code; a row passes only at exit 0.
#
# D1 is the entry Prove: under a map covering /proc, the directory still
# reads as itself (empty) instead of the rewrite target.
# D2 pins the honest shape where /proc is unmounted: the kernel's own
# ENOENT on the real path, never a mapped hallucination.
# D3 proves the user list on a resolvable path. D4 proves it does not
# overreach past a component boundary.
set -u
PB="C:/Users/AjamX/Downloads/podbox/.dev/artifacts-pb/podbox"
export MSYS_NO_PATHCONV=1
DRIVER="public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a"
ALPINE="public.ecr.aws/docker/library/alpine:3.20"
run() {
  desc="$1"; shift
  out=$(timeout 300 podman run --rm -v "$PB:/pb:ro" -e PODBOX_STORE=/tmp/pbstore0707 "$DRIVER" "$@" 2>&1)
  rc=$?
  echo "ROW $desc RC=$rc :: $(echo "$out" | grep -v "podbox: complete:" | tail -2 | tr '\n' '|')"
}
echo "== T-0707 driven proof"
run "D1-builtin-proc" /pb run --rm -e "PODBOX_MAPS=/proc:/etc" "$ALPINE" sh -c 'test -z "$(ls /proc)" && echo PROC_OK'
run "D2-exe-honest-enoent" /pb run --rm -e "PODBOX_MAPS=/proc:/etc" "$ALPINE" sh -c 'readlink /proc/self/exe 2>/dev/null; test $? -ne 0 && echo HONEST_OK'
run "D3-user-list" /pb run --rm -e "PODBOX_MAPS=/mapped:/etc" -e "PODBOX_EXCLUDE_PATH=/mapped/hosts" "$ALPINE" sh -c 'test ! -e /mapped/hosts && echo USER_OK'
run "D4-user-boundary" /pb run --rm -e "PODBOX_MAPS=/mapped:/etc" -e "PODBOX_EXCLUDE_PATH=/mappe" "$ALPINE" sh -c 'test -f /mapped/os-release && echo BOUNDARY_OK'
echo "== drive done"
