#!/bin/sh
# The M5 subject. Print one KEY=VALUE per line; the runner reads those.
set -u
say() { printf '%s\n' "$*"; }

# 1. what is here, before anything is installed.
say "ID=$(id -u):$(id -g)"
if [ -e /dev/null ] && [ ! -s /dev/null ]; then say "DEVNULL=usable"; else say "DEVNULL=MISSING"; fi
echo probe >/dev/null 2>&1 && say "DEVNULL_WRITE=ok" || say "DEVNULL_WRITE=failed"
if [ -r /etc/resolv.conf ]; then
  say "RESOLVER=$(grep -m1 '^nameserver' /etc/resolv.conf 2>/dev/null | awk '{print $2}')"
else
  say "RESOLVER=absent"
fi
if [ -r /etc/nsswitch.conf ]; then
  say "NSSWITCH=$(grep -E '^[[:space:]]*passwd' /etc/nsswitch.conf | head -1 | tr -s ' \t' ' ' | sed 's/^ *passwd: *//')"
else
  say "NSSWITCH=absent"
fi
# ⭐ T-0404's own question, and it is asked with the tool the DISTRIBUTION
# ships rather than by reading the file: `getent` goes through NSS on glibc,
# which is the whole of T-0410.
if command -v getent >/dev/null 2>&1; then
  say "GETENT_ROOT=$(getent passwd 0 2>/dev/null | cut -d: -f1)"
else
  say "GETENT_ROOT=no-getent"
fi

# 2. the package manager, detected.
PM=none
for p in apk apt-get pacman dnf microdnf zypper xbps-install; do
  command -v "$p" >/dev/null 2>&1 && { PM="$p"; break; }
done
say "PM=$PM"

# 3. install a C toolchain with it. ⚠ Every one is non-interactive by flag:
# a prompt with no terminal behind it is a hang, and a hang costs the run.
install_rc=0
case "$PM" in
apk)      apk add --no-cache gcc musl-dev >/tmp/inst.log 2>&1 || install_rc=$? ;;
apt-get)  DEBIAN_FRONTEND=noninteractive apt-get update -qq >/tmp/inst.log 2>&1 &&
          DEBIAN_FRONTEND=noninteractive apt-get install -y -qq gcc libc6-dev >>/tmp/inst.log 2>&1 || install_rc=$? ;;
pacman)   pacman -Sy --noconfirm --needed gcc >/tmp/inst.log 2>&1 || install_rc=$? ;;
dnf)      dnf install -y --setopt=install_weak_deps=False gcc glibc-devel >/tmp/inst.log 2>&1 || install_rc=$? ;;
microdnf) microdnf install -y gcc glibc-devel >/tmp/inst.log 2>&1 || install_rc=$? ;;
zypper)   zypper --non-interactive --gpg-auto-import-keys refresh >/tmp/inst.log 2>&1;
          zypper --non-interactive install gcc glibc-devel >>/tmp/inst.log 2>&1 || install_rc=$? ;;
xbps-install) xbps-install -Suy >/tmp/inst.log 2>&1;
          xbps-install -y gcc >>/tmp/inst.log 2>&1 || install_rc=$? ;;
*)        install_rc=127 ;;
esac
say "INSTALL_RC=$install_rc"
# ⚠ The tail of the log travels with the row, or a failure is a number with no
# cause attached to it.
say "INSTALL_TAIL=$(tail -3 /tmp/inst.log 2>/dev/null | tr '\n' ' ' | tr -s ' ' | cut -c1-220)"

# 4. ⛔ TWO FILES, so a toolchain that installed and cannot LINK is caught.
mkdir -p /tmp/p
cat >/tmp/p/helper.h <<'EOF'
int answer(void);
EOF
cat >/tmp/p/helper.c <<'EOF'
#include "helper.h"
int answer(void) { return 42; }
EOF
cat >/tmp/p/main.c <<'EOF'
#include <stdio.h>
#include "helper.h"
int main(void) { printf("BUILT_AND_RAN=%d\n", answer()); return 0; }
EOF
if command -v cc >/dev/null 2>&1; then CC=cc; else CC=gcc; fi
say "CC=$(command -v $CC 2>/dev/null || echo none)"
if command -v $CC >/dev/null 2>&1; then
  say "CC_VERSION=$($CC --version 2>&1 | head -1)"
  ( cd /tmp/p && $CC -O1 -o prog main.c helper.c ) >/tmp/build.log 2>&1
  say "BUILD_RC=$?"
  say "BUILD_TAIL=$(tail -2 /tmp/build.log 2>/dev/null | tr '\n' ' ' | cut -c1-200)"
  if [ -x /tmp/p/prog ]; then /tmp/p/prog; else say "BUILT_AND_RAN=no-binary"; fi
else
  say "CC_VERSION=absent"
  say "BUILD_RC=127"
  say "BUILT_AND_RAN=no-compiler"
fi
SUBJECT
