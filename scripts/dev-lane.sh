#!/usr/bin/env sh
# dev-lane.sh - the one lane runner. Every proof runs through this and nothing
# else: no hand-rolled `wsl-toolkit run --workspace`, no `wslc` for proofs,
# never `wsl.exe` with a payload argument, never `wsl --shutdown`.
#
#   sh scripts/dev-lane.sh run JOB.sh [--user] [--artifacts DIR]
#   sh scripts/dev-lane.sh gc [--apply]     # list retained jobs, or collect them all
#   sh scripts/dev-lane.sh check             # host readiness (tool, base probe)
#
# ⛔ RUN THIS FROM GIT BASH. The workspace travels as relative `.` after a cd
# to the root, which is the form no MSYS rewrite can touch. PowerShell callers
# must convert paths themselves; this script does not accept them.
#
# ⛔ HOST PYTHON IS `py`, NEVER `python3`. `python3` on this Windows host is a
# Microsoft Store stub (exit 49). Guest payloads keep `python3`.
#
# Exit: the job's own code. 2 means the harness refused or could not start
# (lint failure, missing tool, gc leftovers) and the line says why.
# A job file ends in an exit-code capture, never a pipe: `cmd >log 2>&1`
# followed by `rc=$?` on the next line. This script REFUSES to launch a job
# that pipes a cargo line into anything, because that reports the pipe's
# status and a red suite reads green.
#
# What this closes beyond refactor/lane.sh (kept as history, not used):
#  1. gc uses the full 16-hex id from the `--json` answer and then verifies
#     the ledger is empty; lane.sh never checked, and the 12-char header
#     form exits 0 while removing nothing. Measured: bare 16-hex collects
#     ("ledger compacted to 0 open records").
#  2. The job file is linted before launch: cargo-into-pipe and stacked
#     positional cargo-test filters are refused with the line number.
#  3. With --artifacts, the returned logs are asserted: at least one
#     `test result: ok` with a nonzero pass count where the job ran cargo
#     test, and no `FAILED` or `panicked` line. A green exit over zero
#     executed tests is refused. (`0 passed` lines from multi-target runs
#     are fine beside a real pass; only the absence of any pass fails.)
#  4. `gc` collects BEFORE the record gate pattern: one neighbour's dead job
#     emits ~5 gate errors each, so the subcommand exists to run first.

set -u

# Symlink-resolving self-location: this script may be reached through a link,
# and `dirname $0` then names the link's directory instead of this file's.
_self="$0"
case "$_self" in
*/*) ;;
*) _self=$(command -v "$_self" 2>/dev/null || printf '%s' "$_self") ;;
esac
while [ -L "$_self" ]; do
	_link=$(readlink "$_self") || break
	case "$_link" in
	*/*) _self="$_link" ;;
	*) _self="$(dirname -- "$_self")/$_link" ;;
	esac
done
HERE=$(CDPATH= cd -- "$(dirname -- "$_self")" && pwd) || exit 2
ROOT=$(CDPATH= cd -- "$HERE/.." && pwd) || exit 2

INSTANCE="${PODBOX_LANE_INSTANCE:-podbox}"
IMAGE="${PODBOX_LANE_IMAGE:-docker.io/library/rust:1.98.1-bookworm}"
TIMEOUT="${PODBOX_LANE_TIMEOUT:-45m}"
# Workspace ceiling, bytes. The tool defaults to 1.0 GiB and this tree copies
# 343.6 MiB today; 5 GiB is headroom for corpus growth, not a license to bloat
# the copy. If a run ever approaches it, shrink the workspace, not the ceiling.
MAX_BYTES="${PODBOX_LANE_MAX_BYTES:-5368709120}"
# The tool itself, overridable for shims. Every invocation below goes through
# it; the prose keeps the real name so the next reader knows what it is.
TOOL="${PODBOX_LANE_TOOL:-wsl-toolkit}"

