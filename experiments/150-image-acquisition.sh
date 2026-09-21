#!/usr/bin/env bash
# Question: does the digest `podbox images` reports for a tag equal the digest
# `docker image inspect` reports for the same tag, and does a second pull cost
# nothing?
#
# This is TODO/milestones.md T-1102's acceptance, as a script, so it can be
# re-run rather than recalled. It also carries TODO/image.md T-0201's and
# T-0202's Prove commands:
#
#   1. `podbox pull <ref>` then `podbox images --format '{{.Digest}}' <ref>`
#      equals docker's `{{index .RepoDigests 0}}`;
#   2. a second pull reports every layer as already present and fetches nothing;
#   3. every blob in the store hashes to the name it is stored under;
#   4. a registry named with http:// is refused, and the refusal says so.
#
#   ./150-image-acquisition.sh
#
# ⚠ A MOVING TAG IS A RACE, AND IT IS HANDLED RATHER THAN IGNORED. `alpine:latest`
# can be republished between the two pulls, and the two digests would then
# differ for a reason that is not podbox's. Clause 1 therefore compares, and on
# a mismatch re-pulls BOTH and compares again, and reports which of the two it
# was. Set PODBOX_TEST_IMAGE to a digest-pinned reference to remove the race
# entirely.
#
# Exit: 0 every clause held, 1 one of them did not, 2 could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/image-acquisition.txt"
# ⭐ ghcr, AND NOT DOCKER HUB, AND THE REASON IS MEASURED. TODO/image.md T-0206
# says this clause cannot leave the Hub "because its whole question is whether
# podbox's digest equals the one `docker image inspect` reports". ⛔ That
# premise is wrong and was checked on 2026-09-09: docker pulls from ghcr.io
# perfectly well, and its `RepoDigests[0]` for this reference is the same value
# podbox records. The parity question needs a registry BOTH tools can reach, not
# the Hub specifically, and ghcr has no anonymous pull quota that a third party
# can exhaust on this project's behalf.
REFERENCE="${PODBOX_TEST_IMAGE:-ghcr.io/pkgforge-dev/archlinux:latest}"
# ⭐ Native execution on a Linux lane; staged inside the driver elsewhere,
# because an ELF built here does not execute there. The lane's scratch
# lives under the checkout on a non-native lane: mount sources must be
# Windows paths there (245's rule), and /tmp/... names nothing.
case "$(uname -s)" in
Linux) NATIVE=1; WORK="$(mktemp -d)"; STORE="$WORK/store" ;;
*)     NATIVE=0; WORK="$REPO/experiments/.sweep150-work"; rm -rf "$WORK"; mkdir -p "$WORK/w" || exit 2; STORE="$WORK/w/store" ;;
esac
trap 'rm -rf "$WORK"' EXIT INT TERM
export PODBOX_STORE="$STORE"

# The engine is `experiments/lib/engine.sh`: a docker daemon where one
# answers, else host podman. What each clause asserts is unchanged.
# shellcheck source=lib/engine.sh
. "$HERE/lib/engine.sh"
export ENGINE_REPO ENGINE_WORK
ENGINE_REPO="$REPO"
ENGINE_WORK="$WORK"
if engine_pick; then HAVE_ENGINE=1; else HAVE_ENGINE=0; fi
[ "$HAVE_ENGINE" -eq 1 ] || { echo "SKIP: no engine (a docker daemon or host podman)" >&2; exit 2; }

# The driver: the M5 debian row, pinned. It hosts the staged podbox binary
# for every `$BIN` call on a non-native lane.
DRIVER='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
STAGE="$WORK/stage"
if [ "$NATIVE" -eq 0 ]; then
	[ -r "$BIN" ] || {
		echo "SKIP: $BIN is not readable (set PODBOX_BIN to a guest build artifact)" >&2
		exit 2
	}
	eng_pull "$DRIVER" || exit 2
	mkdir -p "$STAGE" "$WORK/w" || exit 2
	cp "$BIN" "$STAGE/podbox"
	cat >"$WORK/pb.sh" <<'PB_EOF'
