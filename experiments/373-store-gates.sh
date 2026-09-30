#!/bin/sh
# Question: does the store gate fetching and entry, mark missing blobs, and
# recover, with the image Config visible?
#
# TODO/image.md T-1417 (inspect Config), T-1418 (store-exec gate) and
# T-1419 (missing-blob refusal and mark).
#
# The store is not a directory that happens to hold files: payloads
# execute from it, so a filesystem that refuses execution must refuse
# before any byte is fetched, and a record whose blobs are gone must
# refuse rather than run short. Pinned input is alpine 3.20 below.
#
# Clauses:
#   1. empty store: `images` exits 0 with no rows and no stderr mark;
#      `doctor store_exec` prints yes at 0.
#   2. Config: `inspect --format '{{.Config.Env}}'` carries PATH= and
#      `{{.Config.Cmd}}` carries /bin/sh; the JSON document has Config.
#   3. noexec store: on a noexec tmpfs `doctor store_exec` prints no at
#      1, and `run` refuses pre-pull at 125 naming the directory and
#      `$PODBOX_STORE`.
#   4. missing blob: deleting one blob makes `run --pull never` refuse
#      at 125 naming `--pull always`, marks the record on `images`
#      stderr, and fails `verify` at 125; `run --pull always`
#      re-fetches and both run and verify return to 0.
#   5. transport (T-1420): a DNS failure names the host once in plain
#      words, never doubling the URL and never with ureq capitalisation.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could
# not run (no toolchain, no build, no pull, no namespaces).
set -u

REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep373-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-373.txt"

ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "input             $ALPINE"
} >"$REPORT"

