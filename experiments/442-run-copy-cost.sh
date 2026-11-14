#!/bin/sh
# Question: how much of a `podvm run --rm IMAGE true` is the fill of the
# container's own rootfs, and does it grow with the image?
#
# TODO/enter.md T-1749 (V-5 of PodUtils/podcrates#124): a read-only image
# under a tmpfs upper layer would need no copy. The entry measures the
# copy first. Each run fills a new tree from the image (T-1709): FICLONE
# where the filesystem shares blocks, else a copy of the bytes. With
# --verbose the banner names the files, the size, the form and the time of
# the fill.
#
# Inputs: the podvm binary (PODVM_BIN, else the release binary of this
# tree), and three images of growing size (PODVM_442_MIRROR=1 takes them
# from mirror.gcr.io): busybox:musl, ubuntu:24.04 and golang:1.24.7-bookworm.
# Each image is pulled first, so no run pays a download.
#
# Clauses:
#   1. each image runs `true` 5 times at exit 0, and each banner names
#      its fill.
#   2. the report: for each image, the size of the tree, the form of the
#      fill, the median fill time and the median wall time of a run, and
#      the share of the fill.
#
# Exit: 0 each clause matched (the numbers are the result), 1 a run
# failed, 2 the lane could not run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep442-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
OUT="$REPO/experiments/results/run-copy-cost.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
export PODVM_STORE="$WORK/store"
# TODO/image.md T-1781: the copy is what this measures, so no spare tree.
export PODVM_SPARE_TREE=0
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
trap 'rm -rf "$WORK"' EXIT INT TERM
[ -x "$PB" ] || { echo "no podvm binary at $PB: COULD NOT RUN" >&2; exit 2; }

reg=public.ecr.aws/docker/library
[ "${PODVM_442_MIRROR:-0}" = 1 ] && reg=mirror.gcr.io/library
IMAGES="$reg/busybox:musl $reg/ubuntu:24.04 $reg/golang:1.24.7-bookworm"

median() { sort -n | awk '{v[NR]=$1} END {print v[int((NR+1)/2)]}'; }
now_ms() { date +%s%3N; }

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "store filesystem  $(stat -fc %T "$WORK")"
note "podvm             $("$PB" version 2>&1 | head -1)"
note ""
note "image | tree | form | fill ms (median of 5) | run ms (median of 5) | fill share"
for img in $IMAGES; do
	timeout 900 "$PB" pull "$img" >"$WORK/pull.txt" 2>&1 || { tail -3 "$WORK/pull.txt" >&2; echo "pull of $img failed: COULD NOT RUN" >&2; exit 2; }
	: >"$WORK/fill.txt"; : >"$WORK/wall.txt"; line=""
	for _ in 1 2 3 4 5; do
		t0=$(now_ms)
		timeout 300 "$PB" run --rm --verbose "$img" true 2>"$WORK/err.txt" >/dev/null
		rc=$?
		t1=$(now_ms)
		[ "$rc" -eq 0 ] || { note "  FAILED: $img exits $rc: $(tail -1 "$WORK/err.txt")"; fail=1; break; }
		line=$(grep "the container's own rootfs" "$WORK/err.txt" | head -1)
		[ -n "$line" ] || { note "  FAILED: $img: the banner names no fill"; fail=1; break; }
		echo "$line" | sed -n 's/.* in \([0-9]*\) ms.*/\1/p' >>"$WORK/fill.txt"
		echo $((t1 - t0)) >>"$WORK/wall.txt"
	done
	[ -s "$WORK/wall.txt" ] || continue
	f=$(median <"$WORK/fill.txt"); w=$(median <"$WORK/wall.txt")
	size=$(echo "$line" | sed -n "s/.*rootfs: \(.*\), by .*/\1/p")
	form=$(echo "$line" | sed -n 's/.*, by \(.*\), in .*/\1/p')
	note "${img##*/} | $size | $form | $f | $w | $((100 * f / (w > 0 ? w : 1))) %"
done
note ""
if [ "$fail" -eq 0 ]; then
	note "verdict: THE NUMBERS ABOVE ARE THE COST OF THE FILL"
else
	note "verdict: A RUN FAILED"
fi
exit "$fail"
