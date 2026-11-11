#!/bin/sh
# Question: do the provider shapes of issue 172 run as built-in profiles:
# the image, the user, the variables, a read-only root, and the limits that
# a rootfs tier cannot hold refused by name?
#
# TODO/cli.md T-1659. Each run goes through the fixture of experiment 356
# (user and mount namespaces, /dev/pts hidden, chroot(2) denied). The
# project is a directory of this script, the current directory of each run
# (the profiles place it at /workspace).
#
# Pinned inputs: the images of the profiles. PODVM_415_MIRROR=1 takes the
# Docker-library images from mirror.gcr.io (`-s fs.image=`), for a host
# where public.ecr.aws answers 429.
#
# Clauses:
#   1. lambda-al2023: `id -u` prints 993, AWS_LAMBDA_FUNCTION_NAME is set,
#      `touch /x` fails with a read-only error, and the project stays
#      writable. While COORD-042 (issue 261) is open, pod-extract refuses a
#      layer of the image; the clause then reports WAITS, which is not a
#      pass, and the entry stays partial.
#   2. busybox-bare: `wget` is there.
#   3. distroless-static: `sh -c true` refuses by name, and a static
#      program of the project runs.
#   4. vercel-sandbox: `id -u` prints 1000 and USER is vercel-sandbox.
#   5. cloudflare-sandbox: `touch /x` fails with a read-only error and the
#      project stays writable.
#   6. `-s limits.memory=128m` refuses by name on a rootfs tier, and
#      `-s 'needs=["curl"]'` refuses busybox-bare by name.
#   7. each built-in profile lists what it leaves out (`profile show`).
#
# Exit: 0 each clause matched (a WAITS clause aside), 1 a clause disagreed,
# 2 the lane could not run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep415-work"
rm -rf "$WORK"; mkdir -p "$WORK/proj" || exit 2
OUT="$REPO/experiments/results/provider-shapes.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
export PODVM_STORE="$WORK/store"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }
M=""; NODE_M=""; ALP_M=""
if [ "${PODVM_415_MIRROR:-0}" = 1 ]; then
	M="-s fs.image=mirror.gcr.io/library/busybox:musl"
	NODE_M="-s fs.image=mirror.gcr.io/library/node:lts-slim"
	ALP_M="-s fs.image=mirror.gcr.io/library/alpine:3.22"
fi