#!/bin/sh
# pb.sh - podbox in the driver with the shared store. Arguments are podbox's.
export PODBOX_STORE=/w/store
exec /pb "$@"
PB_EOF
	eng_mount "$STAGE/podbox" /pb \
		&& eng_mount "$WORK/w" /w rw \
		&& eng_mount "$WORK/pb.sh" /drv/pb.sh \
		|| { echo "SKIP: the driver inputs could not be staged" >&2; exit 2; }
fi

# pb TIMEOUT ARGS... - podbox with the sweep's store: directly where it
# executes, staged in the driver elsewhere. Timeouts stay per call site,
# as before.
pb() {
	_t="$1"; shift
	if [ "$NATIVE" -eq 1 ]; then
		timeout "$_t" "$BIN" "$@"
	else
		eng_run "$_t" "$DRIVER" "" -- /bin/sh /drv/pb.sh "$@"
	fi
}

if [ "$NATIVE" -eq 1 ]; then
	[ -x "$BIN" ] || {
		echo "SKIP: $BIN is not an executable. Build it:" >&2
		echo "      cargo build --release --target x86_64-unknown-linux-musl" >&2
		exit 2
	}
fi

echo "== conditions"
printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf 'host kernel       %s\n' "$(uname -r)"
printf 'podbox            %s\n' "$(pb 60 version)"
printf 'binary            %s bytes\n' "$(stat -c%s "$BIN")"
printf 'engine            %s\n' "$(engine_describe)"
printf 'reference         %s\n' "$REFERENCE"
printf 'store             a fresh directory under %s\n' "$(dirname "$WORK")"
echo

fail=0

# ⛔ Every exit code below is read from the process that produced it, never
# through a pipe and never from an && chain whose earlier link already
# succeeded. AGENTS.md absolute 8.
pull_podbox() {
	pb 600 pull "$1" >"$WORK/pull.out" 2>"$WORK/pull.err"
	return $?
}

echo "== 1. podbox's digest against docker's, for the same tag"
if ! pull_podbox "$REFERENCE"; then
	echo "  SKIP: podbox pull failed; the registry may be unreachable" >&2
	sed 's/^/    /' "$WORK/pull.err" >&2
	exit 2
fi
sed 's/^/    /' "$WORK/pull.out"

podbox_digest="$(pb 60 images --format '{{.Digest}}' "$REFERENCE")"
podbox_rc=$?
[ "$podbox_rc" -eq 0 ] || { echo "SKIP: podbox images exited $podbox_rc" >&2; exit 2; }

if ! eng_pull --allow-tag "$REFERENCE" >/dev/null 2>"$WORK/engine.err"; then
	echo "  SKIP: the engine pull failed; the registry may be unreachable" >&2
	sed 's/^/    /' "$WORK/engine.err" >&2
	exit 2
fi
engine_digest="$(eng_image_inspect "$REFERENCE" '{{index .RepoDigests 0}}' | cut -d@ -f2)"

printf '  podbox  %s\n' "$podbox_digest"
printf '  %-7s %s\n' "$ENGINE_NAME" "$engine_digest"
race="no"
if [ "$podbox_digest" != "$engine_digest" ]; then
	# ⚠ One re-run, to separate a moving tag from a wrong digest. A tag that
	# moved between the two pulls agrees on the second round; a wrong digest
	# does not.
	echo "  ⚠ they differ. Re-pulling both, to tell a moved tag from a wrong digest"
	pull_podbox "$REFERENCE"
	eng_pull --allow-tag "$REFERENCE" >/dev/null 2>&1
	podbox_digest="$(pb 60 images --format '{{.Digest}}' "$REFERENCE")"
	engine_digest="$(eng_image_inspect "$REFERENCE" '{{index .RepoDigests 0}}' | cut -d@ -f2)"
	printf '  podbox  %s\n' "$podbox_digest"
	printf '  %-7s %s\n' "$ENGINE_NAME" "$engine_digest"
	if [ "$podbox_digest" = "$engine_digest" ]; then
		echo "  ⚠ RECORDED: the tag moved between the first two pulls. They agree now."
		race="yes"
	else
		echo "  FAIL: the digests disagree on a second round, so this is not a race"
		fail=1
	fi
fi
[ "$podbox_digest" = "$engine_digest" ] || fail=1

