#!/bin/sh
# Question: does `podvm run --podvm-tier=machine IMAGE CMD` boot a guest from
# the OCI image, run the command, and return its exit code, again and
# again, and refuse KVM by name where /dev/kvm does not open?
#
# TODO/machine.md T-1656. The guest kernel is the pinned vmlinuz-virt of the
# Alpine netboot archive 3.22.6 (podvm fetches it into the store); the
# initrd is the cached base archive of the image plus a run archive with
# /podvm-init. The pinned input is the alpine digest of experiment 356.
#
# Clauses:
#   1. `sh -c 'uname -r; exit 7'` prints a kernel release and exits 7; the
#      banner names the accelerator.
#   2. a second run says that the base archive came from the cache.
#   3. PODVM_412_BOOTS runs in a row (20 by default) each reach the command
#      and give its code.
#   4. PODVM_MACHINE_ACCEL=kvm where /dev/kvm does not open exits 125 by
#      name and boots nothing (where /dev/kvm opens, the clause records
#      that it ran under KVM).
#   5. the guest fetches a URL through an HTTP proxy that listens on the
#      host's loopback (10.0.2.2 in the guest): `http_proxy` sends busybox
#      wget there, and the proxy answers with the URL that it got.
#   6. `-v HOSTDIR:/workspace` (T-1723): the guest reads a host file
#      there, the banner says that the volume is a 9p mount, a file that
#      the guest writes there is on the host after the run, and `-w` in the
#      volume works. A second volume with `:ro` refuses a write with EROFS.
#   7. stderr comes back apart from stdout: `echo out; echo err >&2`
#      gives `out` on podvm's stdout and `err` on its stderr only.
#   8. stdin: 100000 random bytes piped in give the same SHA-256 in the
#      guest; stdin from /dev/null reads its end at once.
#   9. no emulator stays after the runs.
#  10. without -t, the command's stdout and stderr are not terminals
#      (T-1728): `[ -t 1 ]` and `[ -t 2 ]` are false, and `ls` writes no
#      colour escape.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could not
# run (no build, no pull, no qemu).
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep412-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-412.txt"
BOOTS=${PODVM_412_BOOTS:-20}
ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
# ⚠ PODVM_412_MIRROR=1 takes the same digest from mirror.gcr.io, as
# PODVM_356_MIRROR does.
if [ "${PODVM_412_MIRROR:-0}" = 1 ]; then
	ALPINE='mirror.gcr.io/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
fi
{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "input             $ALPINE"
echo "boots             $BOOTS"
echo "/dev/kvm          $([ -r /dev/kvm ] && [ -w /dev/kvm ] && echo opens || echo 'does not open')"
} >"$REPORT"

