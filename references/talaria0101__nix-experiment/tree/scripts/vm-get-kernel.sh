#!/usr/bin/env bash
# Fetch Alpine linux-virt kernel + busybox-static for the nix-experiment VM.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p vm/dl vm/root
REL="${ALPINE_REL:-v3.22}"
MIRROR="https://dl-cdn.alpinelinux.org/alpine"
cd vm/dl

apk_url() { curl -s "$MIRROR/$REL/main/x86_64/" | grep -oE "href=\"$1-[^\"]*\.apk\"" | head -1 | sed 's/href="//;s/"//'; }

KB=$(apk_url linux-virt); BB=$(apk_url busybox-static)
echo "kernel apk: $KB"; echo "busybox-static apk: $BB"
[ -f "$KB" ] || curl -sLO "$MIRROR/$REL/main/x86_64/$KB"
[ -f "$BB" ] || curl -sLO "$MIRROR/$REL/main/x86_64/$BB"

cd ../root
tar xzf "../dl/$KB" boot/vmlinuz-virt 2>/dev/null || tar xzf "../dl/$KB"
tar xzf "../dl/$BB" 2>/dev/null || true
find . -name "vmlinuz-virt" -o -name "busybox.static" | head
