#!/bin/sh
# Question: does each container of one image get its own rootfs, so that a
# write in one container is not read by another, and what does the copy
# cost?
#
# TODO/image.md T-1709 (issue 234). Before T-1709, each container of one
# image ran in the image's own tree (`<store>/rootfs/<digest>/`). Now
# `prepare` copies that tree into `<store>/trees/<id>/` for each container:
# by FICLONE where the filesystem of the store supports it, else by a
# kernel copy. Each run below goes through the fixture of experiment 356:
# user and mount namespaces, /dev/pts hidden, chroot(2) denied, so the
# payload runs on the userland rung.
#
# Pinned inputs: ALPINE of experiment 356 (alpine 3.20 by digest), and
# BIG, an image of more than 700 MiB extracted (golang 1.24.7-bookworm by
# tag; the conditions print the pulled digest). PODVM_430_MIRROR=1 takes
# both from mirror.gcr.io. PODVM_430_BIG=0 leaves the big image out.
# The result of 2026-11-11 is in two files: container-rootfs.txt (ext4, the
# copy) and container-rootfs-xfs.txt (an XFS loop mount with reflink, the
# FICLONE form).
#
# Clauses:
#   1. a concurrent reader: container A writes /etc/marker and stays; a
#      second container of the same image, started after the write, does
#      not read the file.
#   2. a later run: after A ends (its record stays), a new run does not
#      read /etc/marker; A's own tree still has it.
#   3. two concurrent runs with `--add-host svc:10.0.0.1` and
#      `--add-host svc:10.0.0.2` each read their own value.
#   4. `rm` and `--rm` remove the container's tree: no tree stays after
#      the runs.
#   5. the cost of each form: the banner line of the fill for alpine and
#      for BIG (files, MiB, the form, the milliseconds), the disk of a
#      tree, and whether FICLONE is available on the store's filesystem.
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
# PODVM_430_WORK puts the store on another filesystem (an XFS or btrfs
# mount, for the FICLONE form); PODVM_430_RESULT names the result file.
WORK="${PODVM_430_WORK:-$REPO/experiments/.sweep430-work}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
OUT="$REPO/experiments/results/${PODVM_430_RESULT:-container-rootfs.txt}"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
BIG='public.ecr.aws/docker/library/golang:1.24.7-bookworm'
if [ "${PODVM_430_MIRROR:-0}" = 1 ]; then
	ALPINE='mirror.gcr.io/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
	BIG='mirror.gcr.io/library/golang:1.24.7-bookworm'
fi
STORE="$WORK/store"
export PODVM_STORE="$STORE"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }
trees() { ls "$STORE/trees" 2>/dev/null | wc -l; }

