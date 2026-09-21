#!/usr/bin/env bash
# Question: does a tarball carrying symlinks unpack under podbox with the
# same result as the engine control?
#
# TODO/interpose.md T-1311. The interposed `fchmodat` used to declare and
# forward three arguments where libc takes four, so the real call read a
# fourth register the wrapper never set and tar's symlink mode-set failed.
# This drives the minimal repro through the shipped binary with the plain
# driver beside it: same image, same commands, back to back.
#
#   PODBOX_BIN=/path/to/podbox ./162-tar-symlink-modes.sh
#
# Exit: 0 the unpack ran and matched on both sides; 1 it ran and a side
#       failed, naming it; 2 nothing ran.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="${OUT:-$REPO/experiments/results/tar-symlink-modes.txt}"

# ⭐ Every input pinned. The digest is the row 240 and 152 drive.
BASE='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
DRIVER="$BASE"

case "$(uname -s)" in
Linux) NATIVE=1; WORK="$(mktemp -d)" ;;
*)     NATIVE=0; WORK="$REPO/experiments/.sweep162-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2 ;;
esac
trap 'rm -rf "$WORK"' EXIT INT TERM

# shellcheck source=lib/engine.sh
. "$HERE/lib/engine.sh"
export ENGINE_REPO ENGINE_WORK
ENGINE_REPO="$REPO"
ENGINE_WORK="$WORK"
if engine_pick; then HAVE_ENGINE=1; else HAVE_ENGINE=0; fi

[ -r "$BIN" ] || { echo "SKIP: $BIN is not readable (set PODBOX_BIN to a guest build artifact)" >&2; exit 2; }

{
echo "== conditions"
printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf 'host kernel       %s\n' "$(uname -r)"
printf 'engine            %s\n' "$(engine_describe)"
if [ "$NATIVE" -eq 1 ]; then
  printf 'podbox            %s\n' "$("$BIN" version 2>&1)"
else
  printf 'podbox            %s (staged, not executed on this lane)\n' "$BIN"
fi
printf 'podbox sha256     %s\n' "$(sha256sum "$BIN" | cut -d' ' -f1)"
printf 'driver            %s\n' "$DRIVER"
} | tee "$OUT"

# ------------------------------------------------------------------ the subject
# One file, one link at top level, one link in a subdirectory: the shape the
# nix binary tarball failed on. Prints its own verdict lines.
cat >"$WORK/subject.sh" <<'SUBJECT_EOF'
#!/bin/sh
set -u
mkdir -p /lx/t /lx/t2 /lx/t/src
cd /lx/t || exit 3
echo data >f
chmod 777 f
ln -s f l
echo data >src/g
ln -s g src/h
tar -cf /tmp/t.tar -C /lx/t . || exit 3
tar -xf /tmp/t.tar -C /lx/t2
echo "TAR_RC:$?"
cd /lx/t2 || exit 3
[ -L ./l ] && [ "$(readlink ./l)" = "f" ] && echo "LINK_l ok" || echo "LINK_l FAIL"
[ -L ./src/h ] && [ "$(readlink ./src/h)" = "g" ] && echo "LINK_h ok" || echo "LINK_h FAIL"
test "$(stat -c%a ./f)" = "777" && echo "MODE_f ok" || echo "MODE_f FAIL $(stat -c%a ./f)"
SUBJECT_EOF
chmod +x "$WORK/subject.sh"

# ------------------------------------------------------------------ the driver
if [ "$NATIVE" -eq 0 ]; then
  [ "$HAVE_ENGINE" -eq 1 ] || { echo "SKIP: no engine on a lane where the podbox binary does not execute" >&2; exit 2; }
  mkdir -p "$WORK/w" "$WORK/stage"
  cp "$BIN" "$WORK/stage/podbox"
  eng_pull "$DRIVER" || exit 2
  eng_mount "$WORK/stage/podbox" /pb \
    && eng_mount "$WORK/w" /w rw \
    && eng_mount "$WORK/subject.sh" /subj/subject.sh \
    || { echo "SKIP: the driver inputs could not be staged" >&2; exit 2; }
  cat >"$WORK/side-driver.sh" <<SIDEDRIVER_EOF
