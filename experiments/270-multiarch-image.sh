#!/usr/bin/env bash
# Question: does podbox acquire the platform it was ASKED for, and can one
# store hold two architectures of one tag at the same time?
#
# TODO/image.md T-0212. Until 2026-09-09 `oci::ARCH` was the constant `"amd64"`,
# so podbox built for aarch64 would still have pulled an amd64 rootfs, and the
# store's key was (repository, tag) so a second platform DELETED the first.
#
# ⭐ THE REGISTRY IS ghcr.io/pkgforge-dev/archlinux AND THAT IS DELIBERATE.
# It publishes one tag across eight platforms, which is what this question
# needs, and ghcr has no anonymous pull quota, so this script cannot be turned
# red by somebody else's rate limit the way TODO/image.md T-0206 describes.
#
#   ./270-multiarch-image.sh
#
# Exit: 0 every clause held, 1 one of them did not, 2 could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="$REPO/experiments/results/multiarch-image.txt"
# ⭐ Native execution on a Linux lane; staged inside the driver elsewhere,
# because an ELF built here does not execute there. The lane's scratch
# lives under the checkout on a non-native lane: mount sources must be
# Windows paths there (245's rule), and /tmp/... names nothing.
case "$(uname -s)" in
Linux) NATIVE=1; WORK="$(mktemp -d)"; PSTORE="$WORK/store" ;;
*)     NATIVE=0; WORK="$REPO/experiments/.sweep270-work"; rm -rf "$WORK"; mkdir -p "$WORK/w" || exit 2; PSTORE="/v/store" ;;
esac
trap 'eng_cleanup; rm -rf "$WORK"' EXIT INT TERM
export PODBOX_STORE="$WORK/store"

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
	mkdir -p "$STAGE" || exit 2
	cp "$BIN" "$STAGE/podbox"
	eng_mount "$STAGE/podbox" /pb \
		&& eng_mount "$WORK/w" /w rw \
		&& eng_volmount "x270-$$" /v rw \
		|| { echo "SKIP: the driver inputs could not be staged" >&2; exit 2; }
	eng_pb "$WORK/pb-shim" 120 "$DRIVER" || exit 2
	BIN_RUN="$WORK/pb-shim"
else
	BIN_RUN="$BIN"
fi

# pb TIMEOUT ARGS... - podbox with the sweep's store: directly where it
# executes, staged in the driver elsewhere. Timeouts stay per call site,
# as before.
pb() {
	_t="$1"; shift
	if [ "$NATIVE" -eq 1 ]; then
		timeout "$_t" "$BIN" "$@"
	else
		eng_pbrun "$_t" "$DRIVER" "$PSTORE" -- "$@"
	fi
}

# ⚠ Pinned by tag rather than by digest, because the question is about an INDEX
# and a digest names one manifest. The digest of what each clause resolved to is
# printed, so a re-run against a moved tag is visible rather than silent.
IMAGE="${PODBOX_MULTIARCH_IMAGE:-ghcr.io/pkgforge-dev/archlinux:latest}"

if [ "$NATIVE" -eq 1 ]; then
	[ -x "$BIN" ] || {
		echo "SKIP: $BIN is not an executable. Build it:" >&2
		echo "      cargo build --release --target x86_64-unknown-linux-musl" >&2
		exit 2
	}
fi
command -v file >/dev/null 2>&1 || { echo "SKIP: file(1) is not on PATH" >&2; exit 2; }

# ⭐ The exit codes are DATA, read out of the binary rather than written here.
# TODO/cli.md T-0802 and scripts/common/exit-codes.sh: six clauses across four
# experiments had `[ "$rc" -eq 2 ]` in them and all went red at once the day
# docker's codes were measured.
. "$REPO/scripts/common/exit-codes.sh"
# Read out of the binary; on a lane where it does not execute, the
# generated shim runs that one call staged in the driver.
podbox_exit_codes "$BIN_RUN" || {
	echo "SKIP: cannot read podbox's exit-code table; is jq installed and the binary built?" >&2
	exit 2
}

export PODBOX_STORE="$WORK/store"
# ⛔ Unset, so a caller's default cannot decide what this measures.
unset PODBOX_DEFAULT_PLATFORM DOCKER_DEFAULT_PLATFORM

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -r)"
	printf 'host arch         %s\n' "$(uname -m)"
	printf 'podbox            %s\n' "$(pb 60 version)"
	printf 'image             %s\n' "$IMAGE"
	# ⛔ No absolute path here: on a non-native lane the scratch lives
	# under the checkout, and a checkout path names the operator's home,
	# which `check-no-secrets.sh --public` refuses. The store's freshness
	# is the fact; its address is not.
	if [ "$NATIVE" -eq 1 ]; then
		printf 'store             a fresh directory under %s\n' "$(dirname "$WORK")"
	else
		printf 'store             a fresh container-local volume\n'
	fi
	echo
} >"$WORK/report"

