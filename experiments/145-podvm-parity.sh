#!/usr/bin/env bash
# Question: do `podbox --help` and `podvm --help` list one set of verbs, with
# every tier row driven from the flag rather than read from the table?
#
# TODO/podvm.md T-1302 (drives the tier flag and the podvm name through the
# shipped binary).
#
# Clauses over the binary itself:
#   0. the table carries the tier rows and the podvm row, or nothing below
#      can be driven;
#   1. both help texts list one set of verbs with the tier paragraph;
#   2. the flag routes: machine refuses naming the legs, a third value is a
#      flag error, emulator arguments ride only with the machine tier;
#   3. the name routes: podvm defaults to machine, and podvm with chroot
#      states the override and proceeds past the tier;
#   4. exec mirrors run, and a verb the tier means nothing to refuses the flag
#      as unlisted.
#
# Runs on Linux, native or in a job container: the binary executes
# directly, and nothing here pulls an image. On a Windows host run it
# inside the base with PODBOX_BIN set to a guest build artifact.
#
# ⚠ `set -u` and no `pipefail`: this script also runs under `sh` in a
# Linux job container, where `sh` is dash.
#
# Exit: 0 every row green, 1 a row disagreed, 2 the binary, jq or the table
# could not run.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/podvm-parity.txt"
WORK="$REPO/experiments/.sweep145-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

fail=0
fails=0
driven=0
say() { printf '%s\n' "$*" >>"$WORK/report"; }
pass() { driven=$((driven + 1)); say "  ok: $1"; }
miss() { fail=1; fails=$((fails + 1)); say "  FAIL: $1"; }

[ -x "$BIN" ] || {
	echo "SKIP: $BIN is not an executable. Build it:" >&2
	echo "      cargo build --release --target x86_64-unknown-linux-musl" >&2
	exit 2
}
command -v jq >/dev/null 2>&1 || { echo "SKIP: jq is not on PATH" >&2; exit 2; }

# shellcheck source=../scripts/common/exit-codes.sh
. "$REPO/scripts/common/exit-codes.sh"
podbox_exit_codes "$BIN" || { echo "SKIP: cannot read podbox's exit-code table" >&2; exit 2; }

# `podvm` is the binary under another name: a symlink beside it, so argv[0]
# is what the multicall dispatches on.
ln -sf "$BIN" "$WORK/podvm" || exit 2
PVM="$WORK/podvm"

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -r)"
	printf 'podbox            %s\n' "$("$BIN" version)"
	printf 'jq                %s\n' "$(jq --version)"
	echo
} >"$WORK/report"

# The table, read out of the binary. Every clause starts here, so a row
# the binary does not publish is a row this script cannot drive.
"$BIN" system info --format '{{json .Parity}}' >"$WORK/parity.json" 2>"$WORK/e0"
[ "$?" -eq 0 ] || { echo "SKIP: the parity table did not print" >&2; exit 2; }

say "== 0. the tier rows are data"
for row in "run --podbox-tier" "run --podbox-qemu-arg" "exec --podbox-tier" "exec --podbox-qemu-arg" "podvm -"; do
	set -- $row
	if jq -e --arg v "$1" --arg f "$2" \
		'.[] | select(.verb == $v and ((.flag // "-") | startswith($f)))' \
		"$WORK/parity.json" >/dev/null; then
		pass "table carries $1 $2"
	else
		miss "table has no $1 $2 row"
	fi
done

say ""
say "== 1. both help texts list one set of verbs"
"$BIN" --help >"$WORK/help-podbox" 2>&1
"$PVM" --help >"$WORK/help-podvm" 2>&1
if cmp -s "$WORK/help-podbox" "$WORK/help-podvm"; then
	pass "podbox --help and podvm --help are the same text"
else
	miss "the two help texts differ"
fi
for word in "run" "exec" "podvm" "--podbox-tier"; do
	if grep -qF -- "$word" "$WORK/help-podvm"; then
		pass "podvm --help names $word"
	else
		miss "podvm --help never names $word"
	fi
done

