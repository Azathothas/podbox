#!/bin/sh
# Lane job: T-1312 loader proof. Build the objects, then load the glibc one
# under the nix closure's own glibc 2.27 with a trivial probe binary. If the
# loader binds dlsym and starts the process, the 152 REGISTER wall is down.
set -u
cd /work || exit 2
TARGETS=x86_64-unknown-linux-gnu ./scripts/build-interpose.sh || exit 2
SO=crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so
URL="https://nixos.org/releases/nix/nix-2.2.2/nix-2.2.2-x86_64-linux.tar.bz2"
SIZE=23607712
curl -fsSL "$URL" -o /tmp/nix.tbz2 || exit 2
[ "$(stat -c%s /tmp/nix.tbz2)" = "$SIZE" ] || { echo "size mismatch"; exit 2; }
tar -tjf /tmp/nix.tbz2 | grep -E "glibc-2\.27/lib/(libc\.so\.6|ld-linux)" | head -5
mkdir -p /tmp/nixfull
tar -xjf /tmp/nix.tbz2 -C /tmp/nixfull || exit 2
GLIBCDIR="$(echo /tmp/nixfull/*/store/*-glibc-2.27/lib)"
[ -d "$GLIBCDIR" ] || { echo "no glibc dir"; exit 2; }
echo "glibc dir: $GLIBCDIR"
ls "$GLIBCDIR" | head
printf '#define _GNU_SOURCE\n#include <unistd.h>\n#include <sys/syscall.h>\nint main(void){ long t = syscall(186); return t > 0 ? 0 : 1; }\n' > /tmp/probe.c
cc -static -o /tmp/probe /tmp/probe.c || exit 2
echo "== the static probe needs no loader of its own"
env -u LD_PRELOAD LD_LIBRARY_PATH= /tmp/probe
echo "bare_probe_rc=$?"
echo "== load OUR object under glibc 2.27"
LD_PRELOAD="/work/$SO" LD_LIBRARY_PATH="$GLIBCDIR" /tmp/probe
echo "probe_rc=$?"
echo "== control without preload"
env -u LD_PRELOAD LD_LIBRARY_PATH= /tmp/probe
echo "control_rc=$?"
echo JOB_DONE