command -v cargo >/dev/null 2>&1 || { echo "cargo missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
command -v qemu-system-x86_64 >/dev/null 2>&1 || { echo "qemu-system-x86_64 missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
echo "qemu              $(qemu-system-x86_64 --version | head -1)" >>"$REPORT"
if timeout 1800 cargo build >"$WORK/build.log" 2>&1; then
	echo "build             ok" >>"$REPORT"
else
	echo "build             FAILED" | tee -a "$REPORT"; tail -15 "$WORK/build.log"; exit 2
fi
PB="$REPO/target/x86_64-unknown-linux-musl/debug/podvm"
echo "podvm             $("$PB" version)" >>"$REPORT"
export PODVM_STORE="$WORK/store"
timeout 600 "$PB" pull "$ALPINE" >>"$REPORT" 2>&1 || { echo "pull FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
fail=0
note() { echo "$*" >>"$REPORT"; }

# Clause 1.
start=$(date +%s)
timeout 900 "$PB" run --podvm-tier=machine "$ALPINE" sh -c 'uname -r; exit 7' >"$WORK/c1.out" 2>"$WORK/c1.err"
rc=$?
took=$(( $(date +%s) - start ))
{ echo ""; echo "== 1. one command"; echo "exit              $rc"; echo "seconds           $took (with the kernel fetch and the base archive)"; sed 's/^/  /' "$WORK/c1.out" "$WORK/c1.err"; } >>"$REPORT"
[ "$rc" -eq 7 ] || { note "1: wanted exit 7, got $rc"; fail=1; }
grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+' "$WORK/c1.out" || { note "1: no kernel release printed"; fail=1; }
grep -q "a guest under \(KVM\|TCG\)" "$WORK/c1.err" || { note "1: the banner names no accelerator"; fail=1; }

# Clause 2.
timeout 900 "$PB" run --podvm-tier=machine "$ALPINE" true >/dev/null 2>"$WORK/c2.err"
rc=$?
{ echo ""; echo "== 2. the cache"; echo "exit              $rc"; sed 's/^/  /' "$WORK/c2.err"; } >>"$REPORT"
[ "$rc" -eq 0 ] || { note "2: wanted exit 0, got $rc"; fail=1; }
grep -q "base archive from the cache" "$WORK/c2.err" || { note "2: the base archive was not reused"; fail=1; }

# Clause 3.
ok=0; i=0; start=$(date +%s)
while [ "$i" -lt "$BOOTS" ]; do
	i=$((i + 1))
	want=$((i % 100))
	out=$(timeout 300 "$PB" run --podvm-tier=machine "$ALPINE" sh -c "echo boot-$i; exit $want" 2>>"$WORK/c3.err")
	rc=$?
	if [ "$rc" -eq "$want" ] && [ "$out" = "boot-$i" ]; then ok=$((ok + 1)); else note "3: boot $i gave exit $rc, out '$out'"; fi
done
took=$(( $(date +%s) - start ))
{ echo ""; echo "== 3. boots in a row"; echo "reached           $ok of $BOOTS"; echo "seconds           $took"; } >>"$REPORT"
[ "$ok" -eq "$BOOTS" ] || fail=1

# Clause 4.
PODVM_MACHINE_ACCEL=kvm timeout 300 "$PB" run --podvm-tier=machine "$ALPINE" echo booted >"$WORK/c4.out" 2>"$WORK/c4.err"
rc=$?
{ echo ""; echo "== 4. KVM asked"; echo "exit              $rc"; sed 's/^/  /' "$WORK/c4.out" "$WORK/c4.err"; } >>"$REPORT"
if [ -r /dev/kvm ] && [ -w /dev/kvm ]; then
	[ "$rc" -eq 0 ] && grep -q "a guest under KVM" "$WORK/c4.err" || { note "4: KVM opens, and the run did not boot under KVM"; fail=1; }
else
	[ "$rc" -eq 125 ] || { note "4: wanted exit 125, got $rc"; fail=1; }
	grep -q "does not fall back to TCG" "$WORK/c4.err" || { note "4: the refusal does not name KVM"; fail=1; }
	grep -q "booted" "$WORK/c4.out" && { note "4: a guest booted"; fail=1; }
fi

# Clause 5: a stand-in proxy on the host's loopback answers each GET of
# an absolute URL with that URL.
PORT=$((19000 + $$ % 1000))
python3 -I -c '
import http.server, sys
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        b = ("proxied " + self.path + "\n").encode()
        self.send_response(200); self.send_header("Content-Length", str(len(b))); self.end_headers(); self.wfile.write(b)
    def log_message(self, *a): pass
http.server.HTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
' "$PORT" &
pp=$!
sleep 1
timeout 300 "$PB" run --podvm-tier=machine -e "http_proxy=http://10.0.2.2:$PORT" "$ALPINE" \
	wget -q -O- http://podvm-412.invalid/through-the-proxy >"$WORK/c5.out" 2>"$WORK/c5.err"
rc=$?
kill "$pp" 2>/dev/null; wait "$pp" 2>/dev/null
{ echo ""; echo "== 5. the guest network"; echo "exit              $rc"; sed 's/^/  /' "$WORK/c5.out"; grep -v "machine tier: a guest" "$WORK/c5.err" | sed 's/^/  /'; } >>"$REPORT"
[ "$rc" -eq 0 ] || { note "5: wanted exit 0, got $rc"; fail=1; }
grep -qx "proxied http://podvm-412.invalid/through-the-proxy" "$WORK/c5.out" || { note "5: the proxy did not answer the guest"; fail=1; }

# Clause 6: a volume is a 9p mount of the host directory (T-1723).
mkdir -p "$WORK/ws" "$WORK/ro" && echo host-file >"$WORK/ws/in.txt"
timeout 300 "$PB" run --podvm-tier=machine -v "$WORK/ws:/workspace" -v "$WORK/ro:/ro:ro" -w /workspace "$ALPINE" \
	sh -c 'cat in.txt; echo guest > out.txt && echo wrote; touch /ro/x 2>&1; true' >"$WORK/c6.out" 2>"$WORK/c6.err"
rc=$?
{ echo ""; echo "== 6. a volume"; echo "exit              $rc"; sed 's/^/  /' "$WORK/c6.out" "$WORK/c6.err"; } >>"$REPORT"
[ "$rc" -eq 0 ] || { note "6: wanted exit 0, got $rc"; fail=1; }
grep -qx "host-file" "$WORK/c6.out" || { note "6: the guest did not read the host file"; fail=1; }
grep -qx "wrote" "$WORK/c6.out" || { note "6: the guest did not write"; fail=1; }
grep -q "is a 9p mount of the host directory" "$WORK/c6.err" || { note "6: the run did not say that the volume is a 9p mount"; fail=1; }
[ "$(cat "$WORK/ws/out.txt" 2>/dev/null)" = guest ] || { note "6: the guest's write is not on the host"; fail=1; }
grep -q "Read-only file system" "$WORK/c6.out" || { note "6: a write in the :ro volume was not refused"; fail=1; }
[ -e "$WORK/ro/x" ] && { note "6: a write in the :ro volume is on the host"; fail=1; }

# Clause 7: stderr on its own port.
timeout 300 "$PB" run --podvm-tier=machine "$ALPINE" sh -c 'echo out-412; echo err-412 >&2; exit 4' >"$WORK/c7.out" 2>"$WORK/c7.err"
rc=$?
{ echo ""; echo "== 7. stderr apart"; echo "exit              $rc"; echo "  stdout:"; sed 's/^/    /' "$WORK/c7.out"; echo "  stderr:"; grep -v "machine tier: a guest" "$WORK/c7.err" | sed 's/^/    /'; } >>"$REPORT"
[ "$rc" -eq 4 ] || { note "7: wanted exit 4, got $rc"; fail=1; }
[ "$(cat "$WORK/c7.out")" = "out-412" ] || { note "7: stdout is not out-412 alone"; fail=1; }
grep -qx "err-412" "$WORK/c7.err" || { note "7: err-412 is not on stderr"; fail=1; }

# Clause 8: stdin.
head -c 100000 /dev/urandom >"$WORK/in.bin"
want=$(sha256sum <"$WORK/in.bin" | cut -d' ' -f1)
got=$(timeout 300 "$PB" run --podvm-tier=machine -i "$ALPINE" sha256sum <"$WORK/in.bin" 2>"$WORK/c8.err" | cut -d' ' -f1)
eof=$(timeout 300 "$PB" run --podvm-tier=machine "$ALPINE" sh -c 'cat; echo eof-ok' </dev/null 2>>"$WORK/c8.err")
{ echo ""; echo "== 8. stdin"; echo "host sha256       $want"; echo "guest sha256      $got"; echo "from /dev/null    $eof"; } >>"$REPORT"
[ -n "$got" ] && [ "$got" = "$want" ] || { note "8: the guest read other bytes"; fail=1; }
[ "$eof" = "eof-ok" ] || { note "8: stdin from /dev/null did not end"; fail=1; }

# Clause 10: no terminal without -t (T-1728).
timeout 300 "$PB" run --podvm-tier=machine "$ALPINE" sh -c '[ -t 1 ] && echo out-tty || echo out-pipe; [ -t 2 ] && echo err-tty >&2 || echo err-pipe >&2; ls /' </dev/null >"$WORK/c10.out" 2>"$WORK/c10.err"
rc=$?
{ echo ""; echo "== 10. no terminal"; echo "exit              $rc"; echo "  stdout line 1:   $(head -1 "$WORK/c10.out")"; echo "  stderr:          $(grep -x 'err-[a-z]*' "$WORK/c10.err")"; } >>"$REPORT"
[ "$rc" -eq 0 ] || { note "10: wanted exit 0, got $rc"; fail=1; }
[ "$(head -1 "$WORK/c10.out")" = "out-pipe" ] || { note "10: stdout is a terminal"; fail=1; }
grep -qx "err-pipe" "$WORK/c10.err" || { note "10: stderr is a terminal"; fail=1; }
grep -q "$(printf '\033')" "$WORK/c10.out" && { note "10: ls wrote a colour escape"; fail=1; }

# Clause 9.
if pgrep -f "qemu-system-x86_64.*podvm-init" >/dev/null 2>&1; then note "9: an emulator stayed"; pkill -9 -f "qemu-system-x86_64.*podvm-init"; fail=1; fi
ls -d "$WORK"/store/machine/run-* >/dev/null 2>&1 && { note "9: a run directory stayed"; fail=1; }

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           THE GUEST RUNS THE COMMAND"; else echo "verdict           A CLAUSE DISAGREED"; fi
} >>"$REPORT"
cp "$REPORT" "$REPO/experiments/results/microvm-boot.txt"
cat "$REPORT"
[ "$fail" -eq 0 ]
