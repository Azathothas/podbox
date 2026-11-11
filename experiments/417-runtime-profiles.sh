#!/bin/sh
# Question: do the language-runtime profiles of issue 176 start each runtime
# with the flags of one statement of the paths, and does each runtime
# enforce its own permission model inside podvm's path map?
#
# TODO/cli.md T-1661. The runs of Node and Deno go through the fixture of
# experiment 356 (user and mount namespaces, /dev/pts hidden, chroot(2)
# denied). The run of Bun is on the machine tier (KVM, else TCG): Bun makes
# its own system calls, past the interposer. The project is a directory of
# this script, the current directory of each run (the profiles place it at
# /workspace).
#
# Pinned inputs: the images of the profiles. PODVM_417_MIRROR=1 takes them
# from mirror.gcr.io (`-s fs.image=`), for a host where public.ecr.aws or
# docker.io answers 429.
#
# Clauses:
#   1. node-permission: `permission on`; a read of /etc/hostname and a
#      write of /tmp/out.txt are ERR_ACCESS_DENIED; a write of
#      /workspace/out.txt is allowed, and lstat of /workspace (a mount
#      point) answers a directory.
#   2. deno-permissions: a read of /etc/hostname, a write of /tmp/out.txt
#      and Deno.env.get("HOME") are NotCapable; a read of /workspace works.
#   3. bun-runtime: `bun` runs a script of /workspace in a guest.
#   4. `profile show node-permission` prints the runtime's flags and
#      podvm's path map side by side.
#   5. wasmtime-wasi and workerd-test: no profile yet. wasmtime needs a
#      download recorded by hash on first use (F25 of issue 176), and
#      workerd needs the confine layer of T-1658. The clause reports WAITS,
#      which is not a pass, and the entry stays partial.
#
# Exit: 0 each clause matched (a WAITS clause aside), 1 a clause disagreed,
# 2 the lane could not run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep417-work"
rm -rf "$WORK"; mkdir -p "$WORK/proj" || exit 2
OUT="$REPO/experiments/results/runtime-profiles.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
export PODVM_STORE="$WORK/store"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }
NODE_M=""; DENO_M=""; BUN_M=""
if [ "${PODVM_417_MIRROR:-0}" = 1 ]; then
	NODE_M="-s fs.image=mirror.gcr.io/library/node:lts-slim"
	DENO_M="-s fs.image=mirror.gcr.io/denoland/deno:debian"
	BUN_M="-s fs.image=mirror.gcr.io/oven/bun:slim"
fi

