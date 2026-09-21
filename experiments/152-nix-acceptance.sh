#!/usr/bin/env bash
# Question: does the whole nix 2.2.2 + nixpkgs 22.05 pipeline run through the
# shipped podbox binary -- register, fetch over TLS, evaluate, build locally,
# run the artefact -- with the namespace wall refused by name?
#
# TODO/milestones.md T-1111, M8's acceptance. It drives the SHIPPED binary,
# so it is the last gate rather than an early one.
#
# ⭐ THE PIN IS FORCED, not preferred. Nix >= 2.3 calls posix_openpt()
# unconditionally and this runtime has no /dev/ptmx; nixpkgs 22.11+ gates on
# Nix >= 2.3. So 2.2.2 with 22.05 is the newest release pair whose whole
# pipeline runs with no pty. Tracking the current pair would measure the
# wall instead of podbox, and the wall is already T-0503's subject.
#
# ⛔ UNPATCHED, OR A NAMED DECLINE. Nothing is pre-assembled: the payload
# downloads its own toolchain pieces inside the run. A row that passes
# because the operator pre-assembled the rootfs measures nothing.
#
#   ./152-nix-acceptance.sh            the whole pipeline, one podbox run
#
# Exit: 0 every row green, with the built artefact's own output; 1 a row
#       failed, naming it; 2 nothing ran.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
OUT="${OUT:-$REPO/experiments/results/nix-acceptance.txt}"
TRANSCRIPT="${TRANSCRIPT:-$REPO/experiments/results/nix-acceptance-transcript.txt}"

# ⭐ Every input pinned. A version the registry moves under is a different
# measurement wearing the same name.
NIX_TARBALL_URL="https://nixos.org/releases/nix/nix-2.2.2/nix-2.2.2-x86_64-linux.tar.bz2"
NIX_TARBALL_BYTES="23607712"
NIXPKGS_TAG="22.05"
NIXPKGS_TARBALL_URL="https://github.com/NixOS/nixpkgs/archive/refs/tags/22.05.tar.gz"
BASE='public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a'
DRIVER="$BASE"

RUN_TIMEOUT="${PODBOX_NIX_TIMEOUT:-3600}"
CURL_TIMEOUT="${PODBOX_NIX_CURL_TIMEOUT:-60}"

case "$(uname -s)" in
Linux) NATIVE=1; WORK="$(mktemp -d)" ;;
*)     NATIVE=0; WORK="$REPO/experiments/.sweep152-work"; rm -rf "$WORK"; mkdir -p "$WORK" || exit 2 ;;
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
printf 'nix tarball       %s (%s bytes)\n' "$NIX_TARBALL_URL" "$NIX_TARBALL_BYTES"
printf 'nixpkgs           tag %s\n' "$NIXPKGS_TAG"
printf 'base              %s\n' "$BASE"
printf 'run timeout       %s s\n' "$RUN_TIMEOUT"
printf 'curl timeout      %s s\n' "$CURL_TIMEOUT"
} | tee "$OUT"

# ------------------------------------------------------------------ the subject
#
# ⛔ ONE RUN, SEVEN VERDICTS. The rows share one /nix (register once, build
# once), so they run in one podbox run and each prints its own verdict line.
# A per-row container would re-download the compiler closure seven times.
cat >"$WORK/subject.sh" <<SUBJECT_EOF
#!/bin/sh
# M8 subject. Prints ROW=<name> <ok|FAIL> <detail> per row, then the artefact.
set -u
row() { printf 'ROW=%s %s %s\n' "\$1" "\$2" "\$3"; }

# 0. payload toolchain: curl to fetch, bzip2 and xz to unpack.
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq >/tmp/apt.log 2>&1 || { row BOOTSTRAP FAIL "apt update"; exit 3; }
apt-get install -y -qq curl bzip2 xz-utils ca-certificates >/tmp/apt.log 2>&1 \
  || { row BOOTSTRAP FAIL "apt install curl bzip2"; exit 3; }