fail=0
skipped=0

# --------------------------------------------------------------------- 1
echo "== 1. no --platform pulls the platform podbox was BUILT for" >>"$WORK/report"
if ! pb 900 pull "$IMAGE" >"$WORK/p1.out" 2>"$WORK/p1.err"; then
	printf '  SKIP: the pull did not complete: %s\n' "$(tail -1 "$WORK/p1.err")" >>"$WORK/report"
	cat "$WORK/report"
	exit 2
fi
host_plat="$(pb 60 images --format '{{.Platform}}' "$IMAGE" 2>/dev/null | head -1)"
printf '  asked for         nothing\n' >>"$WORK/report"
printf '  got               %s\n' "$host_plat" >>"$WORK/report"
case "$(uname -m)" in
x86_64) expect=linux/amd64 ;;
aarch64) expect=linux/arm64 ;;
*) expect="" ;;
esac
if [ -n "$expect" ] && [ "$host_plat" != "$expect" ]; then
	printf '  FAIL: this host is %s, so the default should be %s\n' "$(uname -m)" "$expect" >>"$WORK/report"
	fail=1
fi

# --------------------------------------------------------------------- 2
echo >>"$WORK/report"
echo "== 2. --platform takes a DIFFERENT manifest out of the same index" >>"$WORK/report"
# ⚠ The foreign platform is chosen to be the one this host is not.
foreign=linux/arm64
[ "$host_plat" = "linux/arm64" ] && foreign=linux/amd64
if ! pb 900 pull --platform "$foreign" "$IMAGE" >"$WORK/p2.out" 2>"$WORK/p2.err"; then
	printf '  SKIP: the %s pull did not complete: %s\n' "$foreign" "$(tail -1 "$WORK/p2.err")" >>"$WORK/report"
	skipped=1
else
	# ⭐ The tag's digest is the INDEX digest and is the same for both, which is
	# the parity T-0202 is accepted on. What differs is the image ID, which is
	# the config digest of the platform-specific manifest.
	printf '  index digest      %s\n' "$(sed -n 's/^Digest: //p' "$WORK/p2.out")" >>"$WORK/report"
	mapfile -t rows < <(pb 60 images --format '{{.ID}} {{.Platform}}' "$IMAGE" 2>/dev/null | sort -k2)
	printf '  the store now holds %d record(s) for one tag:\n' "${#rows[@]}" >>"$WORK/report"
	printf '    %s\n' "${rows[@]}" >>"$WORK/report"
	if [ "${#rows[@]}" -ne 2 ]; then
		printf '  FAIL: two platforms of one tag did not both survive. A store\n' >>"$WORK/report"
		printf '        keyed without the platform loses the first pull silently.\n' >>"$WORK/report"
		fail=1
	fi
	ids="$(printf '%s\n' "${rows[@]}" | awk '{print $1}' | sort -u | wc -l)"
	if [ "$ids" -ne 2 ]; then
		printf '  FAIL: both records carry the same image ID, so they are one image\n' >>"$WORK/report"
		fail=1
	fi
fi

