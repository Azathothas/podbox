#!/bin/sh
# Question: does podman's vfs driver with ignore_chown_errors open a path
# where layer chowns fail?
#
# TODO/image.md T-0205. The corpus carries the option
# (`references/containers__storage/tree/drivers/vfs/driver.go`,
# `tree/pkg/archive/archive.go`) and nobody had run the combination. The
# chown wall is staged, not assumed: `experiments/lib/chowndeny.py`
# installs a seccomp filter denying chown to any but the caller's own uid
# for the unpack tree, which is the target runtime's shape (mapped ids
# work, unmapped fail). Two userns attempts were measured first and
# refused: rootless maps every uid into the subuid range so nothing fails,
# and a hand-mapped namespace breaks podman's own newuidmap call.
#
#   ./95-podman-vfs-ignorechown.sh
#
# Exit: 0 the combination opens the path, 1 it does not. Never 2: this
# script runs where the session's engine host is, and an unreachable
# engine is the combination not shown, recorded as such (T-0205's Prove).
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
OUT="$REPO/experiments/results/podman-vfs-ignorechown.txt"
WORK="$REPO/experiments/.sweep95-work"
rm -rf "$WORK"; mkdir -p "$WORK/ctx" || exit 1

# Git Bash converts arguments that look like paths before the program sees
# them. Every podman call below carries machine paths, so conversion stays
# off for the whole script (no-op on a Linux lane).
export MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*'

MACHINE="podman-machine-default"
FIXTAG="t0205-fixture"
FILTER="/tmp/chowndeny95-$$.py"
fail=0
ASTORE=""
BSTORE=""

cleanup() {
	for d in $ASTORE $BSTORE; do
		[ -n "$d" ] || continue
		timeout 60 podman machine ssh "$MACHINE" "rm -rf $d" >/dev/null 2>&1
	done
	timeout 120 podman rmi "$FIXTAG" >/dev/null 2>&1
	rm -rf "$WORK"
}
trap cleanup EXIT HUP INT TERM

report() {
	printf '%s\n' "$1" >>"$WORK/report"
}

have() {
	command -v "$1" >/dev/null 2>&1
}

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host              $(uname -srm)"
if have podman; then
	echo "podman client     $(podman --version 2>/dev/null)"
else
	echo "podman client     ABSENT"
fi
} >"$WORK/report"

if ! have podman || ! have base64 || ! have timeout; then
	report "no podman, base64 or timeout on PATH: the combination is not shown here"
	report "verdict           FAIL: engine host unreachable"
	cp "$WORK/report" "$OUT"
	exit 1
fi
if ! timeout 60 podman machine ssh "$MACHINE" 'id; podman --version' >"$WORK/mcheck" 2>&1; then
	report "podman machine ssh answers nothing: the combination is not shown here"
	report "verdict           FAIL: engine host unreachable"
	cp "$WORK/report" "$OUT"
	exit 1
fi
{
echo "machine           $(grep -o 'podman version [0-9.]*' "$WORK/mcheck" | head -1) ($(head -1 "$WORK/mcheck"))"
echo "machine subuids   $(timeout 60 podman machine ssh "$MACHINE" 'cat /etc/subuid' 2>/dev/null | tr '\n' ' ')"
} >>"$WORK/report"

# The filter travels as base64: one ASCII line, immune to the quoting and
# line-ending layers between this shell and the machine's.
if ! base64 -w0 "$HERE/lib/chowndeny.py" | timeout 60 podman machine ssh "$MACHINE" "base64 -d > $FILTER"; then
	report "the filter could not be staged on the machine"
	report "verdict           FAIL: fixture staging failed"
	cp "$WORK/report" "$OUT"
	exit 1
fi

# The fixture: one file owned by uid 1234, which no subuid range maps.
# Pinned base (scripts/common/distro-matrix.sh M5 alpine row).
cat >"$WORK/ctx/Dockerfile" <<'EOF'
FROM public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc
RUN adduser -D -u 1234 owned && mkdir -p /data && echo hello > /data/greeting && chown 1234:1234 /data/greeting && chmod 644 /data/greeting
CMD ["cat", "/data/greeting"]
EOF
# podman.exe is a native Windows program: it takes the context as a
# Windows path, while this shell spells it POSIX. An MSYS path handed
# over verbatim arrives mangled (measured: /c/... reads as C:\c\...),
# so the one local path in this script goes through cygpath.
if command -v cygpath >/dev/null 2>&1; then
	CTX="$(cygpath -w "$WORK/ctx")"
else
	CTX="$WORK/ctx"
