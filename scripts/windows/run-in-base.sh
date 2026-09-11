#!/usr/bin/env sh
# run-in-base.sh - run one Linux job against this checkout, from Windows.
#
# ⭐ IT IS THE WINDOWS HALF OF `scripts/dev.sh`. The tree is copied into a
# disposable container inside the distribution `wsl-toolkit-podbox`, the job
# runs there, and the container is removed when it exits. Nothing is mounted
# from this machine, so nothing the job does can reach this checkout.
#
#   sh scripts/windows/run-in-base.sh              the complete check
#   sh scripts/windows/run-in-base.sh JOB.sh       that script, inside /work
#   PODBOX_IMAGE=... sh scripts/windows/run-in-base.sh
#
# ⛔ NEVER CALL wsl.exe. docs/containers.md carries the rule and the reason: a
# payload handed to wsl.exe as an argument is expanded before the guest reads
# it, and the guest then parses the result a second time.
#
# ⚠ Two repairs happen before the job, and both are for the same cause. NTFS
# carries no POSIX mode bit, so the copy arrives with no executable file and
# any CRLF in the payload reaches a POSIX shell as part of a word.
#
# Exit: the job's own code, or 2 when the job could not be started.
set -u

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 2
ROOT=$(CDPATH= cd -- "$HERE/../.." && pwd) || exit 2
cd "$ROOT" || exit 2

IMAGE="${PODBOX_IMAGE:-docker.io/library/rust:1.98.1-bookworm}"
INSTANCE="${PODBOX_WSL_INSTANCE:-podbox}"
TIMEOUT="${PODBOX_JOB_TIMEOUT:-45m}"

case "${1:-}" in
-h | --help)
	sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
	exit 0
	;;
esac

if ! command -v wsl-toolkit >/dev/null 2>&1; then
	echo "run-in-base: wsl-toolkit is not on PATH." >&2
	echo "  docs/agent-tooling.md says where it lives." >&2
	exit 2
fi

USER_JOB="${1:-}"
if [ -n "$USER_JOB" ] && [ ! -f "$USER_JOB" ]; then
	echo "run-in-base: no such job script: $USER_JOB" >&2
	exit 2
fi

work=$(mktemp -d) || {
	echo "run-in-base: could not create a work directory." >&2
	exit 2
}
trap 'rm -rf "$work"' EXIT HUP INT TERM

# ⭐ The wrapper is what makes a caller's job correct by default. It repairs the
# mode bits from the git index, then runs the payload, then reports the code it
# actually got.
{
	echo "#!/bin/sh"
	echo "set -u"
	echo "cd /work || exit 2"
	echo 'echo "== the machine"'
	echo "uname -sr; id -u; pwd"
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

# ⛔ CRLF is stripped from every payload. A carriage return at the end of a line
# reaches the shell as part of the last word on it, and the error names a file
# that nobody wrote.
strip_cr() {
	tr -d "\015" <"$1" >"$2"
}
strip_cr "$work/wrapper.sh" "$work/wrapper.lf.sh"

# ⚠ The caller's job travels INSIDE the workspace, not as the script, because
# --script takes exactly one file and the wrapper is already using it.
staged=""
if [ -n "$USER_JOB" ]; then
	staged="$ROOT/.podbox-job.sh"
	strip_cr "$USER_JOB" "$staged"
fi
cleanup() {
	rm -rf "$work"
	[ -n "$staged" ] && rm -f "$staged"
}
trap cleanup EXIT HUP INT TERM

# ⚠ The exclusions are a decision, and docs/containers.md carries the table.
# .git and references/ are KEPT: the checks read the index and resolve every
# cited path and line in the corpus.
# ⛔ `codegraph.db` BY NAME, never the `.codegraph` directory. An exclusion of
# the directory also matched a TRACKED corpus file under `references/`, so the
# copy arrived one file short and the guest read the tree as dirty.
# `plant.sh` refuses to start on a dirty tree.
wsl-toolkit --instance "$INSTANCE" run \
	--image "$IMAGE" \
	--workspace . \
	--exclude codegraph.db \
	--exclude target \
	--exclude .dev \
	--script "$work/wrapper.lf.sh" \
	--timeout "$TIMEOUT" \
	--tick 120s
rc=$?
exit "$rc"
