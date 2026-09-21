#!/usr/bin/env bash
# Question: is a guest whose configured memory crosses the per-file ceiling
# refused BEFORE it starts, with the ceiling in the message?
#
# TODO/podvm.md T-1305 (the fleet decision; the ceiling leg is T-1301's
# `prlimit(RLIMIT_FSIZE)` row, and the flag is `--podbox-mem`).
#
# The ceiling is RLIMIT_FSIZE, read live: it bounds a single file, and a
# guest memory image is a single file, so a guest above it dies from
# SIGXFSZ. The writable allowance RULES.md section 8 covers is a different
# resource and is not what this checks.
#
# Clauses over the shipped binary itself:
#   0. the table carries the mem rows, or nothing below can be driven;
#   1. a guest over a lowered ceiling is refused BEFORE it starts, with the
#      ceiling and the guest bytes in the message. The limit is lowered in
#      the driven child only (python3 setrlimit + exec), so the kernel value
#      is real and inherited, and the image was never pulled;
#   2. a guest under that same ceiling passes the check and reaches the
#      legs verdict, with no ceiling language;
#   3. a size that is not a size is a flag error, in both spellings, and an
#      overflow past 64 bits with it;
#   4. the flag outside the machine tier is refused as needs-machine, on run
#      and on exec;
#   5. exec mirrors run on the over-ceiling refusal.
#
# Runs on Linux, native or in a job container: the binary executes
# directly, and nothing here pulls an image. On a Windows host run it
# inside the base with PODBOX_BIN set to a guest build artifact.
#
# ⚠ `set -u` and no `pipefail`: this script also runs under `sh` in a
# Linux job container, where `sh` is dash. No `read -t`, no coproc, no
# arrays: bounded waits are `timeout` and `python3` + `exec`.
#
# Exit: 0 every clause green, 1 a clause failed, 2 a tool, the binary or the
# table could not run.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/podvm-fleet.txt"
WORK="$REPO/experiments/.sweep148-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

STORE="$WORK/store"
export PODBOX_STORE="$STORE"

# ⭐ The lowered ceiling, in bytes. 1 GiB: far above anything the refusal
# path itself writes, far below the 4 GiB over-ceiling guest.
CEIL_BYTES=1073741824
OVER_MEM="4G"
OVER_BYTES=4294967296
UNDER_MEM="1M"

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
for t in jq python3 timeout; do
	command -v "$t" >/dev/null 2>&1 || { echo "SKIP: $t is not on PATH" >&2; exit 2; }
done

# shellcheck source=../scripts/common/exit-codes.sh
. "$REPO/scripts/common/exit-codes.sh"
podbox_exit_codes "$BIN" || { echo "SKIP: cannot read podbox's exit-code table" >&2; exit 2; }

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -r)"
	printf 'podbox            %s\n' "$("$BIN" version)"
	printf 'jq                %s\n' "$(jq --version)"
	printf 'python3           %s\n' "$(python3 --version 2>&1)"
	printf 'natural ceiling   %s\n' "$(python3 -c 'import resource; print(resource.getrlimit(resource.RLIMIT_FSIZE))')"
	printf 'lowered ceiling   %s bytes\n' "$CEIL_BYTES"
	printf 'over guest        %s = %s bytes\n' "$OVER_MEM" "$OVER_BYTES"
	printf 'under guest       %s\n' "$UNDER_MEM"
	echo
} >"$WORK/report"

# The table, read out of the binary. Every clause starts here, so a row
# the binary does not publish is a row this script cannot drive.
"$BIN" system info --format '{{json .Parity}}' >"$WORK/parity.json" 2>"$WORK/e0"
[ "$?" -eq 0 ] || { echo "SKIP: the parity table did not print" >&2; exit 2; }

say "== 0. the mem rows are data"
for row in "run --podbox-mem" "exec --podbox-mem"; do
	set -- $row
	if jq -e --arg v "$1" --arg f "$2" \
		'.[] | select(.verb == $v and ((.flag // "-") | startswith($f)))' \
		"$WORK/parity.json" >/dev/null; then
		pass "table carries $1 $2"
	else
		miss "table has no $1 $2 row"
	fi
done