fi
report ""
report "== 0. the fixture is valid (rootful control)"
if ! timeout 300 podman build -t "$FIXTAG" "$CTX" >"$WORK/build.log" 2>&1; then
	report "fixture build failed; tail:"
	tail -3 "$WORK/build.log" >>"$WORK/report"
	report "verdict           FAIL: fixture invalid"
	cp "$WORK/report" "$OUT"
	exit 1
fi
FIXID="$(timeout 60 podman images --quiet "$FIXTAG" 2>/dev/null | head -1)"
if [ -z "$FIXID" ]; then
	report "fixture built but has no image ID"
	report "verdict           FAIL: fixture invalid"
	cp "$WORK/report" "$OUT"
	exit 1
fi
timeout 120 podman run --rm "$FIXTAG" >"$WORK/ctl.log" 2>&1
if [ "$?" -ne 0 ] || [ "$(cat "$WORK/ctl.log")" != "hello" ]; then
	report "rootful control run did not print hello"
	report "verdict           FAIL: fixture invalid"
	cp "$WORK/report" "$OUT"
	exit 1
fi
report "  rootful run prints hello (image $FIXID)"

# One clause, one fresh machine store. $1 names the shell variable holding
# the new store dir, $2 the extra storage.conf lines (empty: no flag).
unstage() {
	_store="$(timeout 60 podman machine ssh "$MACHINE" 'mktemp -d' 2>/dev/null)" || return 1
	timeout 60 podman machine ssh "$MACHINE" "mkdir -p $_store/root $_store/run && printf '[storage]\ndriver=\"vfs\"\nrunroot=\"$_store/run\"\ngraphroot=\"$_store/root\"\n$2' > $_store/storage.conf" || return 1
	eval "$1=\$_store"
	return 0
}

report ""
report "== A. without the flag the wall holds"
if ! unstage ASTORE ""; then
	report "store staging failed"
	fail=1
elif timeout 240 podman save "$FIXTAG" | timeout 240 podman machine ssh "$MACHINE" "export HOME=$ASTORE XDG_RUNTIME_DIR=$ASTORE CONTAINERS_STORAGE_CONF=$ASTORE/storage.conf && podman unshare python3 $FILTER podman load" >"$WORK/a.log" 2>&1; then
	report "load SUCCEEDED without the flag: no wall here"
	fail=1
elif grep -q "lchown /data/greeting: operation not permitted" "$WORK/a.log"; then
	report "  load refused: lchown /data/greeting: operation not permitted"
else
	report "load failed WITHOUT the chown signature; tail:"
	tail -3 "$WORK/a.log" >>"$WORK/report"
	fail=1
fi

report ""
report "== B. with ignore_chown_errors the same load succeeds"
if ! unstage BSTORE '[storage.options.vfs]\nignore_chown_errors="true"\n'; then
	report "store staging failed"
	fail=1
elif ! timeout 240 podman save "$FIXTAG" | timeout 240 podman machine ssh "$MACHINE" "export HOME=$BSTORE XDG_RUNTIME_DIR=$BSTORE CONTAINERS_STORAGE_CONF=$BSTORE/storage.conf && podman unshare python3 $FILTER podman load" >"$WORK/b.log" 2>&1; then
	report "load FAILED with the flag; tail:"
	tail -3 "$WORK/b.log" >>"$WORK/report"
	fail=1
else
	report "  load exits 0 with ignore_chown_errors=true"
	if grep -qi "ignoreChownErrors" "$WORK/b.log"; then
		report "  stderr carries the ignoreChownErrors warning"
	else
		report "  NOTE: stderr carries NO ignoreChownErrors warning on this podman; the corpus prints one (archive.go), the outcome is the flag's either way"
	fi
fi

report ""
report "== C. the flag-loaded image runs"
if [ "$fail" -eq 0 ] && [ -n "$BSTORE" ]; then
	if timeout 240 podman machine ssh "$MACHINE" "export HOME=$BSTORE XDG_RUNTIME_DIR=$BSTORE CONTAINERS_STORAGE_CONF=$BSTORE/storage.conf && timeout 200 podman run --rm --network=none $FIXID" >"$WORK/c.log" 2>&1 && [ "$(cat "$WORK/c.log")" = "hello" ]; then
		report "  run exits 0 and prints hello"
	else
		report "  run failed; tail:"
		tail -3 "$WORK/c.log" >>"$WORK/report"
		fail=1
	fi
fi

report ""
if [ "$fail" -eq 0 ]; then
	report "verdict           PASS: vfs with ignore_chown_errors opens the path the bare driver refuses"
else
	report "verdict           FAIL: see the clauses above"
fi
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo "written to ${OUT#"$REPO"/}"
[ "$fail" -eq 0 ] || exit 1
exit 0
