# lib.sh — shared pins + helpers for every experiment. Sourced, never executed.
# Every artefact is pinned: URL + sha256. Nothing unpinned is trusted.
set -u
SI_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SI_WORK="$SI_ROOT/work"
SI_LOGS="$SI_ROOT/experiments/logs"
SI_SCRIPTS="$SI_ROOT/scripts"
mkdir -p "$SI_WORK" "$SI_LOGS"

# --- pinned artefacts --------------------------------------------------------
ALPINE_MIRROR="https://dl-cdn.alpinelinux.org/alpine/v3.22"
PIN_VMLINUZ_VIRT_URL="$ALPINE_MIRROR/releases/x86_64/netboot/vmlinuz-virt"
PIN_VMLINUZ_VIRT_SHA="12eb24189f3eb30bd0dcd919248caaa054ed4e87b799a53fdcc3999f157933e4"
PIN_BUSYBOX_URL="$ALPINE_MIRROR/main/x86_64/busybox-static-1.37.0-r20.apk"
PIN_BUSYBOX_SHA="488ad6efd04b5a722719e79f8e0dcc2c24afd6758867af3ce41b04839e60c74b"
PIN_MINIROOTFS_URL="$ALPINE_MIRROR/releases/x86_64/alpine-minirootfs-3.22.0-x86_64.tar.gz"
PIN_MINIROOTFS_SHA="18879884e35b0718f017a50ff85b5e6568279e97233fc42822229585feb2fa4d"

conditions() { # conditions <subject> — print the conditions block
  echo "## conditions"
  echo "date: $(date -u +%FT%TZ)"
  echo "host: $(uname -srmo)"
  echo "cpu: $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | xargs) / $(nproc) threads"
  echo "mem: $(free -h | awk '/^Mem/{print $2}')"
  echo "uid: $(id -u)  caps: $(grep CapEff /proc/self/status | awk '{print $2}')  seccomp: $(grep Seccomp /proc/self/status | awk '{print $2}')"
  echo "uid_map: $(tr '\n' ' ' < /proc/self/uid_map)"
  echo "subject: $1"
}

vfetch() { # vfetch <name> <url> <sha256> — pinned download into work/
  local name="$1" url="$2" sha="$3"
  local dst="$SI_WORK/$name"
  if [ -f "$dst" ] && echo "$sha  $dst" | sha256sum -c --quiet 2>/dev/null; then
    echo "[vfetch] $name cached, hash ok"
    return 0
  fi
  echo "[vfetch] $name <- $url"
  curl -sL --max-time 300 -o "$dst" "$url" || { echo "[vfetch] DOWNLOAD FAILED $name"; return 1; }
  echo "$sha  $dst" | sha256sum -c --quiet || { echo "[vfetch] HASH MISMATCH $name"; return 1; }
}

run_logged() { # run_logged <label> <cmd...> — tee to logs/<label>.log, keep rc
  local label="$1"; shift
  local log="$SI_LOGS/$label.log"
  {
    echo "### cmd: $*"
    conditions "${label%%-*}"
  } > "$log"
  "$@" 2>&1 | tee -a "$log"
  local rc="${PIPESTATUS[0]}"
  echo "### exit: $rc" >> "$log"
  echo "[run_logged] $label rc=$rc log=$log"
  return "$rc"
}
