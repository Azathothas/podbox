#!/usr/bin/env sh
# session-start.sh - the one command a session runs first.
#
# ⭐ IT ANSWERS FOUR QUESTIONS AND THEN STARTS THE ENVIRONMENT.
# Where am I, what time is it, what is installed, and which lane does this host
# use. Then it runs the setup for that lane and returns. It creates nothing
# that a second run would create again.
#
#   ./scripts/session-start.sh           report, then start the environment
#   ./scripts/session-start.sh --check   report only, change nothing
#   ./scripts/session-start.sh --quiet   the verdict and the lane alone
#
# ⛔ THE LANE IS THIS SCRIPT'S DECISION AND NOT THE SESSION'S. A session that
# picks the lane by hand runs the Debian bootstrap against an Arch base, or
# calls wsl.exe on a host where docs/containers.md forbids it.
#
# Exit: 0 the lane is ready, 1 something needs attention and is named,
#       2 it could not run.
set -u

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 2
ROOT=$(CDPATH= cd -- "$HERE/.." && pwd) || exit 2
cd "$ROOT" || exit 2

CHECK=0
QUIET=0
for a in "$@"; do
	case "$a" in
	--check) CHECK=1 ;;
	--quiet) QUIET=1 ;;
	-h | --help)
		sed -n '2,18p' "$0" | sed 's/^# \{0,1\}//'
		exit 0
		;;
	*)
		echo "session-start: unknown option $a" >&2
		exit 2
		;;
	esac
done

say() { [ "$QUIET" -eq 1 ] || printf '%s\n' "$*"; }
row() { [ "$QUIET" -eq 1 ] || printf '  %-14s %s\n' "$1" "$2"; }

problems=0
note_problem() {
	problems=$((problems + 1))
	printf '  ! %s\n' "$1" >&2
}

# ---------------------------------------------------------------- 1. when
#
# ⛔ Read from the machine, never typed. docs/methodology/sessions.md measures
# the session against this instant.
NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null) || NOW="-"
say "== when"
row "utc" "$NOW"
row "local" "$(date +%Y-%m-%dT%H:%M:%S%z 2>/dev/null || echo -)"

# ---------------------------------------------------------------- 2. where
UNAME_S=$(uname -s 2>/dev/null || echo unknown)
UNAME_R=$(uname -r 2>/dev/null || echo unknown)
UNAME_M=$(uname -m 2>/dev/null || echo unknown)

KIND=unknown
case "$UNAME_S" in
Linux)
	KIND=linux
	# ⚠ A container and a WSL guest are both Linux and neither is the host.
	# Each is named rather than folded into one, because the setup differs.
	if [ -f /.dockerenv ] || grep -qaE "docker|podman|containerd|libpod" /proc/1/cgroup 2>/dev/null; then
		KIND=container
	elif grep -qai "microsoft" /proc/sys/kernel/osrelease 2>/dev/null; then
		KIND=wsl-guest
	fi
	;;
MINGW* | MSYS* | CYGWIN*) KIND=windows ;;
Darwin) KIND=darwin ;;
esac

say ""
say "== where"
row "kind" "$KIND"
row "uname" "$UNAME_S $UNAME_R $UNAME_M"
row "cwd" "$ROOT"
row "user" "$(id -un 2>/dev/null || echo -), uid $(id -u 2>/dev/null || echo -)"

if [ ! -f "$ROOT/AGENTS.md" ] || [ ! -d "$ROOT/TODO" ]; then
	echo "session-start: this is not the podbox tree. AGENTS.md or TODO/ is missing." >&2
	exit 2
fi

# ---------------------------------------------------------------- 3. the tree
say ""
say "== the tree"
if command -v git >/dev/null 2>&1 && git rev-parse --git-dir >/dev/null 2>&1; then
	branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo -)
	row "branch" "$branch"
	row "head" "$(git log -1 --format="%h %s" 2>/dev/null || echo -)"
	# ⛔ grep -c exits 1 when it counts zero, which breaks an && chain. The
	# status is ignored here on purpose.
	row "dirty" "$(git status --porcelain 2>/dev/null | grep -c . || true) file(s)"
	gname=$(git config user.name 2>/dev/null || true)
	gmail=$(git config user.email 2>/dev/null || true)
	row "identity" "${gname:-UNSET} ${gmail:-UNSET}"
	# ⛔ docs/conventions/git.md section 1. The identity is the operator's and
	# is never invented. An unset one is named here rather than at the commit.
	if [ -z "$gname" ] || [ -z "$gmail" ]; then
		note_problem "the git identity is unset. Read it from the history, then set it in this repository."
	fi
	if [ "$branch" != "main" ]; then
		note_problem "not on main. TODO/RULES.md section 2 settles the branch and it is not open."
	fi
