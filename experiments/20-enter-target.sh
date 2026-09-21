#!/usr/bin/env bash
# Question: can a container supply the target runtime's kernel-visible shape  - 
# its mount topology, its partial ID map, its seccomp filter and its write
# policy - and which parts does this host refuse?
#
#   ./20-enter-target.sh                     interactive shell inside the reconstruction
#   ./20-enter-target.sh -- id               run one command inside it
#   ./20-enter-target.sh --stage ./podbox \
#       -- /workspace/podbox probe           stage your own binary and run it
#   ./20-enter-target.sh --raw -- sh         the container without the confinement,
#                                            for setting fixtures up
#
# --stage copies a file or directory into what becomes /workspace, which is one
# of the four writable paths inside. Repeatable. This is how you run something
# that is not part of this repository against the reconstructed runtime - which
# is the point of the script for anyone implementing against it.
#
# Exit: 0 the payload ran, its own code otherwise, 2 could not run.
set -euo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
# Sourced before anything that can fail, so the EXIT trap below never
# calls a function that does not exist yet. Naming the engine comes
# later, with the work directory it needs.
# shellcheck source=lib/engine.sh
. "$HERE/lib/engine.sh"
IMAGE="${TARGET_IMAGE:-container-research/target:1}"
# ⭐ The lane's scratch lives under the checkout on a non-native lane:
# mount sources must be Windows paths there (245's rule), and /tmp/...
# names nothing. STAGE is staged into the reconstruction read-write, so
# it sits under the work directory on every lane; TARGET_STAGE still
# overrides it. Fresh per run: the remove-first staging below already
# refuses to nest, and a fresh directory cannot carry a stale one.
case "$(uname -s)" in
Linux) NATIVE=1; WORK="$(mktemp -d)" ;;
*)     NATIVE=0; WORK="$REPO/experiments/.sweep20-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2 ;;
esac
STAGE="${TARGET_STAGE:-$WORK/stage}"
trap 'eng_cleanup; rm -rf "$WORK"' EXIT INT TERM

# The engine is `experiments/lib/engine.sh`: a docker daemon where one
# answers, else host podman. The reconstruction runs privileged through
# the helper's one loud escape, because mount(2) and pivot_root(2) build
# the topology this script exists to measure. What each run asserts is
# unchanged.
export ENGINE_REPO ENGINE_WORK
ENGINE_REPO="$REPO"
ENGINE_WORK="$WORK"
# ⛔ Silenced, not removed: this script's stdout IS the payload channel
# (130 captures it as the rung), and the pick line would ride along into
# it. The choice is still recorded, in the conditions block on stderr.
if engine_pick >/dev/null; then HAVE_ENGINE=1; else HAVE_ENGINE=0; fi
[ "$HAVE_ENGINE" -eq 1 ] || { echo "SKIP: no engine (a docker daemon or host podman)" >&2; exit 2; }

RAW=0
STAGE_IN=()
ARGS=()
while [ "$#" -gt 0 ]; do
	case "$1" in
	--raw) RAW=1; shift ;;
	--stage) STAGE_IN+=("${2:?--stage needs a path}"); shift 2 ;;
	--) shift; ARGS=("$@"); break ;;
	*) ARGS+=("$1"); shift ;;
	esac
done
[ "${#ARGS[@]}" -eq 0 ] && ARGS=(/bin/sh)

eng_image_inspect "$IMAGE" '{{.Id}}' >/dev/null 2>&1 || {
	echo "SKIP: $IMAGE not built. Run ./experiments/10-build-target-image.sh" >&2
	exit 2
}
# By ID, not by fixture tag: the tag is a local name by definition, and
# the helper only takes pinned references. The ID names the exact bytes
# the build produced.
IID="$(eng_image_inspect "$IMAGE" '{{.Id}}')"
IID="${IID#sha256:}"

# ------------------------------------------------------------------ staging
# The harness is built on the host and staged into what becomes /workspace,
# because inside the reconstruction /workspace is one of only four writable
# paths and the toolchain there must not be needed to bootstrap the tools that
# measure it.
#
# ⛔ The harness sources live in the CORPUS, not at the top of this tree. This
# script arrived seeded from `Azathothas/container-research`, where they sit at
# `verification/`, and the three `$REPO/verification/...` paths it carried do
# not exist here: podbox tracks that repository under `references/` instead.
# Every invocation therefore failed with `cd: no such file or directory`, which
# is a citation that does not resolve wearing a shell error message.
# `HARNESS_SRC` names the one place they are, and the check below says so
# rather than letting `cd` say it.
HARNESS_SRC="${HARNESS_SRC:-$REPO/references/Azathothas__container-research/tree/verification}"
command -v go >/dev/null 2>&1 || { echo "SKIP: go is required to build the harness" >&2; exit 2; }
for d in confine probe; do
	[ -d "$HARNESS_SRC/$d" ] || {
		echo "SKIP: $HARNESS_SRC/$d does not exist. Set HARNESS_SRC to the tree" >&2
		echo "      carrying verification/{confine,probe}." >&2
		exit 2
	}
