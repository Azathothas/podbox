#!/usr/bin/env sh
# lane.sh - run one job on a Linux lane, as a non-root user, with the
# repository's own toolset available. This is the entry point every implementor
# uses. It exists because the plain lane has four traps that each cost an agent
# a session, and this script closes all four before the payload runs.
#
#   sh refactor/lane.sh JOB.sh              the job, on the default lane
#   sh refactor/lane.sh JOB.sh --user       the job, as uid 1000
#   sh refactor/lane.sh --check             is this host ready, and what is missing
#   sh refactor/lane.sh --preflight         everything --check does, plus one probe run
#   sh refactor/lane.sh JOB.sh --no-bootstrap   skip the toolset install
#   sh refactor/lane.sh JOB.sh --print-command   print the wsl-toolkit call, run nothing
#
# ⛔ RUN THIS FROM GIT BASH, NOT FROM POWERSHELL, and read its exit code
# without a pipe. This script exists to be the Windows half of a job, and it
# calls `wsl-toolkit` from a shell that does not rewrite guest paths. The tool
# must never see a Git-Bash-rewritten path: MSYS turns `--dir /workspaces/x`
# into `C:/Program Files/Git/workspaces/x` and the tool refuses the rewrite.
#
# ⛔ NEVER call `wsl.exe` with a payload argument, and never `wsl --shutdown`.
# `docs/containers.md` carries the first rule; the second breaks every WSL
# instance on the machine, not just ours.
#
# Exit: the job's own code. 2 means the job could not be started, and the line
# says why. 3 means the host is not ready and the fix is printed.

set -u

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 2
ROOT=$(CDPATH= cd -- "$HERE/.." && pwd) || exit 2

INSTANCE="${PODBOX_LANE_INSTANCE:-podbox}"
IMAGE="${PODBOX_LANE_IMAGE:-docker.io/library/rust:1.98.1-bookworm}"
TIMEOUT="${PODBOX_LANE_TIMEOUT:-45m}"
JOB=""
WANT_USER=0
BOOTSTRAP=1
PRINT_ONLY=0
DUMP_ONLY=0
MODE="run"

# ------------------------------------------------------------------ arguments

while [ $# -gt 0 ]; do
	case "$1" in
	--check | --preflight)
		MODE="$1"
		shift
		;;
	--user)
		WANT_USER=1
		shift
		;;
	--no-bootstrap)
		BOOTSTRAP=0
		shift
		;;
	--print-command)
		PRINT_ONLY=1
		shift
		;;
	--dump-payload)
		DUMP_ONLY=1
		shift
		;;
	-h | --help)
		sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
		exit 0
		;;
	-*)
		echo "lane.sh: unknown option $1" >&2
		exit 2
		;;
	*)
		[ -n "$JOB" ] && {
			echo "lane.sh: one job only, and $JOB is already set." >&2
			exit 2
		}
		JOB="$1"
		shift
		;;
	esac
done

# ------------------------------------------------------------------ tool found

if ! command -v wsl-toolkit >/dev/null 2>&1; then
	echo "lane.sh: wsl-toolkit is not on PATH." >&2
	echo "  docs/agent-tooling.md says where it lives." >&2
	exit 2
fi

# ⛔ THE AUTHORITY IS `wsl-toolkit man --no-pager`, NOT `--help`. A `--help`
# list was truncated on this machine once and cost the claim that `run --user`
# did not exist. It does. Never conclude a flag is absent from `--help` alone.

# ------------------------------------------------------------------ readiness

# ⛔ `base status` without `--probe` reads the configuration and answers
# `registered true` for a base that cannot run anything. Only `--probe` starts
# a container and reads every adapter back from the machine.
ready_report() {
	wsl-toolkit --instance "$INSTANCE" base status --probe 2>&1
}

if [ "$MODE" != "run" ]; then
	echo "== lane: $INSTANCE"
	ready_report
	rc=$?
	echo "== exit $rc"
	if [ "$MODE" = "--preflight" ]; then
		echo "== a probe run"
		tmp=$(mktemp) || exit 2
		cat >"$tmp" <<'PROBE'
#!/usr/bin/env sh
set -u
echo "uid=$(id -u) user=$(id -un)"
echo "cc=$(command -v cc || echo MISSING)"
echo "zig=$(command -v zig || echo MISSING)"
echo "jq=$(command -v jq || echo MISSING)"
echo "sshd=$(command -v sshd || echo MISSING)"
echo "PREFLIGHT_OK"
PROBE
		wsl-toolkit --instance "$INSTANCE" run \
			--image "$IMAGE" --container-lifecycle ephemeral \
			--script "$tmp" --timeout 5m
		echo "== probe exit $?"
		rm -f "$tmp"
	fi
	exit "$rc"
