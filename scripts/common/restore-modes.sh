#!/usr/bin/env sh
# restore-modes.sh - put the executable bit back on a tree that arrived from a
# filesystem which cannot hold one.
#
# ⛔ WHY THIS EXISTS. NTFS carries no POSIX mode bit, so a checkout on Windows
# holds every file at 0644 and `core.fileMode` is false there. A copy of that
# checkout into Linux therefore arrives with every one of them unrunnable, and
# the shell refuses each
# with "Permission denied", and the first line of the failure names the script
# rather than the transfer. Measured on 2026-09-11 with
# `wsl-toolkit run --workspace .` from a Windows checkout.
#
# ⭐ THE GIT INDEX IS THE AUTHORITY. It records mode 100755 for exactly the
# files that are meant to run, on every platform, so the repair reads the index
# and sets nothing else. A `chmod -R +x` over a directory would make data files
# executable and nobody would notice.
#
#   sh scripts/common/restore-modes.sh          repair, and report
#   sh scripts/common/restore-modes.sh --check  report only, change nothing
#
# Exit: 0 nothing to do or the repair worked, 1 something could not be set,
#       2 it could not run.
set -u

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 2
ROOT=$(CDPATH= cd -- "$HERE/../.." && pwd) || exit 2
cd "$ROOT" || exit 2

CHECK=0
case "${1:-}" in
--check) CHECK=1 ;;
-h | --help)
	sed -n '2,22p' "$0" | sed 's/^# \{0,1\}//'
	exit 0
	;;
"") ;;
*)
	echo "restore-modes: unknown option $1" >&2
	exit 2
	;;
esac

# ⚠ A Windows host has no mode bit to read, so this script only makes sense
# where one exists. It says so rather than reporting a clean tree.
case "$(uname -s 2>/dev/null || echo unknown)" in
Linux | Darwin | *BSD) ;;
*)
	echo "restore-modes: this host has no POSIX mode bit to set. Nothing done."
	exit 2
	;;
esac

if ! command -v git >/dev/null 2>&1; then
	echo "restore-modes: git is not on PATH, so the index cannot be read." >&2
	echo "  ⛔ No fallback is attempted: a guess at which files run would set" >&2
	echo "     the bit on data, and that is worse than the failure it repairs." >&2
	exit 2
fi
if ! git rev-parse --git-dir >/dev/null 2>&1; then
	echo "restore-modes: no git index here, so the authority is absent." >&2
	echo "  ⚠ A workspace copied without .git cannot be repaired from the index." >&2
	exit 2
fi

# `git ls-files -s` prints the mode, the object, the stage and the path, and
# a tab separates the path. ⛔ The cut is on the tab alone, because a path
# may contain a space. `core.quotePath=false` stops git escaping a non-ASCII
# name into a quoted form that no `chmod` can open.
want=$(git -c core.quotePath=false ls-files -s | sed -n 's/^100755 [0-9a-f]* [0-9]*	//p')
if [ -z "$want" ]; then
	echo "restore-modes: the index records no executable file."
	exit 0
fi

missing=0
failed=0

# ⚠ `mktemp`, never a name built from the process id. A predictable name in
# a shared directory is a file that somebody else can place first.
work=$(mktemp) || {
	echo "restore-modes: could not create a work file." >&2
	exit 2
}
trap 'rm -f "$work"' EXIT HUP INT TERM

printf '%s\n' "$want" | while IFS= read -r f; do
	[ -n "$f" ] || continue
	[ -f "$f" ] || continue
	[ -x "$f" ] && continue
	printf '%s\n' "$f"
done >"$work"

# ⛔ `grep -c` exits 1 when it counts zero, which breaks an `&&` chain and
# reads as a failed count. Both counts ignore the status on purpose.
total=$(printf '%s\n' "$want" | grep -c . || true)
missing=$(grep -c . <"$work" || true)

if [ "$missing" -eq 0 ]; then
	echo "restore-modes: $total executable file(s) in the index, all already executable."
	exit 0
fi

if [ "$CHECK" -eq 1 ]; then
	echo "restore-modes: $missing of $total index-executable file(s) are NOT executable here."
	sed -n '1,10p' "$work" | sed 's/^/  /'
	[ "$missing" -gt 10 ] && echo "  ... and $((missing - 10)) more"
	exit 1
fi

while IFS= read -r f; do
	chmod +x -- "$f" || failed=$((failed + 1))
done <"$work"

if [ "$failed" -gt 0 ]; then
	echo "restore-modes: $failed of $missing file(s) could not be made executable." >&2
	exit 1
fi
echo "restore-modes: set the executable bit on $missing of $total file(s)."
exit 0