[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" >&2; exit 2; }
unshare -Urm true >/dev/null 2>&1 || { echo "user+mount namespaces refused: COULD NOT RUN" >&2; exit 2; }
cc -O2 -Wall -Werror -static -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" ||
	{ echo "fixture COMPILE FAILED: COULD NOT RUN" >&2; exit 2; }
cat >"$WORK/wrap.sh" <<'EOF'
#!/bin/sh
mount -t tmpfs tmpfs /dev/pts || exit 2
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF
fx() { (cd "$WORK/proj" && timeout 600 unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$PB" "$@"); }

cat >"$WORK/proj/perm.js" <<'EOF'
const fs = require("fs");
console.log("permission " + (process.permission ? "on" : "off"));
const t = (what, f) => { try { f(); console.log(what + " allowed"); } catch (e) { console.log(what + " " + e.code); } };
t("read /etc/hostname", () => fs.readFileSync("/etc/hostname"));
t("write /tmp/out.txt", () => fs.writeFileSync("/tmp/out.txt", "x"));
t("write /workspace/out.txt", () => fs.writeFileSync("/workspace/out.txt", "x"));
console.log("lstat /workspace " + (fs.lstatSync("/workspace").isDirectory() ? "directory" : "other"));
EOF
cat >"$WORK/proj/perm.ts" <<'EOF'
const t = (what: string, f: () => unknown) => { try { f(); console.log(what + " allowed"); } catch (e) { console.log(what + " " + (e as Error).name); } };
t("read /etc/hostname", () => Deno.readTextFileSync("/etc/hostname"));
t("write /tmp/out.txt", () => Deno.writeTextFileSync("/tmp/out.txt", "x"));
t("env HOME", () => Deno.env.get("HOME"));
t("read /workspace/perm.ts", () => Deno.readTextFileSync("/workspace/perm.ts"));
EOF
echo 'console.log("bun " + Bun.version + " " + require("fs").readFileSync("/workspace/b.js").length)' >"$WORK/proj/b.js"

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "fixture           unshare -Urm, /dev/pts hidden, chroot(2) denied (clauses 1 and 2)"
note "machine tier      $([ -w /dev/kvm ] && echo KVM || echo TCG) (clause 3)"
note ""

note "== clause 1  node-permission"
# shellcheck disable=SC2086
fx run --rm --profile node-permission $NODE_M -- node perm.js >"$WORK/c1.out" 2>&1
note "  exit $?"
grep -v '^podvm\|^  \|Extracted\|Pulling\|Digest' "$WORK/c1.out" | sed 's/^/    /' | tail -6 | tee -a "$OUT"
grep -qx "permission on" "$WORK/c1.out" || bad "the permission model is not on"
grep -qx "read /etc/hostname ERR_ACCESS_DENIED" "$WORK/c1.out" || bad "the read of /etc/hostname was not denied"
grep -qx "write /tmp/out.txt ERR_ACCESS_DENIED" "$WORK/c1.out" || bad "the write of /tmp/out.txt was not denied"
grep -qx "write /workspace/out.txt allowed" "$WORK/c1.out" || bad "the write of /workspace/out.txt was not allowed"
[ -f "$WORK/proj/out.txt" ] || bad "the write of /workspace/out.txt did not reach the project"
grep -qx "lstat /workspace directory" "$WORK/c1.out" || bad "lstat of the mount point /workspace is not a directory"
note ""

note "== clause 2  deno-permissions"
# shellcheck disable=SC2086
fx run --rm --profile deno-permissions $DENO_M -- perm.ts >"$WORK/c2.out" 2>&1
note "  exit $?"
grep -v '^podvm\|^  \|Extracted\|Pulling\|Digest' "$WORK/c2.out" | sed 's/^/    /' | tail -4 | tee -a "$OUT"
grep -qx "read /etc/hostname NotCapable" "$WORK/c2.out" || bad "the read of /etc/hostname is not NotCapable"
grep -qx "write /tmp/out.txt NotCapable" "$WORK/c2.out" || bad "the write of /tmp/out.txt is not NotCapable"
grep -qx "env HOME NotCapable" "$WORK/c2.out" || bad "Deno.env is not NotCapable"
grep -qx "read /workspace/perm.ts allowed" "$WORK/c2.out" || bad "the read of /workspace was not allowed"
note ""

note "== clause 3  bun-runtime (machine tier)"
if command -v qemu-system-x86_64 >/dev/null 2>&1; then
	# shellcheck disable=SC2086
	(cd "$WORK/proj" && timeout 900 "$PB" run --rm --profile bun-runtime $BUN_M -- b.js) >"$WORK/c3.out" 2>&1
	note "  exit $?"
	note "    $(tail -1 "$WORK/c3.out")"
	grep -q "^bun [0-9]" "$WORK/c3.out" || bad "bun did not run the script of /workspace"
	n=$(pgrep -c qemu-system || true)
	[ "${n:-0}" -eq 0 ] || bad "an emulator stays after the run"
else
	note "  COULD NOT RUN: qemu-system-x86_64 is not on PATH"
	fail=1
fi
note ""

note "== clause 4  the runtime's flags beside podvm's path map"
(cd "$WORK/proj" && "$PB" profile show node-permission) >"$WORK/c4.out" 2>&1
grep '^# ' "$WORK/c4.out" | sed "s|$REPO|\$REPO|g; s/^/    /" | cut -c1-140 | tee -a "$OUT"
grep -q '^# the runtime enforces: node --permission --allow-fs-read=/workspace' "$WORK/c4.out" ||
	bad "profile show does not print the runtime's flags"
grep -q '^# run flags: .*:/workspace' "$WORK/c4.out" || bad "profile show does not print the path map"
note ""

note "== clause 5  wasmtime-wasi and workerd-test"
note "  WAITS: wasmtime needs a download recorded by hash on first use (F25); workerd needs T-1658"
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: THE RUNTIME PROFILES HOLD (a WAITS clause aside)"
else
	note "verdict: A CLAUSE DISAGREED"
fi
rm -rf "$WORK"
exit "$fail"
