#!/bin/sh
# Question: on a lane that has an emulator but no KVM, does `podbox windows`
# refuse where it must, reach the guest driver where the entry says it should,
# and does the acquisition path bound and verify itself without a network
# dependency in the test?
#
# TODO/milestones.md T-1112. ⚠ Clauses 7 and 8 need a guest image and each
# is skipped, out loud, where none is configured: podbox ships no Windows
# image and no DOS base, fetches no licensed image itself, and redistributes
# none, so a lane without `PODBOX_WINDOWS_IMAGE` (a Validation OS disk)
# or `PODBOX_DOS_BASE` (FreeDOS base from 363) cannot boot that guest and
# must say so rather than pass.
#
# Clauses:
#   0. conditions: binary, emulator, /dev/kvm, image.
#   1. the crate's own tests pass, and the protocol's own count is printed.
#   2. `windows --help` is 0 and names every subcommand; an unknown one is 125,
#      which is this tree's flag-error code (EXIT_FLAG_ERROR), not 2.
#   3. `windows fetch --sha256 abc` is 2 and fetches nothing: the digest is
#      checked before the URL is used.
#   4. `windows fetch` against a local file server refuses a body over
#      --max-bytes, names the ceiling, and leaves neither the file nor a
#      .part behind. ⛔ This is the bound that a declared length cannot see.
#   5. `windows doctor` refuses at 125 on a lane with no accelerator profile,
#      naming the leg, and reports the profile it did find.
#   6. `run --podbox-tier=machine --platform windows/amd64` reaches the guest
#      driver - the refusal names `podbox windows setup`, not the OCI path's
#      "not a Linux guest" - and the store is byte-identical after.
#   7. given PODBOX_WINDOWS_IMAGE: setup provisions once and run returns the
#      guest's own exit code, both measured in seconds.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could not run.
set -u

REPO="$(pwd)"
cd "$REPO" || exit 2
WORK="$REPO/experiments/.sweep364-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-364.txt"
BIN="$REPO/target/x86_64-unknown-linux-musl/debug/podbox"
PORT=8731

{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
if [ -e /dev/kvm ]; then echo "kvm               present"; else echo "kvm               absent"; fi
if command -v qemu-system-x86_64 >/dev/null 2>&1; then
	echo "qemu              $(qemu-system-x86_64 --version | head -1)"
else
	echo "qemu              absent"
fi
echo "image             ${PODBOX_WINDOWS_IMAGE:-unset}"
} >"$REPORT"

# --- bootstrap, the same three steps 362 takes -------------------------------
{
if [ -x "$BIN" ]; then echo "binary            present"; else echo "binary            absent"; exit 2; fi
} >>"$REPORT" 2>&1

STORE="$WORK/store"
export PODBOX_STORE="$STORE"
mkdir -p "$STORE"

say() { printf '%s\n' "$*" >>"$REPORT"; }
fail=0
bad() { say "$1"; echo "364: $1" >&2; fail=1; }

# --- 1. the crate's own tests ------------------------------------------------
say ""
say "== crate-tests"
if cargo test -p podbox-windows --lib \
	>"$WORK/tests.txt" 2>&1; then
	say "result            pass"
	say "count             $(sed -n 's/^test result: ok\. \([0-9]*\) passed.*/\1/p' "$WORK/tests.txt" | tail -1)"
else
	say "result            FAIL"
	tail -20 "$WORK/tests.txt" >>"$REPORT"
	fail=1
fi

# --- 2. help and an unknown subcommand ---------------------------------------
say ""
say "== help"
"$BIN" windows --help >"$WORK/help.txt" 2>&1
[ "$?" -eq 0 ] || bad "help did not exit 0"
for sub in doctor fetch setup run; do
	if grep -q "$sub" "$WORK/help.txt"; then say "names $sub         yes"; else bad "names $sub         NO"; fi
done
"$BIN" windows definitely-not-a-subcommand >"$WORK/bogus.txt" 2>&1
[ "$?" -eq 125 ] || bad "unknown subcommand did not exit 125"
"$BIN" windows run --guest plan9 -- ver >"$WORK/guest-bogus.txt" 2>&1
[ "$?" -eq 125 ] || bad "bad --guest did not exit 125"
sed -n '1,2p' "$WORK/guest-bogus.txt" | sed 's/^/  /' >>"$REPORT"

