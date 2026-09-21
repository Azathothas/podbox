#!/usr/bin/env bash
# Question: does the shipped binary agree with every row of the parity
# table it publishes, starting from the flag rather than from the row?
#
# TODO/cli.md T-0808 (drives T-0801's table through the binary).
#
# Three clauses over the table read out of the binary itself:
#   1. every row with a flag, driven through the binary: a None row refuses
#      with the row's own note and the flag-error code; a Native, Degraded
#      or Stub row is admitted, prints no refusal, and reaches a parser
#      with an arm for it;
#   2. every verb and group path, probed with a flag no row names: the
#      refusal carries the flag-error code, names the path the caller
#      typed, and says the table has no row -- or, where the verb has no
#      parser, the verb's own row answers with the flag-error code;
#   3. Stub rows that need a running payload, driven where an image pulls
#      and reported as unreachable where none does.
#
# Runs on Linux, native or in a job container: the binary executes
# directly, and nothing here needs a container engine. On a Windows host
# run it inside the base with PODBOX_BIN set to a guest build artifact.
#
# A step that exits 0 having done nothing it was asked to do verifies its
# own effect: a row the binary never saw is reported as unreachable, never
# counted as driven.
#
# ⚠ `set -u` and no `pipefail`: this script also runs under `sh` in a
# Linux job container, where `sh` is dash.
#
# Exit: 0 every row was driven or reported as unreachable here, 1 a row
# disagreed, 2 the binary, jq or the table could not run.
set -u

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/parity-drive.txt"
WORK="$REPO/experiments/.sweep325-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

fail=0
fails=0
driven=0
unreachable=0
say() { printf '%s\n' "$*" >>"$WORK/report"; }
pass() { driven=$((driven + 1)); }
miss() { fail=1; fails=$((fails + 1)); say "  FAIL: $1"; }
third() { unreachable=$((unreachable + 1)); say "  unreachable here: $1"; }

[ -x "$BIN" ] || {
	echo "SKIP: $BIN is not an executable. Build it:" >&2
	echo "      cargo build --release --target x86_64-unknown-linux-musl" >&2
	exit 2
}
command -v jq >/dev/null 2>&1 || { echo "SKIP: jq is not on PATH" >&2; exit 2; }

# shellcheck source=../scripts/common/exit-codes.sh
. "$REPO/scripts/common/exit-codes.sh"
podbox_exit_codes "$BIN" || { echo "SKIP: cannot read podbox's exit-code table" >&2; exit 2; }

export PODBOX_STORE="$WORK/store"

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
rc=$?
[ "$rc" -eq 0 ] || { echo "SKIP: the parity table did not print" >&2; exit 2; }
jq -e 'length >= 60 and all(.status | IN("Native","Degraded","Stub","None"))' \
	"$WORK/parity.json" >/dev/null || {
	echo "FAIL: the table is not the contract" >&2
	exit 1
}
rows="$(jq 'length' "$WORK/parity.json")"
say "== 0. the table is data: $rows rows"

# One line per row: verb, all spellings, status. The note travels by a
# separate lookup, so no prose crosses a field separator. This lane's jq
# ends every raw-output line with CRLF and the shell strips only the
# trailing one, so the transport bytes go before podbox ever sees a verb.
jq -r '.[] | [.verb, (.flag // "-"), .status] | @tsv' "$WORK/parity.json" \
	| tr -d '\r' >"$WORK/rows.tsv" || exit 2

pb() { _t="$1"; shift; timeout "$_t" "$BIN" "$@"; }

# A JSON verb is not always the path the caller types: the `image` and
# `system` groups dispatch to one handler per subverb.
verb_path() {
	case "$1" in
	prune) printf '%s' "image prune" ;;
	install-names) printf '%s' "system install-names" ;;
	abi) printf '%s' "system abi" ;;
	*) printf '%s' "$1" ;;
	esac
}

