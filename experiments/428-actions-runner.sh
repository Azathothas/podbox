#!/bin/sh
# Question: does the unmodified official actions/runner start under podvm
# on a host that denies chroot(2), with no --unsafe-host-paths?
#
# TODO/cli.md T-1706 (issue 231). The runner is a payload: podvm adds no
# runner flag. The test image is debian bookworm-slim with the runner's
# libraries (installed by apt on the capable rung of this host) and the
# runner tarball, checked against its pinned SHA-256, in /opt/runner. The
# fixture is experiments/deny-chroot.c under user and mount namespaces, as
# in experiment 356.
#
# Clauses:
#   1. `-w /opt/runner ./bin/Runner.Listener --version` prints the pinned
#      version on the fixture (a relative program, T-1707).
#   2. `./config.sh --help` exits 0 on the fixture.
#   3. under `prlimit --fsize=1073741824` the same --version run with W^X
#      on exits 153, and podvm names RLIMIT_FSIZE and
#      DOTNET_EnableWriteXorExecute=0; with
#      `-e DOTNET_EnableWriteXorExecute=0` it exits 0.
#   4. `./run.sh` with the three credential files of a synthetic blob
#      (mode 0600, staged in the image) against a loopback stand-in of the
#      service (experiments/runner-standin.py: POST /_apis/oauth2/token,
#      POST /session, GET /message) reaches the long poll ("Listening for
#      Jobs"), and the run ends within 30 s of SIGTERM to podvm.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could not
# run (no build, no pull, no download, no namespaces).
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep428-work"
rm -rf "$WORK"; mkdir -p "$WORK/out" || exit 2
REPORT="$WORK/out-428.txt"
VERSION=2.338.2
SHA=baebf204307bac6193480852732f0aba2ae97b6c5597c244f4d5558390147c40
TARBALL="actions-runner-linux-x64-$VERSION.tar.gz"
URL="https://github.com/actions/runner/releases/download/v$VERSION/$TARBALL"
DEBIAN='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
# ⚠ PODVM_428_MIRROR=1 takes library/debian:bookworm-slim from
# mirror.gcr.io by tag, as PODVM_356_MIRROR does; the conditions print the
# pulled digest.
if [ "${PODVM_428_MIRROR:-0}" = 1 ]; then
	DEBIAN='mirror.gcr.io/library/debian:bookworm-slim'
fi
LIBS="ca-certificates libicu72 libssl3 libkrb5-3 zlib1g liblttng-ust1"
{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "input             $DEBIAN"
echo "runner            $TARBALL sha256 $SHA"
echo "libraries         $LIBS"
} >"$REPORT"