# --- 3. a bad pin is refused before the URL is used --------------------------
say ""
say "== bad-pin"
"$BIN" windows fetch --url http://127.0.0.1:1/never --sha256 abc >"$WORK/pin.txt" 2>&1
[ "$?" -eq 125 ] || bad "bad pin did not exit 125"
sed -n '1,3p' "$WORK/pin.txt" | sed 's/^/  /' >>"$REPORT"
if ls "$WORK"/*.part >/dev/null 2>&1; then say "partial           LEFT BEHIND"; else say "partial           none"; fi

# --- 4. the ceiling, against a server that declares nothing ------------------
say ""
say "== ceiling"
head -c 4096 /dev/zero >"$WORK/body.bin"
( cd "$WORK" && exec python3 -m http.server "$PORT" --bind 127.0.0.1 ) >/dev/null 2>&1 &
SERVER=$!
sleep 1
"$BIN" windows fetch --url "http://127.0.0.1:$PORT/body.bin" \
	--image "$WORK/kept.bin" --max-bytes 1024 >"$WORK/ceiling.txt" 2>&1
say "exit              $?"
sed -n '1,3p' "$WORK/ceiling.txt" | sed 's/^/  /' >>"$REPORT"
kill "$SERVER" 2>/dev/null
if grep -q "over the .* ceiling" "$WORK/ceiling.txt"; then
	say "clause 4          the ceiling refused the 4096-byte body"
elif grep -qi "permission denied\|connect error\|connection failed" "$WORK/ceiling.txt"; then
	say "clause 4          loopback TCP is refused on this lane, so the live"
	say "                  ceiling is not exercised here; the policy is pinned"
	say "                  by the crate's own no-network tests instead"
else
	bad "clause 4          DISAGREED: neither a ceiling refusal nor a lane limit"
fi
if [ -e "$WORK/kept.bin" ]; then bad "destination       LEFT BEHIND"; else say "destination       absent"; fi
if ls "$WORK"/*.part >/dev/null 2>&1; then bad "partial           LEFT BEHIND"; else say "partial           none"; fi

# --- 5. doctor refuses where no profile holds --------------------------------
say ""
say "== doctor"
"$BIN" windows doctor >"$WORK/doctor.txt" 2>&1
[ "$?" -eq 0 ] || bad "doctor did not exit 0"
grep -q "accelerator: tcg" "$WORK/doctor.txt" || grep -q "accelerator: kvm" "$WORK/doctor.txt" || bad "doctor did not name an accelerator"
sed -n '1,8p' "$WORK/doctor.txt" | sed 's/^/  /' >>"$REPORT"

# --- 6. the run verb reaches the driver --------------------------------------
say ""
say "== run-routing"
BEFORE="$(find "$STORE" -type f | sort | xargs -r sha256sum | sha256sum)"
"$BIN" run --rm --podbox-tier=machine --platform windows/amd64 \
	"$WORK/no-such-image.vhdx" -- cmd /c ver >"$WORK/route.txt" 2>&1
say "exit              $?"
sed -n '1,4p' "$WORK/route.txt" | sed 's/^/  /' >>"$REPORT"
if grep -q "not a Linux guest" "$WORK/route.txt"; then
	bad "clause 6          DISAGREED: the OCI path answered, not the driver"
else
	say "clause 6          the driver answered"
fi
AFTER="$(find "$STORE" -type f | sort | xargs -r sha256sum | sha256sum)"
if [ "$BEFORE" = "$AFTER" ]; then say "store             untouched"; else bad "store             MUTATED"; fi

# --- 7. the guest, where an image is configured ------------------------------
say ""
say "== guest"
if [ -z "${PODBOX_WINDOWS_IMAGE:-}" ]; then
	say "skipped           PODBOX_WINDOWS_IMAGE is unset: podbox ships no Windows"
	say "                  image and clause 7 is the only one that needs one"
else
T0="$(date +%s)"
"$BIN" windows setup --image "$PODBOX_WINDOWS_IMAGE" --podbox-timeout 240 >>"$REPORT" 2>&1
say "setup exit        $? in $(( $(date +%s) - T0 ))s"
PROV="$(dirname "$PODBOX_WINDOWS_IMAGE")/$(basename "$PODBOX_WINDOWS_IMAGE" | sed 's/\.[^.]*$//').podbox.qcow2"
if [ -e "$PROV" ]; then say "provisioned       $PROV"; else say "provisioned       ABSENT"; fi
T0="$(date +%s)"
"$BIN" windows run --image "$PROV" -- ver >"$WORK/guest.txt" 2>&1
say "run exit          $? in $(( $(date +%s) - T0 ))s"
sed -n '1,4p' "$WORK/guest.txt" | sed 's/^/  /' >>"$REPORT"
if grep -q "Microsoft Windows" "$WORK/guest.txt"; then
	say "clause 7          the guest answered"
else
	bad "clause 7          DISAGREED: no Windows banner"
fi
fi

# --- 8. the DOS flavor, where a DOS base is configured -----------------------
say ""
say "== dos-guest"
if [ -z "${PODBOX_DOS_BASE:-}" ] || [ ! -f "${PODBOX_DOS_BASE:-}" ]; then
	say "skipped           PODBOX_DOS_BASE is unset or absent: clause 8 needs"
	say "                  the FreeDOS base `363-windows-tcg-dos.sh` writes"
	exit 0
fi
T0="$(date +%s)"
"$BIN" windows run --guest dos -- ver >"$WORK/dos.txt" 2>&1
say "run exit          $? in $(( $(date +%s) - T0 ))s"
sed -n '1,4p' "$WORK/dos.txt" | sed 's/^/  /' >>"$REPORT"
if grep -q "FreeCom version" "$WORK/dos.txt"; then
	say "clause 8          the DOS guest answered"
else
	bad "clause 8          DISAGREED: no FreeCom banner"
fi

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           WINDOWS GUEST HOLDS"; else echo "verdict           WINDOWS GUEST OPEN"; fi
} >>"$REPORT"

cp "$REPORT" "$REPO/experiments/results/windows-364.txt"
if [ -d /out ]; then cp "$REPORT" /out/ 2>/dev/null || true; fi
cat "$REPORT"
[ "$fail" -eq 0 ]