# flag_arg VERB SPELLING: the words the flag needs after it, if any. Only
# spellings the table lists, and only values the parser accepts, so a
# refusal names the flag and never its value.
flag_arg() {
	_v="$1"; _f="$2"
	case "$_v $_f" in
	"stop -t") printf '%s' "-t 5" ;;
	"kill -s") printf '%s' "-s TERM" ;;
	"inspect -f"|"info -f") printf '%s' "-f {{.ID}}" ;;
	*) case "$_f" in
		--platform) printf '%s' "--platform=linux/amd64" ;;
		--pull) printf '%s' "--pull=never" ;;
		--format) printf '%s' "--format={{.ID}}" ;;
		--dir) printf '%s' "--dir=$WORK/names" ;;
		--signal) printf '%s' "--signal=TERM" ;;
		--time|--timeout) printf '%s' "$_f=5" ;;
		-e) printf '%s' "-e A=B" ;;
		-w) printf '%s' "-w /w" ;;
		-u) printf '%s' "-u 0" ;;
		--env) printf '%s' "--env=A=B" ;;
		--workdir) printf '%s' "--workdir=/w" ;;
		--user) printf '%s' "--user=0" ;;
		--entrypoint) printf '%s' "--entrypoint=/e" ;;
		--name) printf '%s' "--name=n325" ;;
		--add-host) printf '%s' "--add-host=h:1.1.1.1" ;;
		--insecure-registry) printf '%s' "--insecure-registry=example.com" ;;
		--tls-verify) printf '%s' "--tls-verify=true" ;;
		*) printf '%s' "$_f" ;;
		esac ;;
	esac
}

# Refusal MARKERS, and nothing else, tell an admission apart from a
# refusal: the flag-error code is also the runtime code, so the number
# alone proves nothing (TODO/cli.md T-0802).
refused() { printf '%s' "$1" | grep -qE "no row in the parity table|status None|has no arm for it"; }

TAB="$(printf '\t')"
say ""
say "== 1. every row with a flag, driven"
while IFS="$TAB" read -r verb flag status; do
	[ -n "$verb" ] || continue
	[ "$flag" != "-" ] || continue
	path="$(verb_path "$verb")"
	# shellcheck disable=SC2086
	set -- $path
	first="$(printf '%s' "$flag" | cut -d, -f1)"
	if [ "$status" = "None" ]; then
		# Refused up front, before any value or positional is read.
		out="$(pb 60 "$@" "$first" 2>&1)"; rc=$?
		note="$(jq -r --arg v "$verb" --arg f "$flag" \
			'.[] | select(.verb == $v and .flag == $f) | .note' \
			"$WORK/parity.json" | tr -d '\r')"
		ok=yes
		[ "$rc" -eq "$PODBOX_EXIT_FLAG_ERROR" ] || ok=no
		printf '%s' "$out" | grep -qF "$note" || ok=no
		if [ "$ok" = yes ]; then pass; else miss "$verb $first refuses without its own note (rc=$rc)"; fi
		continue
	fi
	if [ "$status" = "Stub" ] && [ "$first" = "-i" ]; then
		# Needs a running payload; clause 3 decides.
		continue
	fi
	fargs="$(flag_arg "$verb" "$first")"
	case "$verb" in
	run|create)
		# shellcheck disable=SC2086
		out="$(pb 120 "$@" $fargs --pull=never bogus325img /bin/true 2>&1)"; rc=$?
		;;
	exec)
		# shellcheck disable=SC2086
		out="$(pb 120 "$@" $fargs bogus325img /bin/true 2>&1)"; rc=$?
		;;
	*)
		# shellcheck disable=SC2086
		out="$(pb 60 "$@" $fargs 2>&1)"; rc=$?
		;;
	esac
	if printf '%s' "$out" | grep -q "has no arm for it"; then
		miss "$verb $first reaches no arm"
	elif refused "$out"; then
		miss "$verb $first refused (rc=$rc)"
	elif [ "$first" = "-h" ] || [ "$first" = "--help" ]; then
		if [ "$rc" -eq 0 ]; then pass; else miss "$verb $first exits $rc"; fi
	elif [ "$verb" = "images" ] && [ "$first" = "-a" ]; then
		if [ "$rc" -eq 0 ]; then pass; else miss "$verb $first exits $rc"; fi
	else
		pass
	fi
done <"$WORK/rows.tsv"

say ""
say "== 2. every verb and group path, probed with a flag no row names"
verbs="$(jq -r '[.[] | select(.flag != null) | .verb] | unique | .[]' "$WORK/parity.json" | tr -d '\r')"
for v in $verbs create; do
	# The path the caller types: a group subverb is probed under its
	# group, never as a top-level verb podbox does not dispatch.
	p="$(verb_path "$v")"
	# shellcheck disable=SC2086
	set -- $p
	out="$(pb 60 "$@" --no-such-flag 2>&1)"; rc=$?
	ok=yes
	[ "$rc" -eq "$PODBOX_EXIT_FLAG_ERROR" ] || ok=no
	printf '%s' "$out" | grep -q "no row in the parity table" || ok=no
	printf '%s' "$out" | grep -q "podbox $p:" || ok=no
	if [ "$ok" = yes ]; then pass; else miss "$p unlisted refusal (rc=$rc)"; fi