command -v cargo >/dev/null 2>&1 || { echo "cargo missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
unshare -Urm true >/dev/null 2>&1 || { echo "user+mount namespaces refused: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
echo "== bootstrap the lane toolchain"
if timeout 1200 ./scripts/common/bootstrap-env.sh rust cc zig tools >>"$WORK/bootstrap.log" 2>&1; then
	echo "bootstrap         ok" >>"$REPORT"
else
	echo "bootstrap         FAILED" | tee -a "$REPORT"
	tail -15 "$WORK/bootstrap.log"
	exit 2
fi
echo "== interposer objects"
if timeout 1200 ./scripts/build-interpose.sh >>"$WORK/interpose.log" 2>&1; then
	echo "interpose         ok" >>"$REPORT"
else
	echo "interpose         FAILED" | tee -a "$REPORT"
	tail -15 "$WORK/interpose.log"
	exit 2
fi
echo "== build the debug binary"
if timeout 1800 cargo build >>"$WORK/build.log" 2>&1; then
	echo "build             ok" >>"$REPORT"
else
	echo "build             FAILED" | tee -a "$REPORT"
	tail -15 "$WORK/build.log"
	exit 1
fi
PB="$REPO/target/x86_64-unknown-linux-musl/debug/podbox"
[ -x "$PB" ] || { echo "binary missing after build: FAILED" | tee -a "$REPORT"; exit 1; }
echo "podbox            $("$PB" version)" >>"$REPORT"

# shellcheck source=../scripts/common/exit-codes.sh
. "$REPO/scripts/common/exit-codes.sh"
podbox_exit_codes "$PB" || { echo "cannot read podbox's exit-code table: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
RUN_ERR=$PODBOX_EXIT_RUNTIME_ERROR

STORE="$WORK/store"
mkdir -p "$STORE" || exit 2
export PODBOX_STORE="$STORE"
fail=0

step() {
	name="$1"; want="$2"; shift 2
	out="$WORK/out-$name.txt"
	timeout 120 "$@" >"$out" 2>&1
	rc=$?
	echo "step $name exit $rc"
	{
	echo ""
	echo "== $name"
	echo "command           $*"
	echo "exit              $rc"
	echo "output:"
	sed 's/^/  /' "$out"
	} >>"$REPORT"
	[ "$rc" -eq "$want" ] || { echo "step $name: wanted exit $want, got $rc" >>"$REPORT"; fail=1; }
}

# Clause 1 (empty store, T-1419): a store holding nothing lists nothing
# and marks nothing; the exec row passes where files execute.
ESTORE="$WORK/empty-store"
mkdir -p "$ESTORE" || exit 2
step empty-images 0 env PODBOX_STORE="$ESTORE" "$PB" images
grep -q "alpine" "$WORK/out-empty-images.txt" && { echo "empty store listed rows" >>"$REPORT"; fail=1; }
grep -q "missing from the" "$WORK/out-empty-images.txt" && { echo "empty store marked a record" >>"$REPORT"; fail=1; }
step empty-doctor 0 env PODBOX_STORE="$ESTORE" "$PB" doctor store_exec
grep -q "store_exec: yes" "$WORK/out-empty-doctor.txt" || { echo "exec row did not pass on an executable store" >>"$REPORT"; fail=1; }

# The pull happens once, outside every gate below.
if ! timeout 600 "$PB" pull "$ALPINE" >>"$REPORT" 2>&1; then
	echo "pull $ALPINE FAILED: COULD NOT RUN" | tee -a "$REPORT"
	exit 2
fi

# Clause 2 (Config, T-1417): the image's declared Config reads through
# --format and the JSON document alike.
step cfg-env 0 "$PB" inspect --format '{{.Config.Env}}' "$ALPINE"
grep -q "PATH=" "$WORK/out-cfg-env.txt" || { echo "Config.Env has no PATH=" >>"$REPORT"; fail=1; }
step cfg-cmd 0 "$PB" inspect --format '{{.Config.Cmd}}' "$ALPINE"
grep -q "/bin/sh" "$WORK/out-cfg-cmd.txt" || { echo "Config.Cmd has no /bin/sh" >>"$REPORT"; fail=1; }
step cfg-json 0 "$PB" inspect "$ALPINE"
grep -q '"Config"' "$WORK/out-cfg-json.txt" || { echo "JSON inspect has no Config object" >>"$REPORT"; fail=1; }

# Clause 3 (noexec store, T-1418): nothing executes under a noexec
# mount, so the row says no and the entry refuses before any fetch,
# naming the directory and the remedy.
NX="$WORK/noexec-store"
mkdir -p "$NX" || exit 2
step nx-doctor 1 unshare -Urm sh -c "mount -t tmpfs -o noexec tmpfs '$NX' && PODBOX_STORE='$NX' '$PB' doctor store_exec"
grep -q "store_exec: no" "$WORK/out-nx-doctor.txt" || { echo "exec row did not say no on noexec" >>"$REPORT"; fail=1; }
step nx-run "$RUN_ERR" unshare -Urm sh -c "mount -t tmpfs -o noexec tmpfs '$NX' && PODBOX_STORE='$NX' '$PB' run --rm --pull never '$ALPINE' true"
grep -q "PODBOX_STORE" "$WORK/out-nx-run.txt" || { echo "noexec refusal did not name the remedy" >>"$REPORT"; fail=1; }
grep -q "$NX" "$WORK/out-nx-run.txt" || { echo "noexec refusal did not name the directory" >>"$REPORT"; fail=1; }

# Clause 4 (missing blob, T-1419): one deleted blob refuses the run,
# marks the images row, and fails verify; --pull always heals it.
BLOB="$(find "$STORE/blobs" -type f | head -n 1)"
[ -n "$BLOB" ] || { echo "no blob file to delete: FAILED" | tee -a "$REPORT"; exit 1; }
echo "deleted blob      $BLOB" >>"$REPORT"
rm -f "$BLOB" || exit 1
step blob-run "$RUN_ERR" "$PB" run --rm --pull never "$ALPINE" true
grep -q -- "--pull always" "$WORK/out-blob-run.txt" || { echo "blob refusal did not name the recovery" >>"$REPORT"; fail=1; }
step blob-images 0 "$PB" images
grep -q "missing from the" "$WORK/out-blob-images.txt" || { echo "images did not mark the short record" >>"$REPORT"; fail=1; }
step blob-verify "$RUN_ERR" "$PB" verify "$ALPINE"
step blob-heal 0 "$PB" run --rm --pull always "$ALPINE" echo healed-hi
grep -q "healed-hi" "$WORK/out-blob-heal.txt" || { echo "healed run stdout missing" >>"$REPORT"; fail=1; }
step blob-verify2 0 "$PB" verify "$ALPINE"

# Clause 5 (T-1420 transport): a DNS failure names the host once in plain
# words. The old shape doubled the URL with ureq's capitalisation.
BADREG='registry.invalid.example/no-such-image:latest'
step bad-reg "$RUN_ERR" "$PB" pull "$BADREG"
grep -q "registry.invalid.example" "$WORK/out-bad-reg.txt" || { echo "transport error did not name the host" >>"$REPORT"; fail=1; }
grep -q "transport: https://" "$WORK/out-bad-reg.txt" && { echo "transport error doubled the URL" >>"$REPORT"; fail=1; }
grep -q "Dns Failed" "$WORK/out-bad-reg.txt" && { echo "transport error kept ureq capitalisation" >>"$REPORT"; fail=1; }

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           STORE GATES HOLD"; else echo "verdict           STORE GATES OPEN"; fi
} >>"$REPORT"

cp "$REPORT" "$REPO/experiments/results/store-gates.txt"
if [ -d /out ]; then cp "$REPORT" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
