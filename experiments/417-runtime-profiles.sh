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
#   5. workerd-test (the fixture, no command): the test of
#      /workspace/worker.js passes; `typeof process` is undefined;
#      `node:fs` and `node:child_process` do not import; a `fetch` to a host
#      listener fails, and the listener counts no connection.
#   6. wasmtime-wasi: `hello.wat` of /workspace prints "hello from wasi",
#      and the only preopened directory that it lists is /workspace.
#
# Exit: 0 each clause matched, 1 a clause disagreed,
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
cat >"$WORK/proj/hello.wat" <<'EOF'
(module
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_prestat_get" (func $prestat (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_prestat_dir_name" (func $dirname (param i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 64) "hello from wasi\n")
  (data (i32.const 96) "preopen ")
  (data (i32.const 104) "\n")
  ;; write len bytes at ptr to stdout; iovec at 0, nwritten at 8
  (func $out (param $ptr i32) (param $len i32)
    (i32.store (i32.const 0) (local.get $ptr))
    (i32.store (i32.const 4) (local.get $len))
    (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 8))))
  (func (export "_start") (local $fd i32) (local $n i32)
    (call $out (i32.const 64) (i32.const 16))
    (local.set $fd (i32.const 3))
    (block $done
      (loop $next
        ;; prestat at 16: tag (u8) then name length (u32) at 20
        (br_if $done (i32.ne (call $prestat (local.get $fd) (i32.const 16)) (i32.const 0)))
        (local.set $n (i32.load (i32.const 20)))
        (drop (call $dirname (local.get $fd) (i32.const 256) (local.get $n)))
        (call $out (i32.const 96) (i32.const 8))
        (call $out (i32.const 256) (local.get $n))
        (call $out (i32.const 104) (i32.const 1))
        (local.set $fd (i32.add (local.get $fd) (i32.const 1)))
        (br $next)))))
EOF
cat >"$WORK/proj/worker.js" <<'EOF'
export default {
  async test(ctrl, env, ctx) {
    console.log("typeof process = " + typeof process);
    for (const m of ["node:fs", "node:child_process"]) {
      try { await import(m); console.log("import " + m + " ok"); } catch (e) { console.log("import " + m + " failed"); }
    }
    try { await fetch("http://127.0.0.1:PORT_417/"); console.log("fetch connected"); }
    catch (e) { console.log("fetch failed"); }
  }
};
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

note "== clause 5  workerd-test (no outbound service, no TCP connect)"
command -v python3 >/dev/null 2>&1 || { note "python3 missing: COULD NOT RUN"; exit 2; }
cat >"$WORK/listen.py" <<'EOF'
import socket, sys, threading
s = socket.socket(); s.bind(("127.0.0.1", 0)); s.listen(8)
print(s.getsockname()[1], flush=True)
n = 0
def serve():
    global n
    while True:
        c, _ = s.accept(); n += 1
        open(sys.argv[1], "w").write(str(n))
        c.close()
threading.Thread(target=serve, daemon=True).start()
threading.Event().wait()
EOF
echo 0 >"$WORK/conns"
python3 "$WORK/listen.py" "$WORK/conns" >"$WORK/port" 2>/dev/null &
LPID=$!
i=0; while [ ! -s "$WORK/port" ] && [ "$i" -lt 50 ]; do sleep 0.1; i=$((i + 1)); done
sed -i "s/PORT_417/$(cat "$WORK/port")/" "$WORK/proj/worker.js"
fx run --rm --profile workerd-test >"$WORK/c5.out" 2>&1
rc=$?
kill "$LPID" 2>/dev/null
note "  exit $rc: $(grep -E '^(typeof|import|fetch)' "$WORK/c5.out" | tr '\n' ';') $(grep -o '\[ PASS \] main' "$WORK/c5.out")"
note "  connections to the host listener: $(cat "$WORK/conns")"
[ "$rc" -eq 0 ] || { bad "the workerd-test run exited $rc"; grep -v '^  ' "$WORK/c5.out" | tail -3 | sed 's/^/    /' | tee -a "$OUT"; }
grep -q '\[ PASS \] main' "$WORK/c5.out" || bad "the test of the worker did not pass"
grep -q '^typeof process = undefined' "$WORK/c5.out" || bad "process is defined"
grep -q '^import node:fs failed' "$WORK/c5.out" || bad "node:fs imported"
grep -q '^import node:child_process failed' "$WORK/c5.out" || bad "node:child_process imported"
grep -q '^fetch failed' "$WORK/c5.out" || bad "the fetch did not fail"
[ "$(cat "$WORK/conns")" = 0 ] || bad "the worker connected to the host listener"
note ""

note "== clause 6  wasmtime-wasi"
fx run --rm --profile wasmtime-wasi -- hello.wat >"$WORK/c6.out" 2>&1
rc=$?
note "  exit $rc: $(grep -E '^(hello|preopen)' "$WORK/c6.out" | tr '\n' ';')"
[ "$rc" -eq 0 ] || { bad "the wasmtime-wasi run exited $rc"; grep -v '^  ' "$WORK/c6.out" | tail -3 | sed 's/^/    /' | tee -a "$OUT"; }
grep -qx 'hello from wasi' "$WORK/c6.out" || bad "the module did not print hello from wasi"
[ "$(grep -c '^preopen ' "$WORK/c6.out")" = 1 ] || bad "the module does not see one preopened directory"
grep -qx 'preopen /workspace' "$WORK/c6.out" || bad "/workspace is not the preopened directory"
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: THE RUNTIME PROFILES HOLD"
else
	note "verdict: A CLAUSE DISAGREED"
fi
rm -rf "$WORK"
exit "$fail"