for t in cargo cc curl sha256sum prlimit tar openssl python3 pgrep; do
	command -v "$t" >/dev/null 2>&1 || { echo "$t missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
done
unshare -Urm true >/dev/null 2>&1 || { echo "user+mount namespaces refused: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
if timeout 1200 ./scripts/build-interpose.sh >"$WORK/interpose.log" 2>&1 &&
	timeout 1800 cargo build >"$WORK/build.log" 2>&1; then
	echo "build             ok" >>"$REPORT"
else
	echo "build             FAILED" | tee -a "$REPORT"; tail -15 "$WORK/build.log"; exit 2
fi
PB="$REPO/target/x86_64-unknown-linux-musl/debug/podvm"
echo "podvm             $("$PB" version)" >>"$REPORT"
export PODVM_STORE="$WORK/store"

timeout 900 curl -qfsSL -o "$WORK/$TARBALL" "$URL" || { echo "download FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
echo "$SHA  $WORK/$TARBALL" | sha256sum -c - >/dev/null 2>&1 || { echo "runner sha256 MISMATCH: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
timeout 600 "$PB" pull "$DEBIAN" >>"$REPORT" 2>&1 || { echo "pull FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
echo "debian digest     $("$PB" inspect --format '{{.Digest}}' "$DEBIAN" 2>/dev/null)" >>"$REPORT"

# The image: apt installs the libraries on the capable rung, into the
# extracted tree of the image (a named run, not --rm, keeps that tree); the
# host tars the tree, and the runner goes under /opt/runner. A volume is
# refused on the capable rung (T-1653), so the tar runs on the host.
timeout 300 "$PB" extract "$DEBIAN" >>"$WORK/image.log" 2>&1 || { echo "extract FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
DROOT=$("$PB" inspect --format '{{.RootfsPath}}' "$DEBIAN" 2>/dev/null)
[ -d "$DROOT" ] || { echo "debian rootfs missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
timeout 1200 "$PB" run --name build428 -e DEBIAN_FRONTEND=noninteractive "$DEBIAN" sh -c \
	"apt-get -o APT::Sandbox::User=root update >/dev/null && \
	 apt-get -o APT::Sandbox::User=root install -y --no-install-recommends $LIBS >/dev/null && \
	 rm -rf /var/lib/apt/lists/*" \
	>>"$WORK/image.log" 2>&1 || { echo "image build FAILED: COULD NOT RUN" | tee -a "$REPORT"; tail -15 "$WORK/image.log"; exit 2; }
"$PB" rm build428 >/dev/null 2>&1
tar -C "$DROOT" --exclude=./proc --exclude=./sys -cf "$WORK/out/root.tar" . || exit 2
mkdir -p "$WORK/add/opt/runner"
tar -xzf "$WORK/$TARBALL" -C "$WORK/add/opt/runner" || exit 2
# The port of the stand-in, and the three files of a synthetic blob.
PORT=$((18000 + $$ % 1000))
python3 -I "$REPO/experiments/runner-standin.py" files "$PORT" "$WORK/add/opt/runner" ||
	{ echo "credential files FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
chmod 0600 "$WORK/add/opt/runner/.runner" "$WORK/add/opt/runner/.credentials" "$WORK/add/opt/runner/.credentials_rsaparams"
tar -rf "$WORK/out/root.tar" -C "$WORK/add" ./opt || exit 2
timeout 300 "$PB" import "$WORK/out/root.tar" runner-428:1 >>"$REPORT" 2>&1 || { echo "import FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
rm -f "$WORK/out/root.tar"
echo "image             ok" >>"$REPORT"

cc -O2 -Wall -Werror -static -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>>"$REPORT" ||
	cc -O2 -Wall -Werror -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>>"$REPORT" ||
	{ echo "fixture COMPILE FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
cat >"$WORK/wrap.sh" <<'EOS'
#!/bin/sh
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOS
FX="unshare -Urm sh $WORK/wrap.sh $WORK"
$FX "$PB" system info --format '{{.EnteredRung}}' >"$WORK/rung.txt" 2>&1
echo "fixture rung      $(cat "$WORK/rung.txt")" >>"$REPORT"
fail=0
grep -qx userland "$WORK/rung.txt" || { echo "the fixture does not deny chroot" >>"$REPORT"; fail=1; }
note() { echo "$*" >>"$REPORT"; }
# The report is ASCII (docs/conventions/prose.md): the runner prints
# glyphs such as U+221A, and each other byte goes.
ascii() { LC_ALL=C tr -cd '\11\12\15\40-\176'; }
NOISE="Extracted\|cannot apply\|restore ownership\|no mapping\|not mapped\|intended metadata\|reads as a bad"

# One step: $1 the name, $2 the wanted exit, the rest the command.
step() {
	name="$1"; want="$2"; shift 2
	out="$WORK/out-$name.txt"
	timeout 300 "$@" >"$out" 2>&1
	rc=$?
	{ echo ""; echo "== $name"; echo "exit              $rc"; grep -v "$NOISE" "$out" | tail -20 | ascii | sed 's/^/  /'; } >>"$REPORT"
	[ "$rc" -eq "$want" ] || { note "$name: wanted exit $want, got $rc"; fail=1; }
}

RUN="$PB run --rm -e RUNNER_ALLOW_RUNASROOT=1 -w /opt/runner"
step version 0 $FX $RUN runner-428:1 ./bin/Runner.Listener --version
grep -qx "$VERSION" "$WORK/out-version.txt" || { note "version: $VERSION not printed"; fail=1; }
step config-help 0 $FX $RUN runner-428:1 ./config.sh --help
step fsize-wx 153 prlimit --fsize=1073741824 $FX $RUN runner-428:1 ./bin/Runner.Listener --version
grep -q "RLIMIT_FSIZE of 1073741824 bytes" "$WORK/out-fsize-wx.txt" || { note "fsize-wx: RLIMIT_FSIZE not named"; fail=1; }
grep -q "DOTNET_EnableWriteXorExecute=0" "$WORK/out-fsize-wx.txt" || { note "fsize-wx: the W^X switch not named"; fail=1; }
step fsize-nowx 0 prlimit --fsize=1073741824 $FX $RUN -e DOTNET_EnableWriteXorExecute=0 runner-428:1 ./bin/Runner.Listener --version

# Clause 4: run.sh to the long poll of the stand-in, then SIGTERM.
LOG="$WORK/standin.log"; : >"$LOG"
python3 -I "$REPO/experiments/runner-standin.py" serve "$PORT" "$LOG" &
sp=$!
sleep 1
out="$WORK/out-run.txt"
$FX $RUN runner-428:1 ./run.sh >"$out" 2>&1 &
rp=$!
i=0; while [ "$i" -lt 180 ] && ! grep -q "GET /message" "$LOG"; do sleep 1; i=$((i + 1)); done
grep -q "GET /message" "$LOG" || { note "run: the long poll was not reached in 180 s"; fail=1; }
sleep 3
pv=$(pgrep -n -f "podvm run --rm .*runner-428:1 ./run.sh") || pv=""
start=$(date +%s)
if [ -n "$pv" ]; then kill -TERM "$pv"; else note "run: podvm not found"; fail=1; fi
i=0; while [ "$i" -lt 60 ] && kill -0 "$rp" 2>/dev/null; do sleep 1; i=$((i + 1)); done
took=$(( $(date +%s) - start ))
kill -0 "$rp" 2>/dev/null && { note "run: still running 60 s after SIGTERM"; pkill -9 -f "runner-428:1 ./run.sh"; fail=1; }
wait "$rp" 2>/dev/null
kill "$sp" 2>/dev/null; wait "$sp" 2>/dev/null
{ echo ""; echo "== run"; echo "stand-in          $(cut -d' ' -f2- "$LOG" | sort | uniq -c | tr '\n' ';')"; echo "end after SIGTERM ${took} s"; grep -v "$NOISE" "$out" | tail -15 | ascii | sed 's/^/  /'; } >>"$REPORT"
grep -q "Listening for Jobs" "$out" || { note "run: no 'Listening for Jobs'"; fail=1; }
[ "$took" -le 30 ] || { note "run: the end took ${took} s after SIGTERM, over 30"; fail=1; }
pgrep -f "Runner.Listener" >/dev/null 2>&1 && { note "run: a Runner.Listener stayed"; pkill -9 -f Runner.Listener; fail=1; }

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           THE RUNNER STARTS ON THE USERLAND RUNG"; else echo "verdict           A CLAUSE DISAGREED"; fi
} >>"$REPORT"
cp "$REPORT" "$REPO/experiments/results/actions-runner.txt"
cat "$REPORT"
[ "$fail" -eq 0 ]