# run VERB ARGS...: the words, the code, and which of two markers the
# stderr carries. The flag-error code is also the runtime code
# (TODO/cli.md T-0802), so the number alone proves nothing. Every call is
# bounded: a refusal that hangs is a hang with a green label on it.
run() {
	R_OUT="$(timeout 120 "$BIN" "$@" 2>&1)"; R_RC=$?
}
pvm() {
	R_OUT="$(timeout 120 "$PVM" "$@" 2>&1)"; R_RC=$?
}
refused_tier() { printf '%s' "$R_OUT" | grep -q "machine tier refused"; }
flag_error() { printf '%s' "$R_OUT" | grep -q "takes machine or chroot"; }
needs_machine() { printf '%s' "$R_OUT" | grep -q "needs --podbox-tier=machine"; }

say ""
say "== 2. the flag routes"
run run --podbox-tier=machine never-pulled-145:tag /bin/true
if [ "$R_RC" -eq "$PODBOX_EXIT_RUNTIME_ERROR" ] && refused_tier; then
	pass "run --podbox-tier=machine refuses naming the legs"
else
	miss "run --podbox-tier=machine (rc=$R_RC): $R_OUT"
fi
run run --podbox-tier=sandbox never-pulled-145:tag /bin/true
if [ "$R_RC" -eq "$PODBOX_EXIT_FLAG_ERROR" ] && flag_error; then
	pass "run --podbox-tier=sandbox is a flag error"
else
	miss "run --podbox-tier=sandbox (rc=$R_RC): $R_OUT"
fi
run run --podbox-qemu-arg=--cpu,host never-pulled-145:tag /bin/true
if [ "$R_RC" -eq "$PODBOX_EXIT_FLAG_ERROR" ] && needs_machine; then
	pass "run --podbox-qemu-arg without the machine tier is a flag error"
else
	miss "run --podbox-qemu-arg without the tier (rc=$R_RC): $R_OUT"
fi
run run --podbox-tier=machine --podbox-qemu-arg=--cpu,host --podbox-qemu-arg "a b" never-pulled-145:tag /bin/true
if [ "$R_RC" -eq "$PODBOX_EXIT_RUNTIME_ERROR" ] && refused_tier; then
	pass "repeated qemu args with a space ride the machine tier to its refusal"
else
	miss "run --podbox-tier=machine with qemu args (rc=$R_RC): $R_OUT"
fi

say ""
say "== 3. the name routes"
pvm run never-pulled-145:tag /bin/true
if [ "$R_RC" -eq "$PODBOX_EXIT_RUNTIME_ERROR" ] && refused_tier \
	&& printf '%s' "$R_OUT" | grep -q "invoked as \`podvm\`"; then
	pass "podvm run defaults to the machine tier and says its name"
else
	miss "podvm run (rc=$R_RC): $R_OUT"
fi
pvm run --podbox-tier=chroot never-pulled-145:tag /bin/true
if printf '%s' "$R_OUT" | grep -q "wins over the podvm default" \
	&& ! refused_tier; then
	pass "podvm run --podbox-tier=chroot states the override and proceeds past the tier"
else
	miss "podvm run --podbox-tier=chroot (rc=$R_RC): $R_OUT"
fi

say ""
say "== 4. exec mirrors run, and other verbs refuse the flag as unlisted"
run exec --podbox-tier=machine never-pulled-145:tag /bin/true
if [ "$R_RC" -eq "$PODBOX_EXIT_RUNTIME_ERROR" ] && refused_tier; then
	pass "exec --podbox-tier=machine refuses naming the legs"
else
	miss "exec --podbox-tier=machine (rc=$R_RC): $R_OUT"
fi
pvm exec never-pulled-145:tag /bin/true
if [ "$R_RC" -eq "$PODBOX_EXIT_RUNTIME_ERROR" ] && refused_tier; then
	pass "podvm exec defaults to the machine tier"
else
	miss "podvm exec (rc=$R_RC): $R_OUT"
fi
run ps --podbox-tier=machine
if [ "$R_RC" -eq "$PODBOX_EXIT_FLAG_ERROR" ] \
	&& printf '%s' "$R_OUT" | grep -q "no row in the parity table"; then
	pass "ps --podbox-tier is refused as unlisted"
else
	miss "ps --podbox-tier (rc=$R_RC): $R_OUT"
fi

say ""
say "== counts: $driven driven, $fails mismatches"
cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"

[ "$fail" -eq 0 ] || exit 1
exit 0