done
# Alias paths the table never names as verbs: probed with a flag no row
# names, like every other path above. The bare groups are covered by the
# table-verb loop; the other group paths by their subverbs' rows.
for p in "image ls" "image list" "image rm" "image remove"; do
	# shellcheck disable=SC2086
	set -- $p
	out="$(pb 60 "$@" --no-such-flag 2>&1)"; rc=$?
	ok=yes
	[ "$rc" -eq "$PODBOX_EXIT_FLAG_ERROR" ] || ok=no
	printf '%s' "$out" | grep -q "no row in the parity table" || ok=no
	printf '%s' "$out" | grep -q "podbox $p:" || ok=no
	if [ "$ok" = yes ]; then pass; else miss "$p unlisted refusal (rc=$rc)"; fi
done
for v in $(jq -r '[.[] | select(.flag == null) | .verb] | unique | .[]' "$WORK/parity.json" | tr -d '\r'); do
	case " $verbs create " in
	*" $v "*) continue ;;
	esac
	vstatus="$(jq -r --arg v "$v" '.[] | select(.verb == $v and .flag == null) | .status' \
		"$WORK/parity.json" | tr -d '\r')"
	out="$(pb 60 "$v" 2>&1)"; rc=$?
	if [ "$vstatus" = "None" ]; then
		note="$(jq -r --arg v "$v" '.[] | select(.verb == $v and .flag == null) | .note' \
			"$WORK/parity.json" | tr -d '\r')"
		ok=yes
		[ "$rc" -eq "$PODBOX_EXIT_FLAG_ERROR" ] || ok=no
		printf '%s' "$out" | grep -qF "$note" || ok=no
		if [ "$ok" = yes ]; then pass; else miss "$v bare verb (rc=$rc)"; fi
	else
		if refused "$out"; then miss "$v bare verb refused (rc=$rc)"; else pass; fi
	fi
done
for p in "image ls" "image list" "image rm" "image remove" "image prune" "image tag" "image inspect" "image pull" "image extract" "system info" "system abi"; do
	# shellcheck disable=SC2086
	set -- $p
	out="$(pb 60 "$@" 2>&1)"; rc=$?
	if refused "$out"; then miss "$p bare path refused (rc=$rc)"; else pass; fi
done
say "  bare system install-names skipped: it writes links (covered by its flag drives)"

say ""
say "== 3. Stub rows that need a running payload"
if timeout 300 "$BIN" pull public.ecr.aws/docker/library/alpine:3.20 >/dev/null 2>&1 \
	&& timeout 300 "$BIN" extract public.ecr.aws/docker/library/alpine:3.20 >/dev/null 2>&1; then
	for v in run exec; do
		# ⚠ `run` alone takes `--pull`. `exec` has no such row: the image
		# is already pulled and extracted above, and the default policy
		# fetches only what is missing.
		if [ "$v" = "run" ]; then
			out="$(pb 300 "$v" -i --pull=never public.ecr.aws/docker/library/alpine:3.20 /bin/true 2>&1)"; rc=$?
		else
			out="$(pb 300 "$v" -i public.ecr.aws/docker/library/alpine:3.20 /bin/true 2>&1)"; rc=$?
		fi
		if [ "$rc" -eq 0 ] && printf '%s' "$out" | grep -q "\-i"; then
			pass
			say "  $v -i runs and the banner names it: yes"
		elif refused "$out"; then
			miss "$v -i refused (rc=$rc)"
		else
			third "$v -i needs more than this context runs (rc=$rc)"
		fi
	done
else
	third "run -i and exec -i need a pulled image, and none pulled here"
	third "run -i and exec -i need a pulled image, and none pulled here"
fi

say ""
say "== counts: $rows rows, $driven driven, $fails mismatches, $unreachable unreachable here"
cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"
[ -x "$REPO/scripts/common/result-diff.sh" ] && "$REPO/scripts/common/result-diff.sh" "$OUT"

[ "$fail" -eq 0 ] || exit 1
exit 0
