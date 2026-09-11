#!/usr/bin/env sh
# 156-closure-records.sh - does every closed entry carry its recorded run?
#
# ⭐ THE QUESTION. TODO/RULES.md section 5 says an entry closes in place with
# its `Prove` command actually run and the output recorded underneath. Nothing
# in the gate asserts it, so this measures it.
#
# ⛔ THIS IS A MEASUREMENT, NOT A GATE CHECK. TODO/gate.md T-1208 is where it
# becomes one, and a check arrives with its plant in the same change
# (docs/methodology/gate.md). Running this changes nothing.
#
# ⚠ WHY IT EXISTS AT ALL. The first count of this was taken with
# `grep -c '**Done '` and it was wrong by thirty entries, because the trailing
# space missed every record written `**Done.**` with no date. A number that
# moves when you spell the pattern differently needs a script, not a reader.
#
#   sh experiments/156-closure-records.sh            report
#   sh experiments/156-closure-records.sh --list     report, and name every entry
#
# Inputs: TODO/*.md in this checkout, which is the pinned input. Tools: awk.
# Exit: 0 every closed entry carries a record · 1 at least one does not ·
#       2 could not run.
set -u

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 2
ROOT=$(CDPATH= cd -- "$HERE/.." && pwd) || exit 2
cd "$ROOT" || exit 2

LIST=0
case "${1:-}" in
--list) LIST=1 ;;
-h | --help)
	sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
	exit 0
	;;
"") ;;
*)
	echo "156-closure-records: unknown option $1" >&2
	exit 2
	;;
esac

[ -d "$ROOT/TODO" ] || {
	echo "156-closure-records: no TODO directory. This is not the podbox tree." >&2
	exit 2
}
command -v awk >/dev/null 2>&1 || {
	echo "156-closure-records: awk is absent." >&2
	exit 2
}

echo "## conditions"
echo "date: $(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || echo -)"
echo "host: $(uname -sr 2>/dev/null || echo -)"
echo "tree: $(git rev-parse --short HEAD 2>/dev/null || echo -) $(git status --porcelain 2>/dev/null | grep -c . || true) dirty file(s)"
echo "subject: closure records in TODO/"
echo

# ⛔ The record files are excluded by name. INDEX.md, PROGRESS.md and the rest
# carry rows and prose, never entries, and a `### T-` in one of them would be a
# different defect that check-todo.py already owns.
FILES=$(ls "$ROOT"/TODO/*.md | grep -vE "/(INDEX|PROGRESS|RULES|RESUME|reference-map)\.md$")
# shellcheck disable=SC2086
awk -v list="$LIST" '
function flush() {
  if (id == "") return
  total++
  if (status == "done") {
    done_n++
    if (tail_has_text) {
      if (tail_has_bold) { bold++ } else { prose++ }
      recorded++
    } else {
      missing[++miss_n] = id "  " file "  " title
    }
  }
  id = ""; status = ""; title = ""; after_prove = 0
  tail_has_text = 0; tail_has_bold = 0
}
FNR == 1 { file = FILENAME; sub(/.*[\/\\]/, "", file) }
/^### T-[0-9][0-9][0-9][0-9] / {
  flush()
  id = $2; title = $0; sub(/^### T-[0-9]+ /, "", title)
  next
}
id == "" { next }
/^Status:[ \t]+/ { s = $0; sub(/^Status:[ \t]+/, "", s); split(s, a, " "); status = a[1]; next }
/^Prove:/ { after_prove = 1; next }
after_prove == 1 {
  # ⚠ A record is text after the Prove field that is not the entry separator
  # and not a blank line. The continuation lines of a wrapped Prove field are
  # indented, so they are skipped: they are the command, not its output.
  if ($0 ~ /^---[ \t]*$/) next
  if ($0 ~ /^[ \t]*$/) next
  if ($0 ~ /^[ \t]+/) next
  tail_has_text = 1
  if ($0 ~ /\*\*Done/) tail_has_bold = 1
}
END {
  flush()
  printf "entries                       : %d\n", total
  printf "closed                        : %d\n", done_n
  printf "closed with a record          : %d\n", recorded
  printf "  the bold Done shape         : %d\n", bold
  printf "  recorded as prose instead   : %d\n", prose
  printf "closed with NO record         : %d\n", miss_n
  if (miss_n > 0) {
    printf "\n## closed and never recorded\n"
    for (i = 1; i <= miss_n; i++) printf "  %s\n", missing[i]
  }
  if (list == 1 && prose > 0) {
    printf "\n## note: two shapes are in use, and TODO/gate.md T-1208 settles which\n"
  }
  exit (miss_n > 0 ? 1 : 0)
}
' $FILES
rc=$?

echo
echo "## verdict"
if [ "$rc" -eq 0 ]; then
	echo "every closed entry carries its recorded run"
else
	echo "at least one closed entry is marked done with nothing recorded under its Prove"
	echo "TODO/RULES.md section 5 is the rule; TODO/gate.md T-1208 is the check that would hold it"
fi
exit "$rc"