[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" >&2; exit 2; }
unshare -Urm true >/dev/null 2>&1 || { echo "user+mount namespaces refused: COULD NOT RUN" >&2; exit 2; }

# ---- the fixture of experiment 356 ----------------------------------------
cc -O2 -Wall -Werror -static -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>/dev/null ||
	cc -O2 -Wall -Werror -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" ||
	{ echo "fixture COMPILE FAILED: COULD NOT RUN" >&2; exit 2; }
cat >"$WORK/wrap.sh" <<'EOF'
#!/bin/sh
mount -t tmpfs tmpfs /dev/pts || exit 2
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF
fx() { unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$PB" "$@"; }

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "store filesystem  $(stat -fc %T "$WORK")"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "fixture           unshare -Urm, /dev/pts hidden, chroot(2) denied"
note "image             $ALPINE"

timeout 600 "$PB" pull -q "$ALPINE" >"$WORK/pull.log" 2>&1 ||
	{ note "pull FAILED: COULD NOT RUN"; sed 's/^/  /' "$WORK/pull.log"; exit 2; }
note ""

# ---- clause 1: a concurrent reader -----------------------------------------
note "== clause 1  a concurrent reader does not read the write of A"
mkdir -p "$WORK/sig"
fx run --verbose --name a -v "$WORK/sig:/sig" "$ALPINE" \
	sh -c 'echo from-a >/etc/marker && touch /sig/ready && sleep 6' >"$WORK/a.log" 2>&1 &
A=$!
i=0
while [ ! -e "$WORK/sig/ready" ] && [ "$i" -lt 300 ]; do sleep 0.1; i=$((i + 1)); done
[ -e "$WORK/sig/ready" ] || bad "A did not write within 30 s"
fx run --rm "$ALPINE" sh -c 'cat /etc/marker 2>/dev/null || echo absent' >"$WORK/b.log" 2>&1
note "  B read            $(tail -1 "$WORK/b.log")"
[ "$(tail -1 "$WORK/b.log")" = absent ] || bad "B read the write of A"
wait "$A"
rc=$?
note "  A exited          $rc  (want 0)"
[ "$rc" -eq 0 ] || { bad "A exited $rc"; sed 's/^/    /' "$WORK/a.log" | tee -a "$OUT"; }
note ""

# ---- clause 2: a later run --------------------------------------------------
note "== clause 2  a later run does not read the write of A"
fx run --rm "$ALPINE" sh -c 'cat /etc/marker 2>/dev/null || echo absent' >"$WORK/c.log" 2>&1
note "  the later run read $(tail -1 "$WORK/c.log")"
[ "$(tail -1 "$WORK/c.log")" = absent ] || bad "a later run read the write of A"
AROOT=$("$PB" inspect --format '{{.RootfsPath}}' a 2>/dev/null)
note "  A's rootfs        ${AROOT#"$STORE"/}"
case "$AROOT" in "$STORE"/trees/*/rootfs) ;; *) bad "A's rootfs is not a container tree" ;; esac
[ "$(cat "$AROOT/etc/marker" 2>/dev/null)" = from-a ] || bad "A's own tree lost its write"
note ""

# ---- clause 3: two concurrent --add-host values ----------------------------
note "== clause 3  two concurrent --add-host svc: runs read their own value"
fx run --rm --add-host svc:10.0.0.1 "$ALPINE" sh -c 'sleep 2; grep -w svc /etc/hosts' >"$WORK/h1.log" 2>&1 &
H1=$!
fx run --rm --add-host svc:10.0.0.2 "$ALPINE" sh -c 'sleep 2; grep -w svc /etc/hosts' >"$WORK/h2.log" 2>&1 &
H2=$!
wait "$H1"; wait "$H2"
note "  run 1 read        $(tr '\t\n' '  ' <"$WORK/h1.log")"
note "  run 2 read        $(tr '\t\n' '  ' <"$WORK/h2.log")"
grep -q "^10.0.0.1" "$WORK/h1.log" && ! grep -q "10.0.0.2" "$WORK/h1.log" || bad "run 1 did not read its own value"
grep -q "^10.0.0.2" "$WORK/h2.log" && ! grep -q "10.0.0.1" "$WORK/h2.log" || bad "run 2 did not read its own value"
note ""

# ---- clause 4: rm and --rm remove the tree ---------------------------------
note "== clause 4  rm and --rm remove the container's tree"
note "  trees before rm   $(trees)  (want 1: A)"
[ "$(trees)" -eq 1 ] || bad "the --rm runs left a tree"
"$PB" rm a >/dev/null 2>&1 || bad "rm a failed"
note "  trees after rm    $(trees)  (want 0)"
[ "$(trees)" -eq 0 ] || bad "rm left the tree of A"
note ""

# ---- clause 5: the cost ------------------------------------------------------
note "== clause 5  the cost of the copy"
probe="$WORK/ficlone"
mkdir -p "$probe" && printf x >"$probe/a"
if cp --reflink=always "$probe/a" "$probe/b" 2>/dev/null; then
	note "  FICLONE           available on $(stat -fc %T "$WORK")"
else
	note "  FICLONE           not available on $(stat -fc %T "$WORK"): each file is copied"
fi
cost() {
	img="$1"
	fx run --rm --verbose "$img" true >"$WORK/cost.log" 2>&1
	rc=$?
	line=$(grep "the container's own rootfs" "$WORK/cost.log" | sed 's/^podvm: //')
	note "  $2"
	note "    run exited      $rc  (want 0)"
	note "    fill            ${line:--}"
	[ -n "$line" ] || bad "no fill line for $2"
	[ "$rc" -eq 0 ] || { bad "the run of $2 exited $rc"; tail -5 "$WORK/cost.log" | sed 's/^/      /' | tee -a "$OUT"; }
}
cost "$ALPINE" "alpine 3.20"
SRC=$(ls -d "$STORE"/rootfs/*/rootfs | head -1)
note "    image tree      $(du -sm "$SRC" | cut -f1) MiB"
if [ "${PODVM_430_BIG:-1}" = 1 ]; then
	if timeout 1800 "$PB" pull -q "$BIG" >"$WORK/pull-big.log" 2>&1; then
		note "  big image         $BIG"
		note "    digest          $("$PB" inspect --format '{{.Id}}' "$BIG" 2>/dev/null)"
		# The first run extracts; the second shows the copy alone.
		fx run --rm "$BIG" true >/dev/null 2>&1
		cost "$BIG" "golang 1.24.7-bookworm, second run"
		BSRC=$(ls -td "$STORE"/rootfs/*/rootfs | head -1)
		bsize=$(du -sm "$BSRC" | cut -f1)
		note "    image tree      $bsize MiB"
		[ "$bsize" -gt 700 ] || bad "the big image is not larger than 700 MiB"
	else
		bad "the pull of $BIG failed"
		tail -3 "$WORK/pull-big.log" | sed 's/^/    /' | tee -a "$OUT"
	fi
fi
note "  trees after       $(trees)  (want 0)"
[ "$(trees)" -eq 0 ] || bad "a --rm run left its tree"
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: EACH CONTAINER HAS ITS OWN ROOTFS"
else
	note "verdict: A CLAUSE DISAGREED"
fi
rm -rf "$WORK"
exit "$fail"