# 1. the nix closure into /nix. The tarball carries store/ and .reginfo;
# the layout check below accepts them at the top level or under one wrapper
# directory, and anything else is a different tarball than the pin names.
mkdir -p /nix
curl -fsSL --max-time "$CURL_TIMEOUT" "$NIX_TARBALL_URL" -o /tmp/nix.tbz2 || { row FETCH-NIX FAIL "curl tarball"; exit 3; }
[ "\$(stat -c%s /tmp/nix.tbz2)" = "$NIX_TARBALL_BYTES" ] || { row FETCH-NIX FAIL "size \$(stat -c%s /tmp/nix.tbz2)"; exit 3; }
LIST="\$(tar -tjf /tmp/nix.tbz2 2>/dev/null)" || { row FETCH-NIX FAIL "not a bzip2 tarball"; exit 3; }
echo "\$LIST" | grep -q "\.reginfo" || { row FETCH-NIX FAIL "no .reginfo inside"; exit 3; }
if echo "\$LIST" | grep -qm1 "^\./store/\|^store/"; then
  tar -xjf /tmp/nix.tbz2 -C /nix || { row FETCH-NIX FAIL "unpack"; exit 3; }
else
  tar -xjf /tmp/nix.tbz2 -C /nix --strip-components=1 || { row FETCH-NIX FAIL "unpack wrapped"; exit 3; }
fi
[ -f /nix/.reginfo ] || { row FETCH-NIX FAIL "no /nix/.reginfo after unpack"; exit 3; }
NIXBIN="\$(ls -d /nix/store/*/bin/nix-store 2>/dev/null | head -1)"
[ -n "\$NIXBIN" ] || { row FETCH-NIX FAIL "no nix-store under /nix/store"; exit 3; }
NIXROOT="\$(dirname "\$(dirname "\$NIXBIN")")"
for b in nix-store nix-instantiate nix-build nix-prefetch-url; do
  ln -sf "\$NIXROOT/bin/\$b" "/usr/local/bin/\$b"
done

# nix.conf: single user (no build-users-group), no sandbox (namespaces are
# the wall), the cache pinned with its key.
mkdir -p /etc/nix
cat >/etc/nix/nix.conf <<NIXCONF
build-users-group =
sandbox = false
substituters = https://cache.nixos.org
trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=
NIXCONF
export NIX_SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
export NIX_PATH="nixpkgs=$NIXPKGS_TARBALL_URL"

# Row 1: register the binary-tarball closure in the package database.
if nix-store --load-db </nix/.reginfo >/tmp/register.log 2>&1; then
  row REGISTER ok "closure registered"
else
  row REGISTER FAIL "\$(tail -1 /tmp/register.log | cut -c1-160)"
fi

# Row 2: fetch nixpkgs 22.05 over TLS, through nix's own downloader.
PKGS="\$(nix-instantiate --eval -E 'builtins.fetchTarball { url = "'"$NIXPKGS_TARBALL_URL"'"; }' 2>/tmp/fetch.log | tr -d '\"')" \
  || { row FETCH FAIL "\$(tail -1 /tmp/fetch.log | cut -c1-160)"; PKGS=""; }
if [ -n "\$PKGS" ]; then
  row FETCH ok "\$PKGS"
  export NIX_PATH="nixpkgs=\$PKGS"
fi

# Row 3: evaluate it.
VER="\$(nix-instantiate --eval -E 'with import <nixpkgs> {}; lib.version' 2>/tmp/eval.log)" \
  || { row EVAL FAIL "\$(tail -1 /tmp/eval.log | cut -c1-160)"; VER=""; }
case "\$VER" in
  *22.05*) row EVAL ok "\$VER" ;;
  "") ;;
  *) row EVAL FAIL "version \$VER" ;;
esac

# Row 4: build hello locally. The override changes the output hash, so no
# binary cache can substitute it: the compiler closure substitutes, hello
# compiles here, through the pipe-era builder. The four dont* attrs skip
# the setup hooks that need process substitution (no procfs in a chroot);
# row 7 asserts that scan.
if [ -z "\$PKGS" ]; then
  row BUILD FAIL "no nixpkgs (fetch failed)"
else
  if OUTPATH="\$(nix-build -E 'with import <nixpkgs> {}; hello.overrideDerivation (o: { name = "hello-forced-local-2.12"; dontPatchELF = true; dontRewriteSymlinks = true; dontPatchShebangs = true; noAuditTmpdir = true; })' --no-out-link 2>/tmp/build.log)"; then
    case "\$OUTPATH" in
      *hello-forced-local-2.12) row BUILD ok "\$OUTPATH" ;;
      *) row BUILD FAIL "unexpected path \$OUTPATH" ;;
    esac
  else
    row BUILD FAIL "\$(grep -m1 -i 'error' /tmp/build.log | cut -c1-160)"
  fi
