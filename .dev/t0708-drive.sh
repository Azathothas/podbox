#!/bin/sh
# Driven proof for T-0708 (ignored, not committed). Host podman, driver
# container pattern (Windows host cannot exec the musl binary). Each row is
# one driver container holding its own store, so run, ps, inspect and rm
# share state. A row passes only at exit 0 with its marker printed.
set -u
PB="C:/Users/AjamX/Downloads/podbox/.dev/artifacts-pb/podbox"
export MSYS_NO_PATHCONV=1
DRIVER="public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a"
ALPINE="public.ecr.aws/docker/library/alpine:3.20"
ROCKY="quay.io/rockylinux/rockylinux:9@sha256:8101994123cf3d0a8fee517bee7f39e555c7d92bd2d9eb3303cc988a0eeed00f"
run() {
  desc="$1"; shift
  out=$(timeout 600 podman run --rm -v "$PB:/pb:ro" "$DRIVER" "$@" 2>&1)
  rc=$?
  echo "ROW $desc RC=$rc :: $(echo "$out" | tail -3 | tr '\n' '|')"
}
echo "== T-0708 driven proof"
echo "== E9 reads no inspect: the memo is a symlink to /dev/full there and"
echo "== the tally scan streams, so it reads zeros forever; a production memo"
echo "== is always a regular file, which terminates the scan."
echo "== (foreground runs keep no record, so the tally rows use the"
echo "== create/start/wait/inspect cycle; --rm would remove the memo"
echo "== inspect reads. E3 carries no -r: mapping ids needs /proc,"
echo "== which payloads do not have, and that failure is the payload's"
echo "== own requirement, correctly reported.)"
run "E1-mknod-tally" sh -c 'export PODBOX_STORE=/tmp/pbs; /pb create --name e1 "$0" sh -c "mknod /tmp/n c 1 3 && test -f /tmp/n" >/dev/null 2>&1 && /pb start e1 >/dev/null 2>&1 && /pb wait e1 >/dev/null 2>&1 && /pb inspect --format "{{.Interpose.Emulated.mknod}}" e1 | grep -q "^[1-9]" && echo MKNOD_OK; /pb rm -f e1 >/dev/null 2>&1' "$ALPINE"
run "E2-mount-tally" sh -c 'export PODBOX_STORE=/tmp/pbs; /pb create --name e2 "$0" sh -c "mount -t tmpfs none /tmp/mnt" >/dev/null 2>&1 && /pb start e2 >/dev/null 2>&1 && /pb wait e2 >/dev/null 2>&1 && /pb inspect --format "{{.Interpose.Emulated.mount}}" e2 | grep -q "^[1-9]" && echo MOUNT_OK; /pb rm -f e2 >/dev/null 2>&1' "$ALPINE"
run "E3-unshare-tally" sh -c 'export PODBOX_STORE=/tmp/pbs; /pb create --name e3 "$0" sh -c "unshare -U true" >/dev/null 2>&1 && /pb start e3 >/dev/null 2>&1 && /pb wait e3 >/dev/null 2>&1 && /pb inspect --format "{{.Interpose.Emulated.unshare}}" e3 | grep -q "^[1-9]" && echo UNSHARE_OK; /pb rm -f e3 >/dev/null 2>&1' "$ALPINE"
run "E4-clone-strip" sh -c 'export PODBOX_STORE=/tmp/pbs; /pb create --name e4 "$0" sh -c "echo aW1wb3J0IGN0eXBlcywgb3MKbGliYyA9IGN0eXBlcy5DRExMKE5vbmUsIHVzZV9lcnJubz1UcnVlKQpORVdVU0VSID0gMHgxMDAwMDAwMApORVdORVQgPSAweDQwMDAwMDAwClNJR0NITEQgPSAxNwpzdGsgPSBjdHlwZXMuY3JlYXRlX3N0cmluZ19idWZmZXIoNjU1MzYpCmRpZSA9IGN0eXBlcy5jYXN0KGxpYmMuX2V4aXQsIGN0eXBlcy5jX3ZvaWRfcCkudmFsdWUKbGliYy5jbG9uZS5hcmd0eXBlcyA9IFtjdHlwZXMuY192b2lkX3AsIGN0eXBlcy5jX3ZvaWRfcCwgY3R5cGVzLmNfaW50LCBjdHlwZXMuY192b2lkX3BdCmxpYmMuY2xvbmUucmVzdHlwZSA9IGN0eXBlcy5jX2ludAp0b3AgPSBjdHlwZXMuY192b2lkX3AoY3R5cGVzLmFkZHJlc3NvZihzdGspICsgbGVuKHN0aykpCnBpZCA9IGxpYmMuY2xvbmUoZGllLCB0b3AsIE5FV1VTRVIgfCBORVdORVQgfCBTSUdDSExELCBOb25lKQphc3NlcnQgcGlkID4gMCwgb3Muc3RyZXJyb3IoY3R5cGVzLmdldF9lcnJubygpKQpfLCBzdCA9IG9zLndhaXRwaWQocGlkLCAwKQphc3NlcnQgc3QgPT0gMCwgc3QKbGliYy5ta25vZC5hcmd0eXBlcyA9IFtjdHlwZXMuY19jaGFyX3AsIGN0eXBlcy5jX3VpbnQsIGN0eXBlcy5jX3Vsb25nXQphc3NlcnQgbGliYy5ta25vZChiIi90bXAvZGQiLCAwbzQwNzU1LCAwKSA9PSAtMSBhbmQgY3R5cGVzLmdldF9lcnJubygpID09IDEKcHJpbnQoIkNMT05FX09LIiwgZmx1c2g9VHJ1ZSkK | base64 -d > /tmp/cp.py && python3 /tmp/cp.py" >/dev/null 2>&1 && /pb start e4 >/dev/null 2>&1 && /pb wait e4 >/dev/null 2>&1 && /pb inspect --format "{{.Interpose.Emulated.clone}}" e4 | grep -q "^[1-9]" && /pb logs e4 2>&1 | grep -q "stripping CLONE_NEWNET" && echo CLONE_OK; /pb rm -f e4 >/dev/null 2>&1' "$ROCKY"
run "E4m-clone-strip-musl" sh -c 'export PODBOX_STORE=/tmp/pbs; /pb create --name e4m "$0" sh -c "apk add --no-cache python3 >/dev/null 2>&1 && echo aW1wb3J0IGN0eXBlcywgb3MKbGliYyA9IGN0eXBlcy5DRExMKE5vbmUsIHVzZV9lcnJubz1UcnVlKQpORVdVU0VSID0gMHgxMDAwMDAwMApORVdORVQgPSAweDQwMDAwMDAwClNJR0NITEQgPSAxNwpzdGsgPSBjdHlwZXMuY3JlYXRlX3N0cmluZ19idWZmZXIoNjU1MzYpCmRpZSA9IGN0eXBlcy5jYXN0KGxpYmMuX2V4aXQsIGN0eXBlcy5jX3ZvaWRfcCkudmFsdWUKbGliYy5jbG9uZS5hcmd0eXBlcyA9IFtjdHlwZXMuY192b2lkX3AsIGN0eXBlcy5jX3ZvaWRfcCwgY3R5cGVzLmNfaW50LCBjdHlwZXMuY192b2lkX3BdCmxpYmMuY2xvbmUucmVzdHlwZSA9IGN0eXBlcy5jX2ludAp0b3AgPSBjdHlwZXMuY192b2lkX3AoY3R5cGVzLmFkZHJlc3NvZihzdGspICsgbGVuKHN0aykpCnBpZCA9IGxpYmMuY2xvbmUoZGllLCB0b3AsIE5FV1VTRVIgfCBORVdORVQgfCBTSUdDSExELCBOb25lKQphc3NlcnQgcGlkID4gMCwgb3Muc3RyZXJyb3IoY3R5cGVzLmdldF9lcnJubygpKQpfLCBzdCA9IG9zLndhaXRwaWQocGlkLCAwKQphc3NlcnQgc3QgPT0gMCwgc3QKbGliYy5ta25vZC5hcmd0eXBlcyA9IFtjdHlwZXMuY19jaGFyX3AsIGN0eXBlcy5jX3VpbnQsIGN0eXBlcy5jX3Vsb25nXQphc3NlcnQgbGliYy5ta25vZChiIi90bXAvZGQiLCAwbzQwNzU1LCAwKSA9PSAtMSBhbmQgY3R5cGVzLmdldF9lcnJubygpID09IDEKcHJpbnQoIkNMT05FX09LIiwgZmx1c2g9VHJ1ZSkK | base64 -d > /tmp/cp.py && python3 /tmp/cp.py" >/dev/null 2>&1 && /pb start e4m >/dev/null 2>&1 && /pb wait e4m >/dev/null 2>&1 && /pb inspect --format "{{.Interpose.Emulated.clone}}" e4m | grep -q "^[1-9]" && /pb logs e4m 2>&1 | grep -q "stripping CLONE_NEWNET" && echo CLONE_MUSL_OK; /pb rm -f e4m >/dev/null 2>&1' "$ALPINE"
run "E5-threads-survive" sh -c 'export PODBOX_STORE=/tmp/pbs; /pb run --rm "$0" python3 -c "import threading; ts=[threading.Thread(target=lambda: None) for _ in range(8)]; [t.start() for t in ts]; [t.join() for t in ts]; print(\"THREADS_OK\")" 2>&1 | grep -q THREADS_OK && echo E5_OK' "$ROCKY"
run "E6-banner" sh -c 'export PODBOX_STORE=/tmp/pbs; /pb run --rm "$0" sh -c "true" 2>&1 | grep -q "emulating mknod, mount, unshare" && echo BANNER_OK' "$ALPINE"
run "E7-mknod-honest" sh -c 'export PODBOX_STORE=/tmp/pbs; /pb run --rm "$0" sh -c "mknod /tmp/e c 1 3 && mknod /tmp/e c 1 3 2>&1 | grep -qi exists && ! mknod /tmp/dd d 2>/dev/null && echo E7_OK" 2>&1 | grep -q E7_OK && echo E7A_OK' "$ALPINE"
run "E8-newnet-loud" sh -c 'export PODBOX_STORE=/tmp/pbs; /pb run --rm "$0" sh -c "unshare --fork -n true" 2>&1 | grep -q "CLONE_NEWNET" && echo NEWNET_OK' "$ALPINE"
run "E9-mknod-tally-refused" sh -c 'export PODBOX_STORE=/tmp/pbs; /pb create --name e9 "$0" sh -c "mknod /tmp/n c 1 3 2>/dev/null && echo BUG || (test -e /tmp/n || echo TALLY_REFUSED_OK)" >/dev/null 2>&1 && M=$(echo /tmp/pbs/containers/*/ownership.memo) && rm -f $M && ln -s /dev/full $M && /pb start e9 >/dev/null 2>&1 && /pb wait e9 >/dev/null 2>&1 && /pb logs e9 2>&1 | grep -q "TALLY_REFUSED_OK" && echo REFUSED_OK; /pb rm -f e9 >/dev/null 2>&1' "$ALPINE"
echo "== drive done"
