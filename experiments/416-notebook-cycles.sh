#!/bin/sh
# Question: does `podvm notebook` run cycles of a notebook that keep only
# the --persist directory, restart the kernel where asked, end a cycle at
# --kill-after with no process left, and export a marimo file?
#
# TODO/cli.md T-1660, issue 174. Each run goes through the fixture of
# experiment 356 (user and mount namespaces, /dev/pts hidden, chroot(2)
# denied), so each kernel runs on the userland rung with a glibc image.
#
# Pinned inputs: quay.io/jupyter/base-notebook:latest and
# ghcr.io/marimo-team/marimo:latest, the images of the verb.
#
# Clauses:
#   1. --cycles 3 --restart-kernel-after 1: cycle N counts N lines in
#      /persist, one in /tmp, and /workspace holds only the notebook; after
#      the restart `x` is gone; a cell raised, so the exit is 1.
#   2. The control of clause 1, one cycle with no restart: `x` stays.
#   3. --kill-after 5 on a sleeping cell exits 143, and no process of the
#      cycle and no container tree stays.
#   4. --kill-after 45: the kernel ran before the bound (an ipykernel
#      process was seen), the exit is 143, and no process stays.
#   5. A marimo file exports to HTML that holds the output of a cell.
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep416-work"
rm -rf "$WORK"; mkdir -p "$WORK/nb" || exit 2
OUT="$REPO/experiments/results/notebook-cycles.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
export PODVM_STORE="$WORK/store"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }

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
fx() { (cd "$WORK/nb" && timeout 1200 unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$PB" notebook "$@"); }
gone() { ! pgrep -f "ipykernel|nbconvert" >/dev/null 2>&1; }

cat >"$WORK/nb/c.ipynb" <<'EOF'
{"cells":[
{"cell_type":"code","execution_count":null,"id":"a","metadata":{},"outputs":[],"source":["import os\n","x = 41\n","open('/persist/n.txt','a').write('c\\n')\n","open('/tmp/t.txt','a').write('t\\n')\n","print('persist', len(open('/persist/n.txt').read().split()), 'tmp', len(open('/tmp/t.txt').read().split()))\n","print('workspace', sorted(os.listdir('/workspace')))"]},
{"cell_type":"code","execution_count":null,"id":"b","metadata":{},"outputs":[],"source":["print('x kept', 'x' in globals())\n","print('persist after', len(open('/persist/n.txt').read().split()))"]},
{"cell_type":"code","execution_count":null,"id":"c","metadata":{},"outputs":[],"source":["1/0"]}
],"metadata":{"kernelspec":{"display_name":"Python 3","language":"python","name":"python3"}},"nbformat":4,"nbformat_minor":5}
EOF
cat >"$WORK/nb/s.ipynb" <<'EOF'
{"cells":[{"cell_type":"code","execution_count":null,"id":"a","metadata":{},"outputs":[],"source":["import time\n","time.sleep(600)"]}],"metadata":{"kernelspec":{"display_name":"Python 3","language":"python","name":"python3"}},"nbformat":4,"nbformat_minor":5}
EOF
cat >"$WORK/nb/m.py" <<'EOF'
import marimo

app = marimo.App()


@app.cell
def _():
    x = 6 * 7
    return (x,)


@app.cell
def _(x):
    f"answer {x}"
    return


if __name__ == "__main__":
    app.run()
EOF

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "fixture           unshare -Urm, /dev/pts hidden, chroot(2) denied"
note ""

note "== clause 1  three cycles, a kernel restart after cell 1"
fx c.ipynb --persist keep --cycles 3 --restart-kernel-after 1 >"$WORK/c1.out" 2>"$WORK/c1.err"
rc=$?
note "  exit $rc"
sed 's/^/    /' "$WORK/c1.out" | tee -a "$OUT"
[ "$rc" -eq 1 ] || bad "the exit is $rc, not 1 (a cell raised)"
for n in 1 2 3; do
	grep -qx "\[$n\] persist $n tmp 1" "$WORK/c1.out" || bad "cycle $n does not count $n in /persist and 1 in /tmp"
	grep -qx "\[$n\] workspace \['c.ipynb'\]" "$WORK/c1.out" || bad "the /workspace of cycle $n holds more than the notebook"
	grep -qx "\[$n\] x kept False" "$WORK/c1.out" || bad "x survived the restart of cycle $n"
	grep -qx "\[$n\] persist after $n" "$WORK/c1.out" || bad "/persist did not survive the restart of cycle $n"
	grep -qx "\[$n\] ZeroDivisionError: division by zero" "$WORK/c1.out" || bad "cycle $n did not report the raise"
done
note ""

note "== clause 2  the control: no restart"
fx c.ipynb --persist keep2 >"$WORK/c2.out" 2>"$WORK/c2.err"
note "  exit $?"
grep 'x kept' "$WORK/c2.out" | sed 's/^/    /' | tee -a "$OUT"
grep -qx "\[1\] x kept True" "$WORK/c2.out" || bad "x is gone with no restart"
note ""

note "== clause 3  --kill-after 5 on a sleeping cell"
fx s.ipynb --persist keep3 --kill-after 5 >"$WORK/c3.out" 2>"$WORK/c3.err"
rc=$?
sleep 1
note "  exit $rc: $(cat "$WORK/c3.out")"
[ "$rc" -eq 143 ] || bad "the exit is $rc, not 143"
gone || bad "a process of the cycle stays"
t=$(ls "$WORK/store/trees" 2>/dev/null | wc -l)
note "  container trees left: $t"
[ "$t" -eq 0 ] || bad "a container tree stays"
note ""

note "== clause 4  --kill-after 45: the kernel ran, then ended"
fx s.ipynb --persist keep4 --kill-after 45 >"$WORK/c4.out" 2>"$WORK/c4.err" &
bg=$!
seen=0
for _ in $(seq 1 90); do
	pgrep -f ipykernel >/dev/null 2>&1 && seen=1
	kill -0 "$bg" 2>/dev/null || break
	sleep 0.5
done
wait "$bg"
rc=$?
sleep 1
note "  exit $rc; an ipykernel process was seen: $seen"
[ "$rc" -eq 143 ] || bad "the exit is $rc, not 143"
[ "$seen" -eq 1 ] || bad "no kernel ran before the bound"
gone || bad "a process of the cycle stays"
note ""

note "== clause 5  a marimo file exports to HTML"
fx m.py --persist keep5 >"$WORK/c5.out" 2>"$WORK/c5.err"
rc=$?
note "  exit $rc: $(sed "s|$REPO|\$REPO|" "$WORK/c5.out")"
[ "$rc" -eq 0 ] || bad "the export exited $rc"
grep -q "answer 42" "$WORK/nb/m.html" 2>/dev/null || bad "the HTML does not hold the output of the cell"
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: THE NOTEBOOK CYCLES HOLD"
else
	note "verdict: A CLAUSE DISAGREED"
	tail -n 5 "$WORK"/c*.err >&2
fi
rm -rf "$WORK"
exit "$fail"