[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" >&2; exit 2; }
unshare -Urm true >/dev/null 2>&1 || { echo "user+mount namespaces refused: COULD NOT RUN" >&2; exit 2; }
cc -O2 -Wall -Werror -static -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" ||
	{ echo "fixture COMPILE FAILED: COULD NOT RUN" >&2; exit 2; }
cp "$WORK/deny-chroot" "$WORK/proj/static-prog"
cat >"$WORK/wrap.sh" <<'EOF'
#!/bin/sh
mount -t tmpfs tmpfs /dev/pts || exit 2
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF
fx() { (cd "$WORK/proj" && timeout 600 unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$PB" "$@"); }

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "fixture           unshare -Urm, /dev/pts hidden, chroot(2) denied"
note ""

note "== clause 1  lambda-al2023"
fx run --rm --profile lambda-al2023 -- sh -c 'id -u; echo "fn=$AWS_LAMBDA_FUNCTION_NAME"; touch /x 2>&1; echo l > /workspace/lambda.txt && echo project-writable' >"$WORK/c1.out" 2>&1
rc=$?
if grep -q 'the entry "" is not extractable' "$WORK/c1.out"; then
	note "  WAITS: pod-extract refuses a layer of the image (the root entry has an empty name); COORD-042, issue 261"
else
	sed 's/^/    /' "$WORK/c1.out" | tail -6 | tee -a "$OUT"
	[ "$rc" -eq 0 ] || bad "the run exited $rc"
	grep -qx 993 "$WORK/c1.out" || bad "id -u is not 993"
	grep -qx "fn=podvm" "$WORK/c1.out" || bad "AWS_LAMBDA_FUNCTION_NAME is not set"
	grep -qi "read-only" "$WORK/c1.out" || bad "touch /x did not fail with a read-only error"
	grep -qx project-writable "$WORK/c1.out" || bad "the project is not writable"
fi
note ""

note "== clause 2  busybox-bare"
# shellcheck disable=SC2086
fx run --rm --profile busybox-bare $M -- sh -c 'wget 2>&1 | head -1' >"$WORK/c2.out" 2>&1
note "    $(tail -1 "$WORK/c2.out")"
grep -q "BusyBox" "$WORK/c2.out" || bad "wget is not there"
note ""

note "== clause 3  distroless-static"
fx run --rm --profile distroless-static -- sh -c true >"$WORK/c3a.out" 2>&1
rc=$?
note "  sh -c true: exit $rc: $(grep -o 'sh names no file in the image' "$WORK/c3a.out" | head -1)"
[ "$rc" -ne 0 ] || bad "sh -c true ran"
grep -q "sh names no file in the image" "$WORK/c3a.out" || bad "the refusal does not name sh"
fx run --rm --profile distroless-static -- /workspace/static-prog >"$WORK/c3b.out" 2>&1
note "  a static program of the project: $(tail -1 "$WORK/c3b.out")"
grep -q "usage: deny-chroot" "$WORK/c3b.out" || bad "the static program of the project did not run"
note ""

note "== clause 4  vercel-sandbox"
fx run --rm --profile vercel-sandbox -- sh -c 'id -u; echo "user=$USER"' >"$WORK/c4.out" 2>&1
sed 's/^/    /' "$WORK/c4.out" | tail -2 | tee -a "$OUT"
grep -qx 1000 "$WORK/c4.out" || bad "id -u is not 1000"
grep -qx "user=vercel-sandbox" "$WORK/c4.out" || bad "USER is not vercel-sandbox"
note ""

note "== clause 5  cloudflare-sandbox"
# shellcheck disable=SC2086
fx run --rm --profile cloudflare-sandbox $ALP_M -- sh -c 'touch /x 2>&1; echo c > /workspace/cf.txt && echo project-writable' >"$WORK/c5.out" 2>&1
sed 's/^/    /' "$WORK/c5.out" | tail -2 | tee -a "$OUT"
grep -qi "read-only" "$WORK/c5.out" || bad "touch /x did not fail with a read-only error"
grep -qx project-writable "$WORK/c5.out" || bad "the project is not writable"
note ""

note "== clause 6  refusals by name"
# shellcheck disable=SC2086
fx run --rm --profile busybox-bare $M -s limits.memory=128m -- true >"$WORK/c6a.out" 2>&1
rc=$?
note "  memory: exit $rc: $(grep -o '`limits.memory` needs[^:]*' "$WORK/c6a.out" | head -1)"
[ "$rc" -eq 125 ] || bad "limits.memory exited $rc"
# shellcheck disable=SC2086
fx run --rm --profile busybox-bare $M -s 'needs=["curl"]' -- true >"$WORK/c6b.out" 2>&1
rc=$?
note "  needs curl: exit $rc: $(grep -o 'the image has no curl' "$WORK/c6b.out" | head -1)"
[ "$rc" -eq 125 ] || bad "needs curl exited $rc"
grep -q "the image has no curl" "$WORK/c6b.out" || bad "the refusal does not name curl"
note ""

note "== clause 7  what each profile leaves out"
for p in $("$PB" profile list | awk 'NR > 1 {print $1}'); do
	n=$("$PB" profile show "$p" 2>/dev/null | grep -c '^leaves_out')
	note "  $p: $("$PB" profile show "$p" 2>/dev/null | grep '^leaves_out' | cut -c1-110)"
	[ "$n" -eq 1 ] || bad "$p lists nothing that it leaves out"
done
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: THE PROVIDER SHAPES RUN AS PROFILES (a WAITS clause aside)"
else
	note "verdict: A CLAUSE DISAGREED"
fi
rm -rf "$WORK"
exit "$fail"