# --------------------------------------------------------------------- 3
echo >>"$WORK/report"
echo "== 3. the extracted rootfs really is that architecture" >>"$WORK/report"
# ⛔ The clause that makes the two above worth anything. A record can say
# `linux/arm64` and hold amd64 bytes; this reads the ELF header of a binary
# inside the extracted tree and asks the machine, not the metadata.
for want in "$host_plat" "$foreign"; do
	id="$(pb 60 images --format '{{.ID}} {{.Platform}}' "$IMAGE" 2>/dev/null | awk -v p="$want" '$2==p{print $1; exit}')"
	if [ -z "$id" ]; then
		printf '  SKIP: no record for %s to extract\n' "$want" >>"$WORK/report"
		skipped=1
		continue
	fi
	pb 1800 extract "$id" >"$WORK/e.out" 2>"$WORK/e.err"
	rc=$?
	if [ "$rc" -ne 0 ]; then
		printf '  SKIP: extract of %s exited %d: %s\n' "$want" "$rc" "$(tail -1 "$WORK/e.err")" >>"$WORK/report"
		skipped=1
		continue
	fi
	if [ "$NATIVE" -eq 1 ]; then
		root="$(pb 60 inspect --format '{{.RootfsPath}}' "$id" 2>/dev/null)"
		probe=""
		for cand in usr/bin/bash bin/sh usr/bin/busybox bin/busybox; do
			[ -f "$root/$cand" ] && { probe="$root/$cand"; break; }
		done
	else
		# ⭐ The store is container-local (archlinux cannot extract onto a
		# case-insensitive Windows scratch, 245's finding), so the bytes
		# travel out through the shared scratch: the first candidate that
		# copies and reads is the probe, exactly as the native loop picks
		# the first candidate that exists.
		root="$(pb 60 inspect --format '{{.RootfsPath}}' "$id" 2>/dev/null)"
		probe=""
		for cand in usr/bin/bash bin/sh usr/bin/busybox bin/busybox; do
			if eng_run 60 "$DRIVER" "" -- /bin/sh -c 'cat "$0" > /w/probe 2>/dev/null' "$root/$cand" \
				&& [ -s "$WORK/w/probe" ]; then
				probe="$WORK/w/probe"
				probename="$cand"
				break
			fi
		done
	fi
	if [ -z "$probe" ]; then
		printf '  SKIP: %s has no binary this clause knows how to read\n' "$want" >>"$WORK/report"
		skipped=1
		continue
	fi
	# ⚠ The ELF machine word is the SECOND comma-field, not the first: the
	# first is "ELF 64-bit LSB pie executable" for every architecture alike,
	# which is evidence of nothing. The reader has to be able to check this.
	machine="$(file -b "$probe" | cut -d, -f2 | sed 's/^ *//')"
	if [ "$NATIVE" -eq 1 ]; then
		printf '  %-14s %-12s (%s)\n' "$want" "$machine" "${probe#"$root"/}" >>"$WORK/report"
	else
		printf '  %-14s %-12s (%s)\n' "$want" "$machine" "$probename" >>"$WORK/report"
	fi
	# The ELF machine word `file` prints, for the platform we asked for.
	case "$want" in
	linux/arm64) need="aarch64" ;;
	linux/amd64) need="x86-64" ;;
	*) need="" ;;
	esac
	if [ -n "$need" ] && ! file -b "$probe" | grep -q "$need"; then
		printf '  FAIL: %s extracted a tree whose binaries are not %s\n' "$want" "$need" >>"$WORK/report"
		fail=1
	fi
done

# --------------------------------------------------------------------- 4
echo >>"$WORK/report"
echo "== 4. an index that offers nothing for the ask says what it does offer" >>"$WORK/report"
# ⛔ Never a bare 404. The refusal has to name the platforms available, or the
# caller cannot tell a typo from an image that was never built for them.
out="$(pb 300 pull --platform linux/nosucharch "$IMAGE" 2>&1)"
rc=$?
printf '  exit              %d\n' "$rc" >>"$WORK/report"
printf '  says              %s\n' "$(printf '%s' "$out" | tr -d '\n' | cut -c1-96)" >>"$WORK/report"
if [ "$rc" -eq 0 ]; then
	printf '  FAIL: a platform the index does not carry was accepted\n' >>"$WORK/report"
	fail=1
elif ! printf '%s' "$out" | grep -q 'offers'; then
	printf '  FAIL: the refusal does not name what the index does offer\n' >>"$WORK/report"
	fail=1
fi

# --------------------------------------------------------------------- 5
echo >>"$WORK/report"
echo "== 5. a malformed --platform is a USAGE error, before any network" >>"$WORK/report"
out="$(pb 120 pull --platform 'a/b/c/d' "$IMAGE" 2>&1)"
rc=$?
printf '  exit              %d (%s is a cli error, TODO/cli.md T-0802)\n' "$rc" "$PODBOX_EXIT_CLI_ERROR" >>"$WORK/report"
printf '  says              %s\n' "$(printf '%s' "$out" | tr -d '\n' | cut -c1-96)" >>"$WORK/report"
# ⛔ The flag PARSED: podbox's arg loop takes any string as the value and the
# verb refuses it, which is the `images --format '{{.Nope}}'` row of T-0802's
# table (verb refuses afterwards, 1), not the `--pull=bogus` row (parser owns
# the value domain, 125). The 125 expectation predates that measurement.
[ "$rc" -eq "$PODBOX_EXIT_CLI_ERROR" ] || { printf '  FAIL: expected the cli-error code %s\n' "$PODBOX_EXIT_CLI_ERROR" >>"$WORK/report"; fail=1; }

out="$(pb 120 pull --platform "$IMAGE" 2>&1)"
rc=$?
printf '  --platform with no value: exit %d\n' "$rc" >>"$WORK/report"
[ "$rc" -eq "$PODBOX_EXIT_CLI_ERROR" ] || { printf '  FAIL: a flag swallowing its image is not the cli-error code %s\n' "$PODBOX_EXIT_CLI_ERROR" >>"$WORK/report"; fail=1; }

cat "$WORK/report"
mkdir -p "$(dirname "$OUT")"
cp "$WORK/report" "$OUT"
echo
echo "written to ${OUT#"$REPO"/}"
[ -x "$REPO/scripts/common/result-diff.sh" ] && "$REPO/scripts/common/result-diff.sh" "$OUT"

[ "$fail" -eq 0 ] || exit 1
[ "$skipped" -eq 0 ] || exit 2
exit 0