#!/bin/sh
# One side per call: "treat" runs the subject under podbox, "ctl" runs it
# straight in the driver. Readings to /w/\$1/.
set -u
side="\$1"
d="/w/\$side"
mkdir -p "\$d" || exit 6
if [ "\$side" = "treat" ]; then
  export PODBOX_STORE=/tmp/ostore
  rm -rf /tmp/ostore
  /pb pull "$DRIVER" >"\$d/pull.log" 2>&1 || exit 3
  /pb run --rm "$DRIVER" /bin/sh -c "\$(cat /subj/subject.sh)" >"\$d/out" 2>"\$d/err"
  echo "\$?" >"\$d/rc"
else
  /bin/sh /subj/subject.sh >"\$d/out" 2>"\$d/err"
  echo "\$?" >"\$d/rc"
fi
exit 0
SIDEDRIVER_EOF
  eng_mount "$WORK/side-driver.sh" /drv/side.sh \
    || { echo "SKIP: the side driver could not be staged" >&2; exit 2; }
  for side in treat ctl; do
    if ! eng_run 1200 "$DRIVER" "" -- /bin/sh /drv/side.sh "$side" >/dev/null 2>"$WORK/$side-eng.log"; then
      { echo "driver rc=$? on side $side"; tail -5 "$WORK/$side-eng.log"; } | tee -a "$OUT"; exit 1
    fi
  done
  eng_clear
  for side in treat ctl; do
    cp "$WORK/w/$side/out" "$WORK/$side-out" && cp "$WORK/w/$side/err" "$WORK/$side-err" \
      && cp "$WORK/w/$side/rc" "$WORK/$side-rc" \
      || { echo "driver files missing for side $side" | tee -a "$OUT"; exit 1; }
  done
else
  export PODBOX_STORE="${PODBOX_SWEEP_STORE:-$WORK/store}"
  "$BIN" pull "$BASE" >"$WORK/pull.log" 2>&1 || { echo "no-pull (base image)" | tee -a "$OUT"; tail -3 "$WORK/pull.log" | tee -a "$OUT"; exit 2; }
  "$BIN" run --rm "$BASE" /bin/sh -c "$(cat "$WORK/subject.sh")" >"$WORK/treat-out" 2>"$WORK/treat-err"
  echo "$?" >"$WORK/treat-rc"
  /bin/sh "$WORK/subject.sh" >"$WORK/ctl-out" 2>"$WORK/ctl-err"
  echo "$?" >"$WORK/ctl-rc"
fi

# ------------------------------------------------------------------ verdicts
{
  echo "--- treatment stdout"
  cat "$WORK/treat-out" | cut -c1-160
  echo "--- treatment stderr (tar errors, if any)"
  grep -v 'podbox: complete:' "$WORK/treat-err" | tail -6 | cut -c1-200
  echo "--- control stdout"
  cat "$WORK/ctl-out" | cut -c1-160
  echo "--- control stderr"
  tail -4 "$WORK/ctl-err" | cut -c1-200
} | tee -a "$OUT"

get() { grep -E "^$2[: ]" "$WORK/$1-out" | head -1; }
fail=0
for side in treat ctl; do
  line="$(get "$side" "TAR_RC")"
  if [ "$line" = "TAR_RC:0" ]; then
    printf 'row %-6s unpack rc 0\n' "$side" | tee -a "$OUT"
  else
    printf 'row %-6s FAIL unpack %s (side rc=%s)\n' "$side" "${line:-no TAR_RC line}" "$(cat "$WORK/$side-rc" 2>/dev/null || echo ?)" | tee -a "$OUT"; fail=1
  fi
  for v in LINK_l LINK_h MODE_f; do
    line="$(get "$side" "$v")"
    case "$line" in
    "$v ok") printf 'row %-6s %-7s ok\n' "$side" "$v" | tee -a "$OUT" ;;
    *) printf 'row %-6s %-7s FAIL %s\n' "$side" "$v" "${line:-missing}" | tee -a "$OUT"; fail=1 ;;
    esac
  done
done

if [ "$fail" -eq 0 ]; then
  printf 'tar-symlink-modes: every row green under podbox and in the control.\n' | tee -a "$OUT"
  exit 0
fi
printf 'tar-symlink-modes: FAILED (see rows above).\n' | tee -a "$OUT"
exit 1