CMD="${1:-}"
[ $# -gt 0 ] && shift

# ------------------------------------------------------------------ tool found

if ! command -v "$TOOL" >/dev/null 2>&1; then
	echo "dev-lane: $TOOL is not on PATH." >&2
	echo "  docs/agent-tooling.md says where it lives." >&2
	exit 2
fi

# ⛔ THE AUTHORITY IS `wsl-toolkit man --no-pager`, NOT `--help`. A `--help`
# list was truncated on this machine once and cost the claim that `run --user`
# did not exist. It does. Never conclude a flag is absent from `--help` alone.

# ------------------------------------------------------------------ gc

# gc [--apply]: list retained jobs; with --apply, collect them and then prove
# the ledger is empty. Never gc another session's evidence: this collects
# only jobs this machine's sessions left, and it says their count first.
if [ "$CMD" = "gc" ]; then
	APPLY=0
	[ "${1:-}" = "--apply" ] && APPLY=1
	before=$("$TOOL" --instance "$INSTANCE" gc 2>&1)
	rc=$?
	printf '%s\n' "$before"
	[ "$rc" -ne 0 ] && exit 2
	if [ "$APPLY" = "0" ]; then
		exit 0
	fi
	after=$("$TOOL" --instance "$INSTANCE" gc --apply 2>&1)
	arc=$?
	printf '%s\n' "$after"
	[ "$arc" -ne 0 ] && exit 2
	left=$("$TOOL" --instance "$INSTANCE" gc 2>&1 | grep -c "host dir" || true)
	if [ "$left" -ne 0 ]; then
		echo "dev-lane: gc left $left retained job(s); refusing to call this clean." >&2
		exit 2
	fi
	echo "dev-lane: ledger empty."
	exit 0
fi

# ------------------------------------------------------------------ check

if [ "$CMD" = "check" ]; then
	echo "== lane: $INSTANCE"
	"$TOOL" --instance "$INSTANCE" base status --probe 2>&1
	rc=$?
	echo "== exit $rc"
	exit "$rc"
fi

# ------------------------------------------------------------------ run: lint

if [ "$CMD" != "run" ]; then
	echo "dev-lane: usage: sh scripts/dev-lane.sh run JOB.sh [--user] [--artifacts DIR]" >&2
	echo "                sh scripts/dev-lane.sh gc [--apply]" >&2
	echo "                sh scripts/dev-lane.sh check" >&2
	exit 2
fi

JOB=""
WANT_USER=0
ARTIFACTS=""
while [ $# -gt 0 ]; do
	case "$1" in
	--user) WANT_USER=1; shift ;;
	--artifacts)
		[ $# -lt 2 ] && { echo "dev-lane: --artifacts needs a directory." >&2; exit 2; }
		[ -n "${2:-}" ] || { echo "dev-lane: --artifacts needs a non-empty directory." >&2; exit 2; }
		ARTIFACTS="$2"; shift 2 ;;
	-*) echo "dev-lane: unknown option $1" >&2; exit 2 ;;
	*)
		[ -n "$JOB" ] && { echo "dev-lane: one job only." >&2; exit 2; }
		JOB="$1"; shift ;;
	esac
done

[ -z "$JOB" ] && { echo "dev-lane: name a job script." >&2; exit 2; }
[ -f "$JOB" ] || { echo "dev-lane: no such job: $JOB" >&2; exit 2; }

# ⛔ MECHANICAL LINT 1: a cargo line piped into anything reports the pipe's
# status. `cargo test ... | tail` reads rc=0 over a red suite. Refused here,
# with the line number, before anything launches. Anchored to a cargo
# command position so `grep cargo | head` does not trip it. An `env VAR=..`
# prefix changes nothing about what runs, so both lints read a normalized
# copy with it stripped: `env CC=x cargo test | tail` is still refused.
lint_tmp=$(mktemp) || { echo "dev-lane: no temporary file." >&2; exit 2; }
sed 's/^[[:space:]]*env[[:space:]][^|;&]*\<cargo[[:space:]]/cargo /' "$JOB" >"$lint_tmp"
piped=$(grep -nE '^[[:space:]]*cargo[^|]*\|' "$lint_tmp" || true)
if [ -n "$piped" ]; then
	echo "dev-lane: refusing job with a piped cargo line (exit code would be the pipe's):" >&2
	printf '%s\n' "$piped" | head -5 >&2
	rm -f "$lint_tmp"
	exit 2
fi

