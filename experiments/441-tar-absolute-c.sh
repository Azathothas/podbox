#!/bin/sh
# Question: on a host that denies chroot(2), does GNU tar extract into an
# absolute `-C` directory of the rootfs, as it does into a relative one?
#
# TODO/interpose.md T-1743; PodUtils/podcrates#102 (COORD-063) and
# PodUtils/podrunner#70 (COORD-012). podrunner measured on a hosted
# runner with no user namespace: `tar -xf x.tar -C /ABS/d` failed and
# `tar -xf x.tar -C d` passed. GNU tar 1.35 of Ubuntu 24.04 opens the
# directory of `-C` with `__openat_2`, the fortified `openat` of glibc.
#
# The fixture is the one of experiment 356: user and mount namespaces,
# then experiments/deny-chroot.c denies chroot(2), so podvm takes the
# loader family of the userland rung, which podrunner's runner takes.
#
# Inputs: the podvm binary (PODVM_441_BIN, else the release binary of
# this tree, built with the interposer objects), cc, and the pinned
# Ubuntu 24.04 image below (PODVM_441_MIRROR=1 takes it from
# mirror.gcr.io).
#
# Clauses:
#   1. the fixture denies chroot, and the payload runs on the userland
#      rung (PODVM_ACTIVE_MODE).
#   2. podrunner's table, for three bases: B=/tmp/podvm441g and
#      B=/podvm441/app, which the host does not have (podrunner's case),
#      and B=/tmp/podvm441, which the host has as an empty directory. For
#      each base: a relative and an absolute `-C`, with and without `-z`,
#      and `cd` first; each form gives `d/f`.
#   3. the host directory /tmp/podvm441/d still exists after the run and
#      is empty: tar writes no member on the host, and removes nothing.
#
# A trace of the release 0.2.0-beta.6 (2026-11-13) gives
# `openat(AT_FDCWD, "/tmp/podvm441/d", O_PATH|O_DIRECTORY)` with no
# rewrite: the directory of the host opens. Where the host has no such
# directory, tar says "Cannot open".
#
# Exit: 0 each clause matched, 1 a clause failed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep441-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
OUT="$REPO/experiments/results/tar-absolute-c.txt"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  FAILED: $*"; fail=1; }
HOSTB=/tmp/podvm441
HOSTG=/tmp/podvm441g
cleanup() { rm -rf "$WORK" "$HOSTB" "$HOSTG"; }
trap cleanup EXIT INT TERM

UBUNTU='public.ecr.aws/docker/library/ubuntu:24.04@sha256:2198e4c65076e9cb0b3d695219df667c0e3b0072da64ca3c2648aca17dcecaf5'
[ "${PODVM_441_MIRROR:-0}" = 1 ] && UBUNTU='mirror.gcr.io/library/ubuntu:24.04@sha256:2198e4c65076e9cb0b3d695219df667c0e3b0072da64ca3c2648aca17dcecaf5'
PB="${PODVM_441_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"

command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" >&2; exit 2; }
[ -x "$PB" ] || { echo "no podvm binary at $PB: COULD NOT RUN" >&2; exit 2; }
unshare -Urm true >/dev/null 2>&1 || { echo "user+mount namespaces refused: COULD NOT RUN" >&2; exit 2; }
cc -O2 -Wall -Werror -static -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>"$WORK/cc.err" ||
	cc -O2 -Wall -Werror -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>>"$WORK/cc.err" ||
	{ echo "the fixture does not build: COULD NOT RUN" >&2; exit 2; }
cat >"$WORK/wrap.sh" <<'EOF'
#!/bin/sh
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF
FX="unshare -Urm sh $WORK/wrap.sh $WORK"
export PODVM_STORE="$WORK/store"

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$PB" version 2>&1 | head -1)"
note "image             $UBUNTU"
timeout 600 "$PB" pull "$UBUNTU" >"$WORK/pull.txt" 2>&1 || { tail -5 "$WORK/pull.txt" >&2; echo "pull failed: COULD NOT RUN" >&2; exit 2; }

# Clause 3 needs a host directory with the guest's name: an unmapped open
# then finds it, and a member would land in it. The other bases must not
# exist on the host, as on podrunner's runner.
rm -rf "$HOSTB" "$HOSTG"; mkdir -p "$HOSTB/d" || exit 2
[ -e /podvm441 ] && { echo "the host has /podvm441: COULD NOT RUN" >&2; exit 2; }

# One run, as podrunner made it: each form prints its name and ok or failed.
cat >"$WORK/forms.sh" <<'EOF'
echo "mode $PODVM_ACTIVE_MODE"
echo "tar $(tar --version | head -1)"
for B in /tmp/podvm441g /podvm441/app /tmp/podvm441; do
	mkdir -p "$B" && cd "$B" || { echo "$B setup failed"; continue; }
	echo x > f; tar -cf x.tar f; gzip -c x.tar > x.tgz
	form() { name="$1"; shift; rm -rf "$B/d"; mkdir -p "$B/d"; cd "$B"; ( eval "$*" ) >/dev/null 2>"$B/err"; if [ -f "$B/d/f" ]; then echo "$B | $name | ok"; else echo "$B | $name | failed: $(head -1 "$B/err")"; fi; }
	form "tar -xf x.tar -C d" 'tar -xf x.tar -C d'
	form "tar -xzf x.tgz -C d" 'tar -xzf x.tgz -C d'
	form "tar -xf x.tar -C \$B/d" 'tar -xf x.tar -C "$B/d"'
	form "tar -xzf x.tgz -C \$B/d" 'tar -xzf x.tgz -C "$B/d"'
	form "cd d && tar -xf ../x.tar" 'cd d && tar -xf ../x.tar'
	form "cd d && gzip -dc ../x.tgz | tar -xf -" 'cd d && gzip -dc ../x.tgz | tar -xf -'
done
EOF
note ""
note "== the forms, one run under the fixture"
timeout 300 $FX "$PB" run --rm "$UBUNTU" /bin/bash -c "$(cat "$WORK/forms.sh")" >"$WORK/run.txt" 2>"$WORK/run.err"
rc=$?
sed 's/^/  /' "$WORK/run.txt" | tee -a "$OUT"
note "  exit $rc"
[ "$rc" -eq 0 ] || { sed 's/^/  stderr: /' "$WORK/run.err" | tail -5 | tee -a "$OUT"; bad "the run exits $rc"; }

note ""
note "== clause 1: the rung"
grep -q '^mode userland' "$WORK/run.txt" || bad "the payload did not run on the userland rung"
note "== clause 2: each form gives d/f"
forms=$(grep -c ' | ' "$WORK/run.txt")
[ "$forms" -eq 18 ] || bad "$forms of 18 forms ran"
grep -q ' | failed' "$WORK/run.txt" && bad "a form failed"
note "== clause 3: the host directory $HOSTB/d stays, empty"
if [ ! -d "$HOSTB/d" ]; then
	bad "the host directory $HOSTB/d is gone"
elif [ -n "$(ls -A "$HOSTB/d")" ]; then
	bad "tar wrote on the host: $(ls -A "$HOSTB/d" | tr '\n' ' ')"
fi
[ -e "$HOSTG" ] && bad "the run made $HOSTG on the host"

if [ "$fail" -eq 0 ]; then
	note "verdict: EACH FORM EXTRACTS IN THE ROOTFS, AND THE HOST IS UNTOUCHED"
else
	note "verdict: A CLAUSE FAILED"
fi
exit "$fail"