done
mkdir -p "$STAGE/.harness"
# The harness runs INSIDE the reconstruction, so it is built for the
# image's architecture, never the host's: on a non-native lane host go
# targets windows and the build dies on Linux-only syscalls. Measured
# 2026-09-21. The image arch comes from the engine, which built it.
HARNESS_GOARCH="$(eng_image_inspect "$IMAGE" '{{.Architecture}}')"
( cd "$HARNESS_SRC/confine" && GOOS=linux GOARCH="$HARNESS_GOARCH" CGO_ENABLED=0 go build -o "$STAGE/.harness/confine" . )
( cd "$HARNESS_SRC/probe"   && GOOS=linux GOARCH="$HARNESS_GOARCH" CGO_ENABLED=0 go build -o "$STAGE/.harness/probe" . )
# The C probe only where host gcc targets the container: a non-native
# lane's gcc builds for the host, and a windows binary inside the
# reconstruction measures nothing. The shell probe above covers the rows.
if [ "$NATIVE" -eq 1 ] && command -v gcc >/dev/null 2>&1 && [ -f "$HARNESS_SRC/cprobe/cprobe.c" ]; then
	gcc -O2 -static -o "$STAGE/.harness/cprobe" "$HARNESS_SRC/cprobe/cprobe.c"
fi

# enter.sh runs as the last step of targetfs.sh, inside the pivoted root. It
# composes the three mechanisms and says which ones it actually got.
cat > "$STAGE/.harness/enter.sh" <<'ENTER'
#!/bin/sh
# Applies N, F and (where the kernel has Landlock) M, then execs the payload.
set -eu
H=/workspace/.harness

CONFINE_USERNS=1
CONFINE_MOUNTNS=1
CONFINE_MAP_HOSTID="${TARGET_HOST_UID:-1000}"
CONFINE_EXTRA_GROUP=42
CONFINE_SECCOMP=1
CONFINE_DENY_PROCESS_VM=1
export CONFINE_USERNS CONFINE_MOUNTNS CONFINE_MAP_HOSTID CONFINE_EXTRA_GROUP \
       CONFINE_SECCOMP CONFINE_DENY_PROCESS_VM

# M is optional because the kernel may not carry Landlock at all. Ask before
# applying, and say what happened either way: a mechanism that is silently
# absent is worse than one that is loudly missing.
if "$H/probe" check 'landlock_create_ruleset(VERSION)' 2>/dev/null | grep -q ' OK'; then
	CONFINE_LANDLOCK=/tmp:/dev/shm:/workspace:/state
	export CONFINE_LANDLOCK
	echo "target: N+F+M (landlock write-allowlist active)" >&2
else
	echo "target: N+F only - M: unavailable (no CONFIG_SECURITY_LANDLOCK on this kernel);" >&2
	echo "        writes outside /tmp /dev/shm /workspace /state will NOT be denied" >&2
fi

exec "$H/confine" "$@"
ENTER
chmod 0755 "$STAGE/.harness/enter.sh"

# Anything the caller asked to bring along. It lands at /workspace/<basename>.
#
# ⛔ THE DESTINATION IS REMOVED FIRST, AND THAT IS NOT TIDINESS. `$STAGE`
# survives between runs, and `cp -a src/store dest/store` NESTS when
# `dest/store` already exists: the second run produces `dest/store/store` and
# leaves the first run's `dest/store` in place. Measured on 2026-09-08 while
# staging a podbox store twice for TODO/probe.md T-0111: the second run read
# the FIRST run's cache file, served it, and the clause passed for a reason
# that had nothing to do with what it was testing. A file destination
# overwrites and hid this for as long as only files were staged.
for p in ${STAGE_IN+"${STAGE_IN[@]}"}; do
	[ -e "$p" ] || { echo "SKIP: --stage $p does not exist" >&2; exit 2; }
	rm -rf -- "${STAGE:?}/$(basename -- "$p")"
	cp -a -- "$p" "$STAGE/$(basename -- "$p")"
done

# The uid-1000-owned-directory fixture: the census cannot build it itself,
# because chown to an unmapped id is exactly what the runtime denies.
mkdir -p "$STAGE/.fixtures/squash-probe"
chown -R 1000:1000 "$STAGE/.fixtures" 2>/dev/null || true
chown -R 1000:1000 "$STAGE" 2>/dev/null || true

# ------------------------------------------------------------------ run
# The privileged half, through the helper's one loud escape: mount(2)
# and pivot_root(2) build the topology, and nothing unprivileged can.
# STAGE travels read-write; TARGET_STAGE callers (300's clause 7 stages a
# store there) land under the work directory's mount the same way.
eng_mount "$STAGE" /stage rw \
	|| { echo "SKIP: $STAGE could not be staged" >&2; exit 2; }
ENVS="TARGET_HOST_UID=1000 TARGET_WORKSPACE=/stage"

if [ "$RAW" = 1 ]; then
	eng_privrun 3600 "$IID" /bin/sh "$ENVS" -- -c \
		'cp -a /stage/. /target-stage 2>/dev/null; exec "$@"' -- "${ARGS[@]}"
	rc=$?
	eng_clear
	exit "$rc"
fi

echo "== reconstruction conditions" >&2
printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >&2
printf 'host kernel       %s\n' "$(uname -r)" >&2
printf 'engine            %s\n' "$(engine_describe)" >&2
printf 'image             %s\n' "$(eng_image_inspect "$IMAGE" '{{.Id}}')" >&2
printf 'harness sources   %s\n' "$HARNESS_SRC" >&2

eng_privrun 3600 "$IID" "" "$ENVS" -- "${ARGS[@]}"
rc=$?
eng_clear
exit "$rc"
