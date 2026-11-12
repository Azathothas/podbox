#!/bin/sh
# Question: does headless Chromium of Playwright run under podvm, confined,
# where chroot is denied?
#
# TODO/cli.md T-1694 (A-V-05 of PodUtils/podcrates#39); T-1738 (the
# allocator of the interposer) and T-1739 (the children of the browser).
# The profile playwright-chromium runs chrome-headless-shell of
# mcr.microsoft.com/playwright:v1.65.0-noble (pinned by digest) with
# --no-sandbox, in its default mode (a zygote and child processes, T-1739),
# under the confine layer (T-1658), on the loader family of the userland
# rung.
#
# Fixture: the chroot-denying wrapper of experiment 356.
#
# Clauses:
#   1. `run --rm --profile playwright-chromium -- --dump-dom
#      file:///workspace/i.html` prints the DOM of the page.
#   2. `--screenshot=/workspace/s.png` writes a PNG of more than 0 bytes.
#   3. with `-s net.ports=[9]` the layer holds TCP to port 9, and a page
#      that loads an image from a host listener makes no connection to it;
#      the control with no port rule connects.
#   4. no process of the browser stays after the runs.
#   5. the profile runs the default mode: its command has no
#      --single-process, so the DOM of clause 1 came from a renderer child
#      (T-1739).
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep425-work"
rm -rf "$WORK"; mkdir -p "$WORK/proj" || exit 2
OUT="$REPO/experiments/results/chromium.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
export PODVM_STORE="${PODVM_425_STORE:-$WORK/store}"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }
LPID=""
trap '[ -n "$LPID" ] && kill "$LPID" 2>/dev/null; rm -rf "$WORK"' EXIT INT TERM

[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "python3 missing: COULD NOT RUN" >&2; exit 2; }
cc -O2 -Wall -Werror -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" || { echo "fixture COMPILE FAILED: COULD NOT RUN" >&2; exit 2; }
cat >"$WORK/wrap.sh" <<'EOF'
#!/bin/sh
mount -t tmpfs tmpfs /dev/pts || exit 2
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF
fx() { (cd "$WORK/proj" && timeout 300 unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$PB" "$@"); }

# A host listener that counts its connections.
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
PORT=$(cat "$WORK/port")
[ -n "$PORT" ] || { echo "the listener did not start: COULD NOT RUN" >&2; exit 2; }
echo '<html><body><h1 id="t">hello-425</h1></body></html>' >"$WORK/proj/i.html"
echo "<html><body><p id=\"n\">net-425</p><img src=\"http://127.0.0.1:$PORT/x.png\"></body></html>" >"$WORK/proj/n.html"

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "fixture           unshare -Urm, /dev/pts hidden, chroot(2) denied"
note "profile           $("$PB" profile show playwright-chromium 2>/dev/null | grep -m1 -i image)"
note ""

note "== clause 1  the DOM of a page of /workspace"
t0=$(date +%s)
fx run --rm --profile playwright-chromium -- --dump-dom file:///workspace/i.html >"$WORK/c1.out" 2>"$WORK/c1.err"
rc=$?
note "  exit $rc in $(( $(date +%s) - t0 )) s: $(grep -o '<h1[^<]*</h1>' "$WORK/c1.out")"
[ "$rc" -eq 0 ] || { bad "the run exited $rc"; grep -v rehash "$WORK/c1.err" | tail -3 | sed 's/^/    /' | tee -a "$OUT"; }
grep -q '<h1 id="t">hello-425</h1>' "$WORK/c1.out" || bad "the DOM is not printed"
note ""

note "== clause 2  a screenshot"
fx run --rm --profile playwright-chromium -- --screenshot=/workspace/s.png --window-size=400,300 file:///workspace/i.html >/dev/null 2>"$WORK/c2.err"
rc=$?
size=$(wc -c <"$WORK/proj/s.png" 2>/dev/null || echo 0)
note "  exit $rc; s.png: $size bytes, $(head -c 8 "$WORK/proj/s.png" 2>/dev/null | od -An -c | tr -s ' ' | cut -c1-20)"
[ "$rc" -eq 0 ] || bad "the screenshot run exited $rc"
[ "$size" -gt 0 ] || bad "no screenshot"
note ""

note "== clause 3  no network connection"
fx run --rm --profile playwright-chromium -s 'net.ports=[9]' -- --dump-dom file:///workspace/n.html >"$WORK/c3.out" 2>"$WORK/c3.err"
rc=$?
note "  exit $rc; the page: $(grep -o '<p[^<]*</p>' "$WORK/c3.out"); connections to the host listener: $(cat "$WORK/conns")"
grep -q 'net-425' "$WORK/c3.out" || bad "the page did not load"
[ "$(cat "$WORK/conns")" = 0 ] || bad "the browser connected to the host listener"
fx run --rm --profile playwright-chromium -- --dump-dom file:///workspace/n.html >/dev/null 2>&1
note "  control with no port rule: connections: $(cat "$WORK/conns")"
[ "$(cat "$WORK/conns")" -gt 0 ] || bad "the control made no connection: the clause proves nothing"
note ""

note "== clause 4  no process stays"
sleep 1
left=$(ps -eo args | grep -c '[c]hrome-headless-shell')
note "  processes of the browser left: $left"
[ "$left" -eq 0 ] || bad "a process of the browser stayed"
note ""

note "== clause 5  the default mode, with child processes"
cmd=$("$PB" profile show playwright-chromium 2>/dev/null | grep '^# the command runs under')
note "  $(echo "$cmd" | cut -c1-160)"
case "$cmd" in
	*--single-process*) bad "the profile still runs one process" ;;
	"") bad "profile show printed no command" ;;
esac
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: HEADLESS CHROMIUM RUNS CONFINED WHERE CHROOT IS DENIED"
else
	note "verdict: A CLAUSE DISAGREED"
fi
exit "$fail"
