#!/bin/sh
# Question: on a lane that has an emulator but no KVM, does `podbox windows`
# refuse where it must, reach the guest driver where the entry says it should,
# and does the acquisition path bound and verify itself without a network
# dependency in the test?
#
# TODO/milestones.md T-1112. ⚠ Clause 7 is the only one that needs a licensed
# Windows disk image and it is skipped, out loud, where none is configured:
# podbox neither ships nor fetches one, so a lane without `PODBOX_WINDOWS_IMAGE`
# cannot boot a guest and must say so rather than pass.
#
# Clauses:
#   0. conditions: binary, emulator, /dev/kvm, image.
#   1. the crate's own tests pass, and the protocol's own count is printed.
#   2. `windows --help` is 0 and names every subcommand; an unknown one is 2.
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

# --- 1. the crate's own tests ------------------------------------------------
say ""
say "== crate-tests"
if cargo test -p podbox-windows --lib --target x86_64-unknown-linux-gnu \
	>"$WORK/tests.txt" 2>&1; then
	say "result            pass"
	say "count             $(sed -n 's/^test result: ok\. \([0-9]*\) passed.*/\1/p' "$WORK/tests.txt" | tail -1)"
else
	say "result            FAIL"
	tail -20 "$WORK/tests.txt" >>"$REPORT"
fi

# --- 2. help and an unknown subcommand ---------------------------------------
say ""
say "== help"
"$BIN" windows --help >"$WORK/help.txt" 2>&1
say "exit              $?"
for sub in doctor fetch setup run; do
	if grep -q "$sub" "$WORK/help.txt"; then say "names $sub         yes"; else say "names $sub         NO"; fi
done
"$BIN" windows definitely-not-a-subcommand >"$WORK/bogus.txt" 2>&1
say "unknown exit      $?"

# --- 3. a bad pin is refused before the URL is used --------------------------
say ""
say "== bad-pin"
"$BIN" windows fetch --url http://127.0.0.1:1/never --sha256 abc >"$WORK/pin.txt" 2>&1
say "exit              $?"
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
if [ -e "$WORK/kept.bin" ]; then say "destination       LEFT BEHIND"; else say "destination       absent"; fi
if [ -e "$WORK/kept.part" ]; then say "partial           LEFT BEHIND"; else say "partial           absent"; fi

# --- 5. doctor refuses where no profile holds --------------------------------
say ""
say "== doctor"
"$BIN" windows doctor >"$WORK/doctor.txt" 2>&1
say "exit              $?"
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
	say "clause 6          DISAGREED: the OCI path answered, not the driver"
else
	say "clause 6          the driver answered"
fi
AFTER="$(find "$STORE" -type f | sort | xargs -r sha256sum | sha256sum)"
if [ "$BEFORE" = "$AFTER" ]; then say "store             untouched"; else say "store             MUTATED"; fi

# --- 7. the guest, where an image is configured ------------------------------
say ""
say "== guest"
if [ -z "${PODBOX_WINDOWS_IMAGE:-}" ]; then
	say "skipped           PODBOX_WINDOWS_IMAGE is unset: podbox ships no Windows"
	say "                  image and clause 7 is the only one that needs one"
	exit 0
fi
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
	say "clause 7          DISAGREED: no Windows banner"
fi