echo
echo "== 2. a second pull fetches nothing"
pull_podbox "$REFERENCE"
second_rc=$?
sed 's/^/    /' "$WORK/pull.out"
[ "$second_rc" -eq 0 ] || { echo "  FAIL: the second pull exited $second_rc"; fail=1; }
if grep -qi 'already' "$WORK/pull.out"; then
	echo "  ok      every layer was already present"
else
	echo "  FAIL: the second pull did not report a layer as already present"
	fail=1
fi
if grep -q 'Pull complete' "$WORK/pull.out"; then
	echo "  FAIL: the second pull fetched a layer"
	fail=1
fi

echo
echo "== 3. every stored blob hashes to the name it is stored under"
# ⛔ The store's own claim, checked against the bytes. TODO/image.md T-0202
# verifies as it writes; this asserts the result on disk rather than trusting
# that it did.
mismatched=0
counted=0
for blob in "$STORE"/blobs/sha256/*; do
	[ -f "$blob" ] || continue
	counted=$((counted + 1))
	want="$(basename "$blob")"
	got="$(sha256sum "$blob" | cut -d' ' -f1)"
	if [ "$want" != "$got" ]; then
		printf '  MISMATCH %s is stored as %s\n' "$got" "$want"
		mismatched=$((mismatched + 1))
	fi
done
printf '  %d blob(s), %d mismatched\n' "$counted" "$mismatched"
[ "$counted" -gt 0 ] || { echo "  FAIL: the store holds no blobs after a pull"; fail=1; }
[ "$mismatched" -eq 0 ] || fail=1

echo
echo "== 4. a plain-HTTP registry is refused rather than downgraded"
# ⛔ TODO/image.md T-0201. tcp/80 egress is broken on the runtime podbox
# targets, so a fallback hangs instead of failing. `timeout` is the proof that
# it did not hang: a downgrade would sit here until the timeout fired.
pb 30 pull "http://registry.invalid/library/archlinux:latest" \
	>"$WORK/http.out" 2>"$WORK/http.err"
http_rc=$?
printf '  exit %s\n' "$http_rc"
sed 's/^/    /' "$WORK/http.err"
if [ "$http_rc" -eq 124 ]; then
	echo "  FAIL: it hung, which is the failure mode this refusal exists to prevent"
	fail=1
elif [ "$http_rc" -eq 0 ]; then
	echo "  FAIL: it accepted an http:// reference"
	fail=1
elif grep -qi 'HTTPS only' "$WORK/http.err"; then
	echo "  ok      refused by name"
else
	echo "  FAIL: it refused without saying that podbox is HTTPS only"
	fail=1
fi

{
	printf '# podbox image acquisition against the engine, TODO/milestones.md T-1102\n'
	printf '# taken %s on kernel %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(uname -r)"
	printf '# TODO/image.md T-0201 and T-0202 carry clauses 2 to 4.\n'
	printf 'reference         %s\n' "$REFERENCE"
	printf 'podbox_digest     %s\n' "$podbox_digest"
	printf 'engine_digest     %s\n' "$engine_digest"
	printf 'digests_match     %s\n' \
		"$([ "$podbox_digest" = "$engine_digest" ] && echo yes || echo no)"
	printf 'tag_moved         %s\n' "$race"
	printf 'blobs_stored      %s\n' "$counted"
	printf 'blobs_mismatched  %s\n' "$mismatched"
	# ⛔ NO `|| echo 0` HERE. `grep -c` PRINTS 0 and EXITS 1 on zero matches, so
	# a fallback beside it fires next to the real value and the record gets two
	# lines where it wants one. Measured here on 2026-09-08, and it is the trap
	# AGENTS.md names twice.
	printf 'second_pull_fetched %s\n' \
		"$(grep -c 'Pull complete' "$WORK/pull.out" 2>/dev/null)"
	printf 'http_refused_rc   %s\n' "$http_rc"
	echo
	echo '## the second pull, verbatim'
	sed 's/^/  /' "$WORK/pull.out"
	echo
	echo '## the http:// refusal, verbatim'
	sed 's/^/  /' "$WORK/http.err"
} > "$OUT"
echo
echo "  written to $OUT"

if [ -x "$REPO/scripts/common/result-diff.sh" ]; then
	echo
	echo "== against the committed reading"
	"$REPO/scripts/common/result-diff.sh" "$OUT" || true
fi

[ "$fail" -eq 0 ] || exit 1
exit 0