fi

# ------------------------------------------------------------------ the job

if [ -z "$JOB" ]; then
	echo "lane.sh: name a job script, or use --check." >&2
	exit 2
fi
if [ ! -f "$JOB" ]; then
	echo "lane.sh: no such job: $JOB" >&2
	exit 2
fi

work=$(mktemp -d) || {
	echo "lane.sh: could not create a work directory." >&2
	exit 2
}
trap 'rm -rf "$work"' EXIT HUP INT TERM

# ⚠ The caller's job is an input file. Its bytes are not changed, so this
# strips CRLF before it sends the payload: a carriage return at the end of a
# line reaches the shell as part of the last word on it, and the error then
# names a file nobody wrote.
tr -d "\015" <"$JOB" >"$work/job.lf.sh"

# The drop script, for --user only. It is a file, not an argument, so no shell
# quoting sits between the payload and the account it drops to.
cat >"$work/drop.sh" <<'DROP'
#!/usr/bin/env sh
# Runs as the unprivileged account. Reports who it is, then runs the job.
export HOME="${LANE_HOME:-/home/toolkit}"
mkdir -p "$HOME" 2>/dev/null || true
[ -w "$HOME" ] || export HOME=/tmp/lane-home
# ⚠ APPEND, DO NOT REPLACE. cargo and rustup live in the image's
# CARGO_HOME=/usr/local/cargo/bin, which is on root's PATH and not on a bare
# one. Measured: replacing PATH gave `cargo: not found` and exit 127. sudo
# lands in /usr/bin, which a login profile may take off PATH, so the standard
# directories are PREPENDED and the image's own PATH is kept after them.
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin${PATH:+:$PATH}"
cd /work || exit 2
echo "uid=$(id -u) user=$(id -un) pwd=$(pwd) HOME=$HOME"
rc=0
sh /in/job.sh || rc=$?
echo "== rc=$rc"
exit "$rc"
DROP
tr -d "\015" <"$work/drop.sh" >"$work/drop.lf.sh"

