#!/bin/sh
# Question: does the spare tree of an image take the copy of its rootfs
# off a run, keep each container apart, and go with the image?
#
# TODO/image.md T-1781. After a run takes its tree, podvm fills the next
# one for the same image in a detached `podvm tree-prefill`; the next run
# takes it with one rename. The tree of a `--rm` run goes aside with one
# rename, and the same helper removes it after podvm's exit.
# `PODVM_SPARE_TREE=0` turns both off.
#
# Pinned inputs:
#   OCI     golang:1.24-bookworm and alpine 3.20 of the store PODVM_STORE
#           (default the store of experiment 443)
#   podvm   PODVM_448_BIN (default the release build of the workspace)
#   RUNS    PODVM_448_RUNS (default 5)
#
# Clauses:
#   1. with the spare, each run after the first names the spare in its
#      banner (`--verbose`), and the median of RUNS runs of `run --rm
#      golang true` (each after its spare is ready) is lower than the
#      median with `PODVM_SPARE_TREE=0` by at least half of the median of
#      the fill with no spare.
#   2. a file that one run writes under / is not in the root of the next
#      run, which took the spare.
#   3. in a fresh store with alpine: `rmi` of a second tag of the image
#      keeps its extraction and its spare; `rmi` of the last tag removes
#      both (before T-1781, `rmi` left each extraction).
#   4. no `tree-prefill` stays after the last spare is ready, and no tree
#      that a `--rm` put aside stays in the trash.
#
# Exit: 0 each clause matched, 1 one or more failed, 2 the lane could not
# run.
set -u

REPO="$(cd "$(dirname "$0")/.." && pwd)"
PB="${PODVM_448_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
export PODVM_STORE="${PODVM_STORE:-$REPO/.work/qa/store}"
RUNS="${PODVM_448_RUNS:-5}"
G=mirror.gcr.io/library/golang:1.24-bookworm
A=mirror.gcr.io/library/alpine:3.20
W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT INT TERM

[ -x "$PB" ] || { echo "no podvm binary at $PB: COULD NOT RUN" >&2; exit 2; }
for i in "$G" "$A"; do
	"$PB" image inspect "$i" >/dev/null 2>&1 || { echo "no $i in $PODVM_STORE: COULD NOT RUN" >&2; exit 2; }
done

FAIL=0
verdict() { # clause, test, what
	if eval "$2"; then echo "PASS $1: $3"; else echo "FAIL $1: $3"; FAIL=1; fi
}
ms() { echo $(( $(date +%s%N) / 1000000 )); }
median() { tr ' ' '\n' | grep . | sort -n | awk '{a[NR]=$1} END {print (NR ? a[int((NR+1)/2)] : "-")}'; }
digest_of() { "$PB" image inspect --format '{{.Id}}' "$1" 2>/dev/null | sed 's/:/_/'; }
spare_of() { echo "$PODVM_STORE/rootfs/$(digest_of "$1")/spare"; }
ready() { # image: wait until its spare is there and no prefill runs (300 s)
	_w=0
	while { [ ! -d "$(spare_of "$1")" ] || pgrep -f "tree-prefill $PODVM_STORE" >/dev/null; } && [ $_w -lt 300 ]; do
		sleep 1; _w=$((_w + 1))
	done
}

# 1. The cost with no spare, then with the spare.
off=""; fill=""
i=0
while [ $i -lt "$RUNS" ]; do
	t0=$(ms); line=$(PODVM_SPARE_TREE=0 "$PB" run --rm --verbose "$G" true 2>&1 | grep "own rootfs"); t1=$(ms)
	off="$off $((t1 - t0))"
	fill="$fill $(echo "$line" | sed -n 's/.* in \([0-9]*\) ms.*/\1/p')"
	i=$((i + 1))
done
"$PB" run --rm "$G" true >/dev/null 2>&1
on=""; named=0
i=0
while [ $i -lt "$RUNS" ]; do
	ready "$G"
	t0=$(ms); line=$("$PB" run --rm --verbose "$G" true 2>&1 | grep "own rootfs"); t1=$(ms)
	on="$on $((t1 - t0))"
	case "$line" in *"by the spare tree"*) named=$((named + 1)) ;; esac
	i=$((i + 1))
done
mo=$(echo "$off" | median); mn=$(echo "$on" | median); mf=$(echo "$fill" | median)
echo "  1| no spare:$off (median $mo ms; fill median $mf ms)"
echo "  1| spare:$on (median $mn ms; $named of $RUNS named the spare)"
verdict 1 '[ "$named" = "$RUNS" ] && [ $((mo - mn)) -ge $((mf / 2)) ]' \
	"the spare saves $((mo - mn)) ms of a run; the fill costs $mf ms"

# 2. A write of one run is not in the next run's root.
ready "$A"
"$PB" run --rm "$A" sh -c 'echo leak > /leak448' >/dev/null 2>&1
ready "$A"
seen=$("$PB" run --rm --verbose "$A" sh -c 'cat /leak448 2>/dev/null || echo clean' 2>"$W/e2")
echo "  2| $seen ($(grep -o 'by [a-zA-Z ]*,' "$W/e2" | head -1))"
verdict 2 '[ "$seen" = clean ] && grep -q "by the spare tree" "$W/e2"' "the next run takes the spare, and it holds no write of the run before"

# 3. rmi removes the spare, in a store of its own.
S3="$W/store"
mkdir -p "$S3" && chmod 700 "$S3"
PODVM_STORE="$S3" "$PB" pull -q "$A" >/dev/null 2>&1 && PODVM_STORE="$S3" "$PB" run --rm "$A" true >/dev/null 2>&1
i=0; while [ -z "$(ls -d "$S3"/rootfs/*/spare 2>/dev/null)" ] && [ $i -lt 120 ]; do sleep 1; i=$((i + 1)); done
before=$(ls -d "$S3"/rootfs/*/spare 2>/dev/null | wc -l)
while pgrep -f "tree-prefill $S3" >/dev/null; do sleep 1; done
PODVM_STORE="$S3" "$PB" tag "$A" other448:1 >/dev/null 2>&1
PODVM_STORE="$S3" "$PB" rmi other448:1 >/dev/null 2>&1
shared=$(ls -d "$S3"/rootfs/*/rootfs "$S3"/rootfs/*/spare 2>/dev/null | wc -l)
PODVM_STORE="$S3" "$PB" rmi "$A" >/dev/null 2>&1
after=$(ls "$S3"/rootfs 2>/dev/null | wc -l)
echo "  3| spares before rmi $before; after rmi of a second tag: $shared of 2 kept; after rmi of the last tag: $after entries"
verdict 3 '[ "$before" = 1 ] && [ "$shared" = 2 ] && [ "$after" = 0 ]' \
	"rmi of a tag keeps an extraction that another tag names; rmi of the last removes it with the spare"

ready "$G"; ready "$A"
left=$(pgrep -af "tree-prefill" 2>/dev/null | cut -c1-100)
trash=$(ls -a "$PODVM_STORE/trees" 2>/dev/null | grep -c '^\.trash-')
verdict 4 '[ -z "$left" ] && [ "$trash" = 0 ]' "no tree-prefill stays (${left:-none}), and the trash is empty ($trash)"

if [ $FAIL = 0 ]; then echo "THE SPARE TREE TAKES THE COPY OFF A RUN"; fi
exit $FAIL
