#!/usr/bin/env bash
# Question: `podbox-complete` supplies `/etc/passwd`, `/etc/group` and
# `/etc/resolv.conf` into a rootfs (TOOL.md section 6.5). Under what condition
# does the payload actually read them?
#
# The answer is not "when the file is there". glibc resolves a user through
# NSS, and `/etc/nsswitch.conf` names which service answers. `files` reads
# `/etc/passwd`; anything else does not, and a supplied file is then a shim
# that silently does nothing.
#
# Four checks:
#
#   A. a supplied `/etc/passwd` against two nsswitch settings, with a novel
#      user that exists in no image. This is the requirement.
#   B. what pinned images actually ship, because the requirement is only
#      expensive if real images disagree.
#   C. `PT_INTERP`-free is not dependency-free: what a `-static` glibc binary
#      opens after its own execve, counted with the instrument that does not
#      inflate itself on `/etc/ld.so.cache`.
#   D. glibc's gconv modules and DT_NEEDED, which is the settled reason not to
#      bundle them into a musl artefact.
#
# ⭐ A uses a user name no distribution ships, so a hit cannot come from the
# image's own `/etc/passwd`.
#
# Inputs pinned by manifest digest:
#   ubuntu@sha256:c664f8f86ed5a386b0a340d981b8f81714e21a8b9c73f658c4bea56aa179d54a
#   debian@sha256:2f65600e1252c5649d2213e1d1ea4d74253d26514dc6530102a875e429245929
#   alpine@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc
#
# Exit: 0 every check ran and matched, 1 a check ran and did not match,
#       2 could not run (no cc and no staged probe, no engine).
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
ROOT="$(CDPATH= cd -- "$HERE/.." && pwd)"
OUT="${OUT:-$HERE/.nsswitch}"
rm -rf "$OUT"; mkdir -p "$OUT"

UBUNTU='ubuntu@sha256:c664f8f86ed5a386b0a340d981b8f81714e21a8b9c73f658c4bea56aa179d54a'
DEBIAN='debian@sha256:2f65600e1252c5649d2213e1d1ea4d74253d26514dc6530102a875e429245929'
ALPINE='alpine@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'

# The engine is `experiments/lib/engine.sh`: a docker daemon where one
# answers, else host podman. What each check asserts is unchanged.
# shellcheck source=lib/engine.sh
. "$HERE/lib/engine.sh"
export ENGINE_REPO ENGINE_WORK
ENGINE_REPO="$ROOT"
ENGINE_WORK="$OUT"

# The static probe's source lives in one file so the guest build that
# supplies lanes without a C toolchain compiles the same bytes this lane
# builds natively. A static glibc probe is the point: musl reads
# /etc/passwd directly whatever nsswitch says, so a musl probe would pass
# check A on every row and prove nothing.
cp "$HERE/src/nsswitch-q.c" "$OUT/q.c"
if command -v cc >/dev/null 2>&1; then
  CC_VIA="$(cc --version 2>/dev/null | head -1)"
elif [ -r "${Q_STATIC:-}" ]; then
  CC_VIA="guest-built static probe"
fi

echo "== conditions"
printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf 'host kernel       %s\n' "$(uname -r)"
_ldd="$(ldd --version 2>&1)"
printf 'host libc         %s\n' "${_ldd%%$'\n'*}"
printf 'cc                %s\n' "${CC_VIA:-absent}"
printf 'strace            %s\n' "$(strace -V 2>/dev/null | head -1 || echo absent)"
echo

if [ -z "${CC_VIA:-}" ] && [ ! -r "${Q_STATIC:-}" ]; then
  echo "SKIP: no cc and no guest-built static probe (set Q_STATIC)" >&2; exit 2
fi
if engine_pick; then HAVE_ENGINE=1; else HAVE_ENGINE=0; fi
[ "$HAVE_ENGINE" -eq 1 ] || { echo "SKIP: no engine (a docker daemon or host podman)" >&2; exit 2; }
printf 'engine            %s\n' "$(engine_describe)"
echo

rc=0
fail() { printf 'FAIL %s\n' "$1"; rc=1; }
pass() { printf 'ok   %s\n' "$1"; }

if command -v cc >/dev/null 2>&1; then
  cc -static -o "$OUT/q_static" "$OUT/q.c" 2>"$OUT/cc.log" || {
    echo "SKIP: cannot build a static probe; see $OUT/cc.log" >&2; exit 2; }
else
  cp "$Q_STATIC" "$OUT/q_static" || {
    echo "SKIP: cannot stage the guest-built static probe" >&2; exit 2; }
fi

cat > "$OUT/passwd" <<'EOF'
root:x:0:0:root:/root:/bin/sh
podboxsupplied:x:1000:1000:supplied by podbox-complete:/workspace:/bin/sh
EOF
printf 'passwd: files\ngroup: files\n'     > "$OUT/nsw.files"
printf 'passwd: systemd\ngroup: systemd\n' > "$OUT/nsw.other"

echo "== A. is a supplied /etc/passwd consulted? nsswitch decides"
for ns in files other; do
  eng_mount "$OUT/passwd" /etc/passwd \
    && eng_mount "$OUT/nsw.$ns" /etc/nsswitch.conf \
    && eng_mount "$OUT/q_static" /q \
    || { fail "A: the $ns run could not be staged"; eng_clear; continue; }
  out="$(eng_run 180 "$UBUNTU" "" -- /q 2>&1 | head -1)"
  eng_clear
  printf '  nsswitch=%-6s -> %s\n' "$ns" "$out"
  eval "res_$ns=\$out"