# The payload. It runs as the container's own user, which is root unless
# `--user` is given, and it hands evidence back through /out.
#
# ⛔ TRAP 1, the toolset. A bare job image has `cc` and `ssh` and no `zig` and
# no `jq`. `.cargo/config.toml` routes the C compiler to `scripts/zig-cc.sh`, so
# every crate that pulls `ring` fails at `cc-rs` before a single test runs.
# Only `podbox-probe` is exempt. CI avoids this by bootstrapping first.
# ⛔ TRAP 2, sudo under `--user`. The bootstrap pins `ZIG_PREFIX="/opt/zig"`
# (scripts/common/bootstrap-env.sh:71) and needs root to write there. Under
# `--user 1000:1000` it fails with `mv: cannot move ... to '/opt/zig':
# Permission denied` and reports `FAILED zig`. So the job starts as root,
# lays down one passwordless sudo rule, and drops to the account for the real
# work. Measured: `installed zig: 0.16.0 at /opt/zig`, 0 failed, at uid 1000.
{
	echo "#!/usr/bin/env sh"
	echo "set -u"
	echo "mkdir -p /out || exit 2"
	echo 'echo "== the machine"'
	echo "uname -sr; id -u; id -un; pwd"
	if [ "$BOOTSTRAP" = "1" ]; then
		echo 'echo "== the toolset"'
		echo "./scripts/common/bootstrap-env.sh rust cc zig tools || exit 1"
	fi
	if [ "$WANT_USER" = "1" ]; then
		# The bootstrap above runs as root, which is what it needs: it pins
		# ZIG_PREFIX=/opt/zig. The account is set up AFTER, because a
		# --user 0:0 job has no unprivileged account to grant sudo to, and
		# the sudoers rule is worthless without the toolset it unlocks.
		echo 'echo "== lane: preparing a non-root account"'
		echo "export DEBIAN_FRONTEND=noninteractive"
		echo "apt-get update -qq >/dev/null 2>&1"
		echo "apt-get install -y -qq sudo >/dev/null 2>&1"
		echo "id toolkit >/dev/null 2>&1 || useradd -m -u 1000 -s /bin/sh toolkit"
		# ⛔ The copy arrives owned by root, because the tool built it as root.
		# A non-root user then cannot write /work, and git refuses it outright:
		# `fatal: detected dubious ownership in repository at '/work'`, which
		# turns check-todo.py into "git ls-files failed ... This is not a pass".
		# Both are fixed by handing the tree to the account, NOT by adding a
		# safe.directory exception: a job that cannot write the code it tests
		# proves nothing.
		echo "chown -R toolkit:toolkit /work 2>/dev/null"
		# A --user 0:0 job has no unprivileged account, so useradd gives the
		# new one home=/home/toolkit. When the account already exists its home
		# is whatever the image says, and `rust:1.98.1-bookworm` resolves
		# uid 1000 to /root, which uid 1000 cannot write. Point HOME somewhere
		# writable before dropping, or cargo and rustup both fail on $CARGO_HOME.
		echo "TID=\$(getent passwd toolkit | cut -d: -f3)"
		echo 'echo "== lane: account uid $TID"'
		echo "mkdir -p /etc/sudoers.d"
		echo "printf 'toolkit ALL=(ALL) NOPASSWD:ALL\\n' >/etc/sudoers.d/toolkit"
		echo "chmod 0440 /etc/sudoers.d/toolkit"
		echo "visudo -c >/dev/null 2>&1 || { echo 'lane: the sudo rule does not parse' >&2; exit 2; }"
		echo 'echo "== lane: dropping to the unprivileged account"'
		# ⛔ NOT a nested `sh -c "..."` string. Measured: the escaped-quote
		# payload died silently after the drop line, exit 2, with nothing on
		# either stream, and the cause was not visible from the outside. A
		# separate script file has no quoting layer at all, so the drop is one
		# command with no argument to mangle. It travels as a second input.
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
# `.git` and `references/` are KEPT: the record gate resolves every cited path
# and line in the corpus, and `TODO/reference-map.md` reads it.
# ⛔ TRAP 3. `codegraph.db` BY NAME, never the `.codegraph` directory. An
# exclusion of the directory also matches a TRACKED corpus file under
# `references/`, the copy arrives one file short, and the guest reads the tree
# as dirty. `plant.sh` refuses to start on a dirty tree.
# ⛔ `target` stays excluded. Two `cargo build`s on one shared target directory
# both fail `E0463: couldn't find crate`; each job gets its own.
set -- --exclude codegraph.db --exclude codegraph.db-wal \
	--exclude codegraph.db-shm --exclude daemon.log --exclude daemon.pid \
	--exclude target --exclude .dev --exclude .tmp

# ⛔ TRAP 4, `--max-bytes` is 1.0 GiB and this tree is near it. A call that
# does not carry these exclusions is refused outright:
# `workspace refused: the workspace passes 1.0 GiB at
# references/mhx__dwarfs/api/releases.json`, exit 2. And every large exclusion
# turns the record gate red: `--exclude references` gives 12 "does not exist"
# errors from `TODO/reference-map.md`, `--exclude experiments` a
# `FileNotFoundError` on `experiments/110-bloat-delta.sh`. The safe reduction
# is zero bytes. The 343 MiB copy is the correct number.

if [ "$DUMP_ONLY" = "1" ]; then
	cat "$work/payload.lf.sh"
	exit 0
fi

if [ "$PRINT_ONLY" = "1" ]; then
	echo "wsl-toolkit --instance $INSTANCE run \\"
	echo "  --image $IMAGE \\"
	echo "  --workspace $ROOT \\"
	echo "  --container-lifecycle ephemeral \\"
	echo "  --user 0:0 \\"
	printf '  %s \\\n' "$@"
	echo "  --input job.sh=$work/job.lf.sh \\"
	echo "  --script $work/payload.lf.sh \\"
	echo "  --timeout $TIMEOUT --tick 120s"
	exit 0
fi

# ⛔ Every job leaves a retained record, and `scripts/check-todo.py` fails
# while any is kept, with `wsl-toolkit: lane job still kept`. A neighbour's
# dead job therefore fails YOUR landing proof. The id it prints is 12 hex
# characters and `gc --job` needs the full 16; the short form exits 0 and
# removes NOTHING, which is a silent failure. So the job is collected by its
# full id, read from the `--json` answer, and the answer is parsed here.
out=$(wsl-toolkit --instance "$INSTANCE" run \
	--image "$IMAGE" \
	--workspace "$ROOT" \
	--container-lifecycle ephemeral \
	--user 0:0 \
	"$@" \
	--input "job.sh=$work/job.lf.sh" \
	--input "drop.sh=$work/drop.lf.sh" \
	--script "$work/payload.lf.sh" \
	--json \
	--timeout "$TIMEOUT" \
	--tick 120s 2>&1)
rc=$?

printf '%s\n' "$out"

job_id=$(printf '%s' "$out" | sed -n 's/.*"id"[[:space:]]*:[[:space:]]*"\(wtk-[0-9a-f]\{16\}\)".*/\1/p' | head -1)
if [ -n "$job_id" ]; then
	short=${job_id#wtk-}
	wsl-toolkit --instance "$INSTANCE" gc --job "$short" --apply >/dev/null 2>&1
fi

exit "$rc"
