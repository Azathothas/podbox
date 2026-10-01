#!/bin/sh
# Can the release ship the exact SSH helper names with runnable static bytes?
# Usage: package-ssh.sh RELEASE_DIR ARCH [QEMU]
# Exit: 0 matched, 1 failed, 2 could not run. T-1405.
set -u
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 2
ROOT=$(CDPATH= cd -- "$HERE/.." && pwd) || exit 2
BIN_DIR=${1:-}
ARCH=${2:-}
QEMU=${3:-}
[ -d "$BIN_DIR" ] && [ -n "$ARCH" ] || exit 2
case "$ARCH" in x86_64|aarch64|riscv64gc|loongarch64|armv7|i686|powerpc64le) ;; *) exit 2 ;; esac
WORK=$(mktemp -d) || exit 2
trap 'rm -rf "${WORK:?}"' EXIT HUP INT TERM
mkdir -p "$WORK/package" || exit 2
for helper in node operator proxy shell; do
    binary="$BIN_DIR/$helper"
    [ -x "$binary" ] || { echo "SSH-PACKAGE-FAIL missing executable: $helper" >&2; exit 1; }
    readelf -h "$binary" >"$WORK/header" 2>&1 || {
        echo "SSH-PACKAGE-FAIL invalid ELF: $helper" >&2; exit 1;
    }
    readelf -l "$binary" >"$WORK/program" 2>&1 || exit 1
    if grep -q INTERP "$WORK/program"; then
        echo "SSH-PACKAGE-FAIL dynamic interpreter: $helper" >&2; exit 1
    fi
    if [ -n "$QEMU" ]; then
        timeout 15 env -u PODSSH_RELAY -u PODSSH_NAME -u PODSSH_NODE_TOKEN -u PODSSH_CONNECT_TOKEN \
            "$QEMU" "$binary" --help >"$WORK/usage" 2>&1
    else
        timeout 15 env -u PODSSH_RELAY -u PODSSH_NAME -u PODSSH_NODE_TOKEN -u PODSSH_CONNECT_TOKEN \
            "$binary" --help >"$WORK/usage" 2>&1
    fi
    rc=$?
    [ "$rc" -eq 125 ] && grep -q "usage: $helper" "$WORK/usage" || {
        echo "SSH-PACKAGE-FAIL usage contract: $helper (exit $rc)" >&2; exit 1;
    }
    cp "$binary" "$WORK/package/$helper" || exit 1
done
cp "$ROOT/LICENSE" "$WORK/package/LICENSE" || exit 1
cp "$ROOT/crates/podbox-ssh/shims/LICENSE" "$WORK/package/SSH-SHIM-LICENSE" || exit 1
# Host python is `python` or `py`, never bare `python3` (a Store stub, exit 49, on Windows).
PYBIN="$(command -v python || command -v py || command -v python3)" || exit 2
"$PYBIN" "$ROOT/scripts/release-licenses.py" --output "$WORK/package/dependency-licenses" || exit "$?"
epoch=${SOURCE_DATE_EPOCH:-$(git -C "$ROOT" log -1 --format=%ct)}
tar --sort=name --mtime="@$epoch" --owner=0 --group=0 --numeric-owner \
    -cf "$WORK/package.tar" -C "$WORK/package" . || exit 1
gzip -n "$WORK/package.tar" || exit 1
mkdir -p "$ROOT/dist" || exit 2
asset="$ROOT/dist/podbox-ssh-$ARCH.tar.gz"
cp "$WORK/package.tar.gz" "$asset" || exit 1
(cd "$ROOT/dist" && sha256sum "podbox-ssh-$ARCH.tar.gz" >"podbox-ssh-$ARCH.tar.gz.sha256") || exit 1
echo "SSH-PACKAGE-OK $ARCH: node operator proxy shell and retained notices"
