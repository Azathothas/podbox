#!/usr/bin/env bash
# Question: can a container image supply the target runtime's userspace - the
# toolchain it has (go, gcc, python3, tar, zstd) and the files it lacks
# (/etc/passwd, /run, /var, /dev/fuse)?
#
# Builds one image, pinned by base digest. Prints its own conditions.
# Exit: 0 built, 1 build failed, 2 could not run (no docker).
set -euo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
IMAGE="${TARGET_IMAGE:-container-research/target:1}"
# ⭐ The lane's scratch lives under the checkout on a non-native lane:
# mount sources must be Windows paths there (245's rule), and /tmp/...
# names nothing. Nothing here is read back host-side, so either scratch
# does; what matters is that the build context stays under a declared root.
case "$(uname -s)" in
Linux) WORK="$(mktemp -d)" ;;
*)     WORK="$REPO/experiments/.sweep10-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2 ;;
esac
trap 'rm -rf "$WORK"' EXIT INT TERM

# The engine is `experiments/lib/engine.sh`: a docker daemon where one
# answers, else host podman. What the build asserts is unchanged.
# shellcheck source=lib/engine.sh
. "$HERE/lib/engine.sh"
export ENGINE_REPO ENGINE_WORK
ENGINE_REPO="$REPO"
ENGINE_WORK="$WORK"
if engine_pick; then HAVE_ENGINE=1; else HAVE_ENGINE=0; fi
[ "$HAVE_ENGINE" -eq 1 ] || { echo "SKIP: no engine (a docker daemon or host podman)" >&2; exit 2; }

echo "== building $IMAGE"
eng_build 1800 "$IMAGE" "$HERE/Dockerfile.target" "$HERE" -- || exit 1

echo
echo "== conditions"
printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf 'host kernel       %s\n' "$(uname -r)"
printf 'engine            %s\n' "$(engine_describe)"
printf 'image             %s\n' "$IMAGE"
printf 'image id          %s\n' "$(eng_image_inspect "$IMAGE" '{{.Id}}')"

echo
echo "== what the image has, and has not"
# By ID, not by fixture tag: the tag is a local name by definition, and
# eng_run only takes pinned references. The ID names the exact bytes the
# build above produced; the `sha256:` prefix some engines print is not
# part of it. What the census asserts is unchanged.
IID="$(eng_image_inspect "$IMAGE" '{{.Id}}')"
IID="${IID#sha256:}"
# Exported, not assigned: eng_run reads it as a global, and an assignment
# no same-file command reads is a SC2034 waiting to happen. eng_clear
# below takes it back down.
export ENG_ENTRYPOINT="/bin/sh"
eng_run 300 "$IID" "" -- -c '
  printf "go                %s\n" "$(go version 2>/dev/null | cut -d" " -f3 || echo MISSING)"
  printf "gcc               %s\n" "$(gcc -dumpversion 2>/dev/null || echo MISSING)"
  printf "python3           %s\n" "$(python3 -V 2>&1 | cut -d" " -f2 || echo MISSING)"
  printf "tar               %s\n" "$(tar --version | head -1 | cut -d" " -f4)"
  printf "zstd              %s\n" "$(zstd --version 2>&1 | sed "s/.*v\([0-9.]*\).*/\1/")"
  printf "rustc             %s\n" "$(rustc --version 2>/dev/null || echo "MISSING (as on the target)")"
  printf "docker on PATH    %s\n" "$(docker --version 2>/dev/null || echo MISSING)"
'
eng_clear
echo
echo "next: ./experiments/20-enter-target.sh"