else
	note_problem "git is absent, or this directory is not a checkout"
fi

# ---------------------------------------------------------------- 4. tools
#
# ⚠ A NAME ON PATH IS NOT A WORKING PROGRAM. python3 on Windows resolves to a
# Store stub that exits without running anything, so each version is read by
# RUNNING the tool rather than by finding it.
say ""
say "== tools"
tool_version() {
	case "$1" in
	python3 | py) "$1" -c "import sys; print(sys.version.split()[0])" 2>/dev/null | head -1 ;;
	# ⚠ Three tools spell the question differently. Asking the wrong one
	# reports a working tool as one that answered nothing.
	zig) zig version 2>/dev/null | head -1 ;;
	go) go version 2>/dev/null | head -1 ;;
	wsl-toolkit) wsl-toolkit version 2>/dev/null | head -1 ;;
	*) "$1" --version 2>/dev/null | head -1 ;;
	esac
}
for t in git cargo rustc zig python3 py jq curl tar podman docker go scc codegraph wsl-toolkit; do
	if command -v "$t" >/dev/null 2>&1; then
		v=$(tool_version "$t")
		[ -n "$v" ] || v="on PATH and answered nothing"
		row "$t" "$v"
	else
		row "$t" "-"
	fi
done

# ---------------------------------------------------------------- 5. codegraph
#
# ⭐ AGENTS.md absolute 12: CodeGraph answers before grep does. The index is per
# machine and is not tracked, so it is built here and synced every session.
say ""
say "== codegraph"
if ! command -v codegraph >/dev/null 2>&1; then
	row "index" "codegraph is absent. grep is the fallback and it is slower."
elif [ "$CHECK" -eq 1 ]; then
	if [ -d .codegraph ]; then row "index" "present, and --check builds nothing"; else row "index" "absent, and --check builds nothing"; fi
elif [ -d .codegraph ]; then
	if codegraph sync . >/dev/null 2>&1; then row "index" "synced"; else row "index" "sync failed. Run codegraph sync"; fi
else
	if codegraph init . >/dev/null 2>&1; then row "index" "built"; else row "index" "init failed. Run codegraph init"; fi
fi

# ---------------------------------------------------------------- 6. the lane
say ""
say "== the lane"
LANE=none
case "$KIND" in
linux | container | wsl-guest) LANE=native ;;
windows) LANE=wsl-toolkit ;;
esac
row "lane" "$LANE"

case "$LANE" in
native)
	row "setup" "scripts/common/bootstrap-env.sh, started by scripts/dev.sh"
	if [ "$CHECK" -eq 1 ]; then
		"$ROOT/scripts/common/bootstrap-env.sh" --check
	else
		# ⛔ In the background on purpose. It compiles behind the reading, and
		# the reading needs no toolchain.
		"$ROOT/scripts/dev.sh" || note_problem "dev.sh did not start"
	fi
	;;
wsl-toolkit)
	# ⛔ docs/containers.md: never wsl.exe, and the instance is podbox.
	if ! command -v wsl-toolkit >/dev/null 2>&1; then
		note_problem "wsl-toolkit is not on PATH. docs/agent-tooling.md says where it lives."
	else
		row "instance" "podbox, which is the distribution wsl-toolkit-podbox"
		if [ "$CHECK" -eq 1 ]; then
			wsl-toolkit --instance podbox base status --probe ||
				note_problem "the base is not usable. Run this script without --check."
		else
			# ⚠ Idempotent. It says ALREADY EXISTS and creates nothing when the
			# distribution is already registered.
			wsl-toolkit --instance podbox base ensure --probe ||
				note_problem "base ensure failed. It builds from an OCI image, so it needs a container engine on this host: podman machine start"
		fi
		row "host half" "sh scripts/common/check-gate.sh --fast"
		row "linux half" "sh scripts/windows/run-in-base.sh JOB.sh"
	fi
	;;
*)
	note_problem "this host has no lane. docs/containers.md names the three that exist."
	;;
esac

# ---------------------------------------------------------------- 7. the reading
say ""
say "== read these, in this order"
say "  1. AGENTS.md           the router and the absolutes"
say "  2. TODO/PROGRESS.md    the state, the work order, the open questions"
say "  3. TODO/RESUME.md      what the last session left in flight"
say "  4. the TODO/ entry that your task names, in full"

say ""
if [ "$problems" -gt 0 ]; then
	say "== $problems thing(s) need attention. Each is named above."
	exit 1
fi
say "== ready. The session start instant is $NOW."
exit 0
