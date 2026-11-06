#!/bin/sh
# cloud-setup.sh - make a cloud session or a Linux host ready for the builds
# and checks of podvm. Each step is skipped when its tool is there, so the
# script can run at the start of each session.
#
#   sh scripts/cloud-setup.sh
#
# 1. zig from the mirrors that PodUtils/podcrates keeps (its fetch-zig.sh at
#    a pinned commit), because ziglang.org can be outside the allowlist of
#    the egress proxy of a cloud session. The version and the SHA-256 are
#    the pins of scripts/common/bootstrap-env.sh.
# 2. scripts/common/bootstrap-env.sh for the rest: the Rust toolchain and the
#    musl target, cargo-bloat, gcc, the binary tools and OpenSSH.
# 3. A report of the host facts that podvm's checks depend on. A cloud
#    container is a restricted host of its own kind: record what it allows.
#
# Exit codes: 0 ready, 1 a step failed, 2 could not run (not Linux).
set -u

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)
[ "$(uname -s)" = Linux ] || { echo "cloud-setup: Linux only" >&2; exit 2; }

PODCRATES_COMMIT=54427f6677b17cc96638f0fafc44b615e467fb4f
ZIG_VERSION=0.16.0
ZIG_SHA256=70e49664a74374b48b51e6f3fdfbf437f6395d42509050588bd49abe52ba3d00
tools=${PODVM_TOOLS:-$HOME/.local/podvm}
work=$root/.work/cloud-setup
rc=0

mkdir -p "$work" "$tools" "$HOME/.local/bin"
case ":$PATH:" in *":$HOME/.local/bin:"*) ;; *) PATH=$HOME/.local/bin:$PATH; export PATH ;; esac

if [ "$(zig version 2>/dev/null)" != "$ZIG_VERSION" ]; then
    for f in fetch-zig.sh zig-mirrors.txt; do
        curl -qfsSL --max-time 60 -o "$work/$f" \
            "https://raw.githubusercontent.com/PodUtils/podcrates/$PODCRATES_COMMIT/scripts/$f" || rc=1
    done
    if [ "$rc" -eq 0 ] &&
        ZIG_SOURCE=podvm sh "$work/fetch-zig.sh" "$ZIG_VERSION" "$ZIG_SHA256" "$tools/zig.tar.xz" "$work/zig-mirrors.txt" &&
        tar -xJf "$tools/zig.tar.xz" -C "$tools"; then
        rm -f "$tools/zig.tar.xz"
        ln -sf "$tools/zig-x86_64-linux-$ZIG_VERSION/zig" "$HOME/.local/bin/zig"
    else
        echo "cloud-setup: zig $ZIG_VERSION could not be fetched" >&2
        rc=1
    fi
fi

bash "$root/scripts/common/bootstrap-env.sh" rust bloat cc tools openssh || rc=1

# The host facts. Each line is measured here, not assumed.
fact() { printf '  %-22s %s\n' "$1" "$2"; }
echo "cloud-setup: host facts"
fact "kernel" "$(uname -r)"
if [ -e /dev/kvm ]; then fact "/dev/kvm" present; else fact "/dev/kvm" absent; fi
if [ -e /dev/ptmx ]; then fact "/dev/ptmx" present; else fact "/dev/ptmx" absent; fi
if [ -e /dev/fuse ]; then fact "/dev/fuse" present; else fact "/dev/fuse" absent; fi
if unshare -U true 2>/dev/null; then fact "user namespaces" allowed; else fact "user namespaces" denied; fi
if [ "$(id -u)" = 0 ]; then fact "uid" "0 (root)"; else fact "uid" "$(id -u)"; fi
fact "seccomp" "$(grep -m1 '^Seccomp:' /proc/self/status 2>/dev/null | awk '{print $2}')"
fact "disk free (repo)" "$(df -h "$root" 2>/dev/null | awk 'NR==2 {print $4}')"
if [ -n "${HTTPS_PROXY:-${https_proxy:-}}" ]; then fact "egress" "through HTTPS_PROXY"; else fact "egress" "direct"; fi
fact "zig" "$(zig version 2>/dev/null || echo missing)"
fact "rustc" "$(rustc --version 2>/dev/null || echo missing)"
echo "cloud-setup: probe the host with podvm itself: ./target/x86_64-unknown-linux-musl/release/podvm probe"
exit "$rc"