done
if [ "${res_files:-}" = FOUND ] && [ "${res_other:-}" = NOTFOUND ]; then
  pass "A: the supplied /etc/passwd is read under 'files' and IGNORED otherwise"
  echo "     ⛔ podbox-complete must supply /etc/nsswitch.conf too, or assert it"
  echo "        already names files. Supplying /etc/passwd alone is a no-op on"
  echo "        any rootfs whose nsswitch names another service."
elif [ "${res_files:-}" != FOUND ]; then
  fail "A: the supplied /etc/passwd was NOT read even under 'files'; the control is broken"
else
  fail "A: the supplied /etc/passwd was read under a non-files service, so nsswitch is not the switch here"
fi
echo

echo "== B. what pinned images actually ship"
saw_files=0; saw_none=0
# The three pins are fetched before any timed clause, as 105 does; a pin
# that does not fetch reads no-pull on its row, exactly as before.
for img in "$UBUNTU" "$DEBIAN" "$ALPINE"; do
  eng_pull "$img" || echo "  no-pull $img"
done
for row in "ubuntu-20.04 $UBUNTU" "debian-12 $DEBIAN" "alpine-3.22 $ALPINE"; do
  name=${row%% *}; img=${row##* }
  ns="$(eng_run 180 "$img" "" -- sh -c 'grep -E "^passwd" /etc/nsswitch.conf 2>/dev/null || echo NO-NSSWITCH-FILE' 2>&1 | tr -d '\r')"
  [ -n "$ns" ] || { printf '  %-14s no-pull\n' "$name"; continue; }
  printf '  %-14s %s\n' "$name" "$ns"
  case "$ns" in
    *NO-NSSWITCH*) saw_none=$((saw_none+1)) ;;
    *files*)       saw_files=$((saw_files+1)) ;;
  esac
done
# ⚠ A row that could not be pulled is neither of these and is not counted.
if [ $((saw_files + saw_none)) -eq 0 ]; then
  echo "SKIP: no image could be read, so B measured nothing" >&2; exit 2
fi
pass "B: $saw_files image(s) name files, $saw_none ship no nsswitch.conf at all"
echo "     A musl rootfs has no NSS and reads /etc/passwd directly, so the"
echo "     requirement in A is a glibc-rootfs requirement."
echo

echo "== C. PT_INTERP-free is not dependency-free"
printf '  static probe PT_INTERP headers: %s\n' "$(readelf -l "$OUT/q_static" 2>/dev/null | grep -c INTERP)"
if command -v strace >/dev/null 2>&1; then
  # ⛔ THREE TRAPS, all of which inflate or deflate this count:
  #   count only after the LAST execve, or the shell and the tracer are counted;
  #   require .so or .so.N at the END of the name, because /etc/ld.so.cache is
  #   an index and matching it as an object inflated every early reading;
  #   -f, because a child's opens belong to the tree and not to one pid.
  strace -f -e trace=openat,open,execve "$OUT/q_static" >/dev/null 2>"$OUT/strace.txt"
  opened="$(awk '/execve\(/{n=NR} {l[NR]=$0} END{for(i=n+1;i<=NR;i++) print l[i]}' "$OUT/strace.txt" \
            | grep -oE '"[^"]+"' | tr -d '"' | grep -E '\.so(\.[0-9]+)*$' | sort -u)"
  cache="$(grep -c 'ld\.so\.cache' "$OUT/strace.txt")"
  printf '  objects opened after the last execve: %s\n' "$(printf '%s' "$opened" | grep -c . )"
  [ -n "$opened" ] && printf '%s\n' "$opened" | sed 's/^/     /'
  printf '  ld.so.cache lines seen (the trap, correctly excluded): %s\n' "$cache"
  pass "C: measured on this host with nsswitch '$(grep -E '^passwd' /etc/nsswitch.conf 2>/dev/null | tr -s ' ')'"
  echo "     ⚠ The count is a property of THIS host's nsswitch, not of static"
  echo "        linking. A rootfs naming a non-files service is where a static"
  echo "        glibc payload reaches for a host NSS module it cannot have."
else
  echo "  COULD NOT RUN: no strace on this host."
  echo "  This is the one check here that needs it; A, B and D stand without it."
fi
echo

echo "== D. glibc's gconv modules and DT_NEEDED"
GCONV=/usr/lib/x86_64-linux-gnu/gconv
if [ -d "$GCONV" ]; then
  tot=0; wl=0
  for m in "$GCONV"/*.so; do
    [ -e "$m" ] || continue
    tot=$((tot+1))
    readelf -dW "$m" 2>/dev/null | grep -q 'libc\.so\.6' && wl=$((wl+1))
  done
  printf '  %s of %s gconv modules record DT_NEEDED libc.so.6\n' "$wl" "$tot"
  if [ "$tot" -gt 0 ] && [ "$wl" -eq "$tot" ]; then
    pass "D: bundling gconv into a musl artefact reintroduces a second libc"
  else
    fail "D: not every gconv module records libc.so.6; re-read this host's glibc"
  fi
else
  echo "  COULD NOT RUN: no gconv directory on this host."
fi
echo

echo "== verdict"
[ "$rc" -eq 0 ] && echo "  every check that ran matched" || echo "  see the FAIL lines above"
exit "$rc"