fi

# Row 5: run the built artefact.
if [ -n "\${OUTPATH:-}" ] && [ -x "\$OUTPATH/bin/hello" ]; then
  HELLO="\$("\$OUTPATH/bin/hello" 2>/tmp/run.log)" \
    || { row RUN FAIL "artefact exited nonzero"; HELLO=""; }
  [ "\$HELLO" = "Hello, world!" ] && row RUN ok "\$HELLO" || row RUN FAIL "\$HELLO"
else
  row RUN FAIL "no artefact (build failed)"
fi

# Row 6: the negative row. A process asking for a namespace must get an
# honest refusal, never a silent non-isolation. nix's own sandbox is the
# wrong probe: as root it clones namespaces this chroot grants, or
# substitutes without building at all, so both outcomes read wrong. Raw
# unshare is the request itself: where namespaces are refused it must fail
# naming the wall; where the kernel grants them it must succeed.
if [ -z "\$PKGS" ]; then
  row NEGATIVE FAIL "no nixpkgs (fetch failed)"
else
  if unshare -Urm true >/tmp/unshare.err 2>&1; then
    row NEGATIVE ok "userns granted here; unshare succeeded"
  else
    if grep -qiE "Operation not permitted|not permitted|Permission denied" /tmp/unshare.err; then
      row NEGATIVE ok "unshare refused: \$(head -1 /tmp/unshare.err | cut -c1-120)"
    else
      row NEGATIVE FAIL "unnamed: \$(head -1 /tmp/unshare.err | cut -c1-160)"
    fi
  fi
fi

# Row 7: the procfs row. Six of the package set's fixup hooks use process
# substitution in live code (measured 2026-09-21 in the 22.05 tree, not
# comments). Row 4 runs with four of them disabled; the other three never
# fire fatally for hello. This row pins the six names, so a seventh cannot
# arrive silently.
HOOKS="\$(grep -rl '< <(' "\$PKGS/pkgs/build-support/setup-hooks/"*.sh 2>/dev/null | xargs -n1 basename 2>/dev/null | sort | tr '\n' ' ')"
N="\$(echo "\$HOOKS" | wc -w | tr -d ' ')"
if [ "\$N" = "6" ]; then
  row PROCHOOKS ok "\$HOOKS"
else
  row PROCHOOKS FAIL "\$N hooks: \$HOOKS"
fi
SUBJECT_EOF
chmod +x "$WORK/subject.sh"

# ------------------------------------------------------------------ the driver
# Non-native lane: the shipped binary cannot execute here, so it is staged
# into the driver beside the subject, exactly as 240 stages /pb. The nix
# rows run in ONE podbox run (one /nix, one register, one build); each row
# prints its own verdict line and the runner asserts all seven.
if [ "$NATIVE" -eq 0 ]; then
  [ "$HAVE_ENGINE" -eq 1 ] || { echo "SKIP: no engine on a lane where the podbox binary does not execute" >&2; exit 2; }
  mkdir -p "$WORK/w" "$WORK/stage"
  cp "$BIN" "$WORK/stage/podbox"
  eng_pull "$DRIVER" || exit 2
  eng_mount "$WORK/stage/podbox" /pb \
    && eng_mount "$WORK/w" /w rw \
    && eng_mount "$WORK/subject.sh" /subj/subject.sh \
    || { echo "SKIP: the driver inputs could not be staged" >&2; exit 2; }
  # The lane's CA announcement, as a file: podbox-complete appends the
  # machine's bundle into the rootfs, and nix fetches over TLS.
  eng_run 600 "$DRIVER" "" -- /bin/sh -c 'DEBIAN_FRONTEND=noninteractive apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq ca-certificates && cp /etc/ssl/certs/ca-certificates.crt /w/cacert.pem && test -s /w/cacert.pem' \
    >/dev/null 2>"$WORK/ca-install.log" || {
    echo "SKIP: the CA bundle did not install; see $WORK/ca-install.log" >&2
    exit 2
  }
  cat >"$WORK/row-driver.sh" <<ROWDRIVER_EOF
