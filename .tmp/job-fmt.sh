#!/bin/sh
# Format the tree with the lane's stable rustfmt and report the diff.
set -u
OUT="${PODBOX_ARTIFACTS:-/tmp/artifacts-fmt}"
mkdir -p "$OUT"
cargo fmt --all
echo "fmt exits 0"
git -C /work status --short >"$OUT/fmt-files.txt" 2>&1
git -C /work diff --stat >"$OUT/fmt-stat.txt" 2>&1
cat "$OUT/fmt-stat.txt"
echo "verdict FMT-DONE"
