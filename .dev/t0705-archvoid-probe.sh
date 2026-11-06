#!/bin/sh
# Full-stderr interpose probe for the two 240 rows whose transcripts hide
# the interpose line past `tail -5` (T-0705 evidence; ignored, not committed).
# Host podman, driver-container pattern: Windows cannot exec the musl binary.
set -u
PB="C:/Users/AjamX/Downloads/podbox/.dev/artifacts-pb/podbox"
export MSYS_NO_PATHCONV=1
DRIVER="public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a"
ARCH="ghcr.io/pkgforge-dev/archlinux@sha256:b2507f1964270cab3cc190aa8df858521f6a7e45e969146a012877891d5dfc9b"
VOID="ghcr.io/void-linux/void-musl@sha256:af9308c79fd14ddc7b53e181caee312d182c89186792dc1ef5b8215865b444ac"
probe() {
  name="$1"; img="$2"
  out=$(timeout 600 podman run --rm -v "$PB:/pb:ro" -e PODBOX_STORE=/tmp/pbstore-av "$DRIVER" /pb run --rm "$img" /bin/sh -c 'echo PAYLOAD_OK' 2>&1)
  rc=$?
  echo "ROW $name RC=$rc payload=$(printf '%s\n' "$out" | grep -c PAYLOAD_OK) interpose=$(printf '%s\n' "$out" | grep -c "interpose") declined=$(printf '%s\n' "$out" | grep -c "declined") preloaded=$(printf '%s\n' "$out" | grep -c "preloaded")"
  printf '%s\n' "$out" | grep "interpose" | cut -c1-160
}
echo "== arch/void full-stderr interpose probe"
probe archlinux "$ARCH"
probe voidlinux-musl "$VOID"
echo "== probe done"