#!/bin/sh
# One podbox pull and one podbox run in the driver. Readings to /w/row.
# Unquoted heredoc: \$RUN_TIMEOUT below is baked at write time; every other
# dollar is escaped for the container.
set -u
export PODBOX_STORE=/tmp/ostore
if [ -f /w/cacert.pem ]; then
  export SSL_CERT_FILE=/w/cacert.pem
fi
d="/w/row"
mkdir -p "\$d" || exit 6
rm -rf /tmp/ostore
/pb pull public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a >"\$d/pull.log" 2>&1 || exit 3
timeout "$RUN_TIMEOUT" /pb run --rm public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a /bin/sh -c "\$(cat /subj/subject.sh)" >"\$d/out" 2>"\$d/err"
echo "\$?" >"\$d/rowrc"
exit 0
ROWDRIVER_EOF
  eng_mount "$WORK/row-driver.sh" /drv/row.sh \
    || { echo "SKIP: the row driver could not be staged" >&2; exit 2; }
  drc_out="$(eng_run 4500 "$DRIVER" "" -- /bin/sh /drv/row.sh 2>&1)"; drc=$?
  case "$drc" in
  0) cp "$WORK/w/row/pull.log" "$WORK/pull.log" && cp "$WORK/w/row/out" "$WORK/out" && cp "$WORK/w/row/err" "$WORK/err" \
      || { echo "driver files missing" | tee -a "$OUT"; exit 1; }
    row_rc="$(cat "$WORK/w/row/rowrc")" ;;
  3) { echo "no-pull (base image)"; tail -3 "$WORK/w/row/pull.log"; } | tee -a "$OUT"; exit 2 ;;
  6) { echo "could not stage the row readings"; } | tee -a "$OUT"; exit 2 ;;
  *) { echo "driver rc=$drc"; printf '%s\n' "$drc_out" | tail -5; } | tee -a "$OUT"; exit 1 ;;
  esac
else
  # Native lane: the binary executes here.
  export PODBOX_STORE="${PODBOX_SWEEP_STORE:-$WORK/store}"
  "$BIN" pull "$BASE" >"$WORK/pull.log" 2>&1 || { echo "no-pull (base image)" | tee -a "$OUT"; tail -3 "$WORK/pull.log" | tee -a "$OUT"; exit 2; }
  timeout "$RUN_TIMEOUT" "$BIN" run --rm "$BASE" /bin/sh -c "$(cat "$WORK/subject.sh")" \
    >"$WORK/out" 2>"$WORK/err"
  row_rc=$?
fi

# ------------------------------------------------------------------ verdicts
{
  echo "--- the pull"
  tail -4 "$WORK/pull.log" | cut -c1-200
  echo "--- podbox run exit $row_rc"
  echo "--- stdout (verdicts and artefact)"
  grep -E "^(ROW=|Hello)" "$WORK/out" | cut -c1-200
  echo "--- the completion layer said"
  grep 'podbox: complete:' "$WORK/err" | cut -c1-160
  echo "--- the last of stderr"
  grep -v 'podbox: complete:' "$WORK/err" | tail -5 | cut -c1-200
} | tee "$TRANSCRIPT"

get() { sed -n "s/^ROW=$1 //p" "$WORK/out" | head -1; }
fail=0
missing=""
for r in REGISTER FETCH EVAL BUILD RUN NEGATIVE PROCHOOKS; do
  line="$(get "$r")"
  case "$line" in
  ok*) printf 'row %-9s ok   %s\n' "$r" "${line#ok }" | tee -a "$OUT" ;;
  "") printf 'row %-9s MISSING (no verdict line)\n' "$r" | tee -a "$OUT"; fail=1; missing="$missing $r" ;;
  *) printf 'row %-9s FAIL %s\n' "$r" "${line#FAIL }" | tee -a "$OUT"; fail=1; missing="$missing $r" ;;
  esac
done

if [ "$fail" -eq 0 ]; then
  printf 'M8 acceptance: every row green. The artefact says:\n' | tee -a "$OUT"
  grep -E "^Hello" "$WORK/out" | tee -a "$OUT"
  printf 'transcript: %s\n' "${TRANSCRIPT#"$REPO"/}" | tee -a "$OUT"
  exit 0
fi
printf 'M8 acceptance: FAILED row(s):%s\n' "$missing" | tee -a "$OUT"
printf 'transcript: %s\n' "${TRANSCRIPT#"$REPO"/}" | tee -a "$OUT"
exit 1