# ⛔ MECHANICAL LINT 2: `cargo test` takes one positional filter; extras go
# after `--`. Two bare positionals silently narrow or error depending on the
# runner. Refused here. Flags taking values are skipped, not counted.
multifiltered=$(awk '
/^[[:space:]]*cargo[[:space:]]+test([[:space:]]|$)/ {
	n = split($0, w, /[[:space:]]+/)
	pos = 0; skipnext = 0
	for (i = 1; i <= n; i++) {
		if (skipnext) { skipnext = 0; continue }
		if (w[i] == "cargo" || w[i] == "test") continue
		if (w[i] == "--") break
		if (w[i] == ">" || w[i] == "2>&1" || substr(w[i],1,1) == ">") break
		if (w[i] ~ /^(-p|--package|--manifest-path|--target|--test|--bin|--example|--features|--config)$/) { skipnext = 1; continue }
		if (substr(w[i],1,1) == "-") continue
		pos++
	}
	if (pos > 1) print NR": "$0
}' "$lint_tmp" || true)
if [ -n "$multifiltered" ]; then
	echo "dev-lane: refusing stacked cargo-test filters (one positional per line, extras after --):" >&2
	printf '%s\n' "$multifiltered" | head -5 >&2
	rm -f "$lint_tmp"
	exit 2
fi
rm -f "$lint_tmp"

# ------------------------------------------------------------------ run: launch
# Payload mechanics ported from refactor/lane.sh (traps T1-T7 there, verified):
# bootstrap as root (zig/jq absent bare), drop via file (never quoted string),
# chown (never safe.directory), codegraph.db by name, PATH prepend, no exec.

work=$(mktemp -d) || { echo "dev-lane: no work directory." >&2; exit 2; }
trap 'rm -rf "$work"' EXIT HUP INT TERM

tr -d "\015" <"$JOB" >"$work/job.lf.sh"

cat >"$work/drop.sh" <<'DROP'
#!/usr/bin/env sh
export HOME="${LANE_HOME:-/home/toolkit}"
mkdir -p "$HOME" 2>/dev/null || true
[ -w "$HOME" ] || export HOME=/tmp/lane-home
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin${PATH:+:$PATH}"
cd /work || exit 2
echo "uid=$(id -u) user=$(id -un) pwd=$(pwd) HOME=$HOME"
rc=0
sh /in/job.sh || rc=$?
echo "== rc=$rc"
exit "$rc"
DROP
tr -d "\015" <"$work/drop.sh" >"$work/drop.lf.sh"

{
	echo "#!/usr/bin/env sh"
	echo "set -u"
	echo "mkdir -p /out || exit 2"
	echo 'echo "== the machine"'
	echo "uname -sr; id -u; id -un; pwd"
	echo 'echo "== the toolset"'
	echo "./scripts/common/bootstrap-env.sh rust cc zig tools openssh || exit 1"
	echo "rustc --version; cargo --version"
	echo 'echo "== disk, before anything is written"'
	echo 'df -h /work | tail -1'
	if [ "$WANT_USER" = "1" ]; then
		echo 'echo "== lane: preparing a non-root account"'
		echo "export DEBIAN_FRONTEND=noninteractive"
		echo "apt-get update -qq >/dev/null 2>&1"
		echo "apt-get install -y -qq sudo >/dev/null 2>&1"
		echo "id toolkit >/dev/null 2>&1 || useradd -m -u 1000 -s /bin/sh toolkit"
		echo "chown -R toolkit:toolkit /work 2>/dev/null"
		echo "TID=\$(getent passwd toolkit | cut -d: -f3)"
		echo 'echo "== lane: account uid $TID"'
		echo "mkdir -p /etc/sudoers.d"
		echo "printf 'toolkit ALL=(ALL) NOPASSWD:ALL\\n' >/etc/sudoers.d/toolkit"
		echo "chmod 0440 /etc/sudoers.d/toolkit"
		echo "visudo -c >/dev/null 2>&1 || { echo 'lane: the sudo rule does not parse' >&2; exit 2; }"
		echo 'echo "== lane: dropping to the unprivileged account"'
		echo 'setpriv --reuid="$TID" --regid="$TID" --clear-groups -- /bin/sh /in/drop.sh'
		echo "drop_rc=\$?"
		echo 'echo "== lane: rc=$drop_rc"'
		echo 'exit "$drop_rc"'
	fi
	echo 'echo "== the job"'
	echo 'rc=0'
	echo 'sh /in/job.sh || rc=$?'
	echo 'echo "== rc=$rc"'
	echo 'exit "$rc"'
} >"$work/payload.sh"
tr -d "\015" <"$work/payload.sh" >"$work/payload.lf.sh"

# ⚠ The exclusions are a decision and `docs/containers.md` carries the table.
# The ceiling is 5 GiB (`$MAX_BYTES`, was the tool default 1.0 GiB) and this
# tree copies 343.6 MiB; `references/` and `experiments/` stay IN because the
# record gate resolves every cited path.
# `codegraph.db` BY NAME, never the `.codegraph` directory (it would match a
# TRACKED corpus file and the guest would read the tree as dirty).
set -- --exclude codegraph.db --exclude codegraph.db-wal \
	--exclude codegraph.db-shm --exclude daemon.log --exclude daemon.pid \
	--exclude target --exclude .dev --exclude .tmp
if [ -n "$ARTIFACTS" ]; then
	mkdir -p "$ARTIFACTS" || { echo "dev-lane: cannot create $ARTIFACTS" >&2; exit 2; }
	set -- "$@" --artifacts "$ARTIFACTS"
fi

# Relative workspace after cd: rewrite-proof (run-in-base.sh form).
cd "$ROOT" || exit 2
out_file="$work/answer.txt"
"$TOOL" --instance "$INSTANCE" run \
	--image "$IMAGE" \
	--workspace . \
	--max-bytes "$MAX_BYTES" \
	--container-lifecycle ephemeral \
	--user 0:0 \
	"$@" \
	--input "job.sh=$work/job.lf.sh" \
	--input "drop.sh=$work/drop.lf.sh" \
	--script "$work/payload.lf.sh" \
	--json \
	--timeout "$TIMEOUT" \
	--tick 120s >"$out_file" 2>&1
rc=$?

cat "$out_file"

# ------------------------------------------------------------------ run: collect
# FULL id, then verify. The 12-char form exits 0 and removes nothing.

job_id=$(grep -ao '"id"[[:space:]]*:[[:space:]]*"[0-9a-f]\{16\}"' "$out_file" | head -1 | grep -ao '[0-9a-f]\{16\}' | head -1)
if [ -z "$job_id" ]; then
	echo "dev-lane: no job id in the answer; nothing to collect." >&2
	exit 2
fi
"$TOOL" --instance "$INSTANCE" gc --job "$job_id" --apply >/dev/null 2>&1
left=$("$TOOL" --instance "$INSTANCE" gc 2>&1 | grep -c "$job_id" || true)
if [ "$left" -ne 0 ]; then
	echo "dev-lane: job $job_id still retained after gc; collect it by hand." >&2
	exit 2
fi

# ------------------------------------------------------------------ run: assert evidence
# With --artifacts, the returned logs must show executed, passing tests.
# Without them there is nothing to assert beyond the job's own exit code.

# Full-line comments run nothing: a `# cargo test` in a comment is not a test
# run and must not trigger the evidence assertion.
if [ -n "$ARTIFACTS" ] && grep -v '^[[:space:]]*#' "$work/job.lf.sh" | grep -q "cargo test"; then
	results=$(grep -ah "test result:" "$ARTIFACTS"/*.log 2>/dev/null || true)
	if [ -z "$results" ]; then
		echo "dev-lane: job ran cargo test but no test result reached $ARTIFACTS." >&2
		exit 2
	fi
	bad=$(printf '%s\n' "$results" | grep -ac "FAILED\|panicked" || true)
	if [ "$bad" -ne 0 ]; then
		echo "dev-lane: failing test result in $ARTIFACTS:" >&2
		printf '%s\n' "$results" | grep -a "FAILED\|panicked" | head -5 >&2
		exit 2
	fi
	good=$(printf '%s\n' "$results" | grep -ac "test result: ok\. [1-9]" || true)
	if [ "$good" -eq 0 ]; then
		echo "dev-lane: no passing test result in $ARTIFACTS." >&2
		exit 2
	fi
	echo "dev-lane: $good passing suite(s) in evidence."
fi

if [ "$rc" -eq 0 ]; then
	echo "dev-lane: green (job rc=0)."
else
	echo "dev-lane: FAILED (job rc=$rc)." >&2
fi
exit "$rc"
