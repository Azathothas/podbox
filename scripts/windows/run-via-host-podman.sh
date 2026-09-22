#!/bin/sh
# run-via-host-podman.sh - host-podman substitute for run-in-base.sh.
#
# ⭐ USE ONLY WHILE the wsl-toolkit base is unusable. run-in-base.sh is the
# Windows half of `scripts/dev.sh`; this script does the same job through the
# host's own podman machine instead: the tree is copied into a disposable
# container (nothing is mounted, so nothing the job does reaches this
# checkout), the job runs there, the container is removed when it exits.
#
#   sh scripts/windows/run-via-host-podman.sh              the complete check
#   sh scripts/windows/run-via-host-podman.sh JOB.sh       that script, inside /work
#   PODBOX_IMAGE=... sh scripts/windows/run-via-host-podman.sh
#   PODBOX_ARTIFACTS=DIR sh scripts/windows/run-via-host-podman.sh JOB.sh
#
# A job hands its evidence back through /out, named by PODBOX_ARTIFACTS,
# because the container is removed when it exits.
#
# Exit: the job's own code, or 2 when the job could not be started.
set -u

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 2
ROOT=$(CDPATH= cd -- "$HERE/../.." && pwd) || exit 2
cd "$ROOT" || exit 2

IMAGE="${PODBOX_IMAGE:-docker.io/library/rust:1.98.1-bookworm}"
TIMEOUT="${PODBOX_JOB_TIMEOUT:-45m}"
ARTIFACTS="${PODBOX_ARTIFACTS:-}"

case "${1:-}" in
-h | --help)
	sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
	exit 0
	;;
esac

if ! command -v podman >/dev/null 2>&1; then
	echo "run-via-host-podman: podman is not on PATH." >&2
	exit 2
fi

USER_JOB="${1:-}"
if [ -n "$USER_JOB" ] && [ ! -f "$USER_JOB" ]; then
	echo "run-via-host-podman: no such job script: $USER_JOB" >&2
	exit 2
fi

# winpath P -> the spelling the Windows podman binary wants. `mktemp -d`
# lives outside any drive spelling (/tmp/...), which a sed on the first
# component cannot convert, so cygpath carries it where one is installed.
winpath() {
	if command -v cygpath >/dev/null 2>&1; then
		cygpath -m "$1"
	else
		printf '%s' "$1" | sed 's,^/\([a-zA-Z]\)/,\1:/,'
	fi
}

# Every engine call carries the MSYS guards, which experiments/lib/engine.sh
# holds: a /c/... source arrives at the Windows binary as garbage without
# them, and a container path like /work is rewritten without them.
eng() {
	MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' podman "$@"
}

work=$(mktemp -d) || {
	echo "run-via-host-podman: could not create a work directory." >&2
	exit 2
}
# The container id this run registered, for the cleanup below.
CID=""

cleanup() {
	rm -rf "$work"
	if [ -n "$CID" ]; then
		eng rm -f "$CID" >/dev/null 2>&1 || true
	fi
}
trap cleanup EXIT HUP INT TERM

name="podbox-job-$$-$(date +%s)"
CID="$(eng create --name "$name" "$IMAGE" sleep 3600)" || {
	echo "run-via-host-podman: could not create the job container." >&2
	CID=""
	exit 2
}
eng start "$CID" >/dev/null || {
	echo "run-via-host-podman: could not start the job container." >&2
	exit 2
}

# ⭐ The copy carries .git and references/, which the checks read, and leaves
# out build output, session state and the live-index sidecars. NUL is a
# reserved name Windows opens as the null device (docs/containers.md), so it
# travels as 0 of its bytes and is excluded.
eng exec "$CID" mkdir -p /work /out || exit 2
tar --exclude=target --exclude=.dev \
	--exclude=codegraph.db --exclude=codegraph.db-wal \
	--exclude=codegraph.db-shm --exclude=daemon.log --exclude=daemon.pid \
	--exclude=NUL \
	-cf "$work/tree.tar" . || exit 2
eng cp "$(winpath "$work/tree.tar")" "$CID:/work.tar" || exit 2
eng exec -w /work "$CID" tar -xf /work.tar || exit 2

# ⭐ The wrapper is what makes a caller's job correct by default. It repairs
# the mode bits from the git index (NTFS carries none), then runs the
# payload, then reports the code it actually got.
{
	echo "#!/bin/sh"
	echo "set -u"
	echo "cd /work || exit 2"
	echo 'echo "== the machine"'
	echo "uname -sr; id -u; pwd"
	echo 'echo "== trust the copied checkout for the index read below"'
	echo "git config --global --add safe.directory /work || exit 2"
	echo 'echo "== restore the executable bit that NTFS could not carry"'
	echo "sh scripts/common/restore-modes.sh || exit 2"
	if [ -n "$USER_JOB" ]; then
		echo 'echo "== the job"'
		echo "sh /work/.podbox-job.sh"
	else
		echo 'echo "== bootstrap"'
		echo "./scripts/common/bootstrap-env.sh rust cc zig tools || exit 1"
		echo 'echo "== dev.sh check"'
		echo "./scripts/dev.sh check"
	fi
	echo "rc=\$?"
	echo 'echo "== rc=$rc"'
	echo "exit \"\$rc\""
} >"$work/wrapper.sh"

# ⛔ CRLF is stripped from every payload. A carriage return at the end of a
# line reaches the shell as part of the last word on it.
tr -d "\015" <"$work/wrapper.sh" >"$work/wrapper.lf.sh" || exit 2
eng cp "$(winpath "$work/wrapper.lf.sh")" "$CID:/work/wrapper.sh" || exit 2
if [ -n "$USER_JOB" ]; then
	tr -d "\015" <"$USER_JOB" >"$work/job.lf.sh" || exit 2
	eng cp "$(winpath "$work/job.lf.sh")" "$CID:/work/.podbox-job.sh" || exit 2
fi

if [ -n "$ARTIFACTS" ]; then
	mkdir -p "$ARTIFACTS" || {
		echo "run-via-host-podman: cannot create $ARTIFACTS." >&2
		exit 2
	}
fi

eng exec -w /work "$CID" sh /work/wrapper.sh
rc=$?
echo "== rc=$rc"

if [ -n "$ARTIFACTS" ]; then
	eng cp "$CID:/out/." "$(winpath "$ARTIFACTS")" || {
		echo "run-via-host-podman: could not copy /out back." >&2
		exit 2
	}
fi
exit "$rc"