# Run the binary with RLIMIT_FSIZE lowered to $CEIL_BYTES in the child
# only: python3 sets soft and hard, asserts the take, and execs, so the
# kernel value the binary reads is real and inherited. The parent keeps
# its own limit, so the report can still be written.
limited() {
	CEIL="$CEIL_BYTES" timeout 120 python3 - "$BIN" "$@" <<'EOF'
import os, resource, sys
ceil = int(os.environ["CEIL"])
resource.setrlimit(resource.RLIMIT_FSIZE, (ceil, ceil))
assert resource.getrlimit(resource.RLIMIT_FSIZE)[0] == ceil
os.execv(sys.argv[1], sys.argv[1:])
EOF
}
run() {
	R_OUT="$(timeout 120 "$BIN" "$@" 2>&1)"; R_RC=$?
}
ceiling_msg() { printf '%s' "$R_OUT" | grep -q "RLIMIT_FSIZE ceiling $CEIL_BYTES bytes"; }
over_msg() { printf '%s' "$R_OUT" | grep -q "guest memory $OVER_BYTES bytes"; }
refused_tier() { printf '%s' "$R_OUT" | grep -q "machine tier refused"; }
holds_msg() { printf '%s' "$R_OUT" | grep -q "the machine tier holds"; }
flag_error() { printf '%s' "$R_OUT" | grep -q "takes a byte count"; }
needs_machine() { printf '%s' "$R_OUT" | grep -q "needs --podbox-tier=machine"; }

say ""
say "== 1. over the ceiling is refused before anything starts"
R_OUT="$(limited run --podbox-tier=machine --podbox-mem="$OVER_MEM" never-pulled-148:tag /bin/true 2>&1)"; R_RC=$?
if [ "$R_RC" -eq "$PODBOX_EXIT_RUNTIME_ERROR" ] && refused_tier && ceiling_msg && over_msg; then
	pass "run --podbox-tier=machine --podbox-mem=$OVER_MEM refused naming the ceiling ($CEIL_BYTES) and the guest ($OVER_BYTES)"
else
	miss "over-ceiling run (rc=$R_RC): $R_OUT"
fi

say ""
say "== 2. under the ceiling reaches the legs verdict with no ceiling language"
R_OUT="$(limited run --podbox-tier=machine --podbox-mem="$UNDER_MEM" never-pulled-148:tag /bin/true 2>&1)"; R_RC=$?
if [ "$R_RC" -eq "$PODBOX_EXIT_RUNTIME_ERROR" ] && ! ceiling_msg; then
	if refused_tier || holds_msg; then
		pass "run --podbox-tier=machine --podbox-mem=$UNDER_MEM passed the check to the legs verdict"
	else
		miss "under-ceiling run reached neither verdict (rc=$R_RC): $R_OUT"
	fi
else
	miss "under-ceiling run (rc=$R_RC): $R_OUT"
fi

say ""
say "== 3. what is not a size is a flag error"
run run --podbox-tier=machine --podbox-mem=bogus never-pulled-148:tag /bin/true
if [ "$R_RC" -eq "$PODBOX_EXIT_FLAG_ERROR" ] && flag_error; then
	pass "--podbox-mem=bogus is a flag error"
else
	miss "--podbox-mem=bogus (rc=$R_RC): $R_OUT"
fi
run run --podbox-tier=machine --podbox-mem 0 never-pulled-148:tag /bin/true
if [ "$R_RC" -eq "$PODBOX_EXIT_FLAG_ERROR" ] && flag_error; then
	pass "--podbox-mem 0 is a flag error"
else
	miss "--podbox-mem 0 (rc=$R_RC): $R_OUT"
fi
run run --podbox-tier=machine --podbox-mem=18446744073709551615T never-pulled-148:tag /bin/true
if [ "$R_RC" -eq "$PODBOX_EXIT_FLAG_ERROR" ] && flag_error; then
	pass "--podbox-mem past 64 bits is a flag error"
else
	miss "--podbox-mem overflow (rc=$R_RC): $R_OUT"
fi

say ""
say "== 4. the flag outside the machine tier is refused as needs-machine"
run run --podbox-mem=1G never-pulled-148:tag /bin/true
if [ "$R_RC" -eq "$PODBOX_EXIT_FLAG_ERROR" ] && needs_machine; then
	pass "run --podbox-mem without the machine tier is a flag error"
else
	miss "run --podbox-mem without the tier (rc=$R_RC): $R_OUT"
fi
run exec --podbox-mem=1G never-pulled-148:tag /bin/true
if [ "$R_RC" -eq "$PODBOX_EXIT_FLAG_ERROR" ] && needs_machine; then
	pass "exec --podbox-mem without the machine tier is a flag error"
else
	miss "exec --podbox-mem without the tier (rc=$R_RC): $R_OUT"
fi

say ""
say "== 5. exec mirrors run on the over-ceiling refusal"
R_OUT="$(limited exec --podbox-tier=machine --podbox-mem="$OVER_MEM" never-pulled-148:tag /bin/true 2>&1)"; R_RC=$?
if [ "$R_RC" -eq "$PODBOX_EXIT_RUNTIME_ERROR" ] && refused_tier && ceiling_msg && over_msg; then
	pass "exec --podbox-tier=machine --podbox-mem=$OVER_MEM refused naming the ceiling and the guest"
else
	miss "over-ceiling exec (rc=$R_RC): $R_OUT"
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
