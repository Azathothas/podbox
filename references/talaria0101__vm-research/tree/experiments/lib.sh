# Shared pins + fetch-verify for every experiment. Sourced, never executed.
# Every artefact is pinned: URL + sha256. Nothing unpinned is trusted.
set -u
VR_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VR_WORK="$VR_ROOT/experiments/work"
VR_LOGS="$VR_ROOT/experiments/logs"
mkdir -p "$VR_WORK" "$VR_LOGS"

# --- pinned artefacts -------------------------------------------------------
ALPINE_MIRROR="https://dl-cdn.alpinelinux.org/alpine/v3.22"
PIN_VMLINUZ_VIRT_URL="$ALPINE_MIRROR/releases/x86_64/netboot/vmlinuz-virt"
PIN_VMLINUZ_VIRT_SHA="12eb24189f3eb30bd0dcd919248caaa054ed4e87b799a53fdcc3999f157933e4"
PIN_INITRAMFS_VIRT_URL="$ALPINE_MIRROR/releases/x86_64/netboot/initramfs-virt"
PIN_INITRAMFS_VIRT_SHA="41724bde62ccec81ab2b46745690928b0a80c7a353008db6364c868368944a39"
PIN_BUSYBOX_URL="$ALPINE_MIRROR/main/x86_64/busybox-static-1.37.0-r20.apk"
PIN_BUSYBOX_SHA="488ad6efd04b5a722719e79f8e0dcc2c24afd6758867af3ce41b04839e60c74b"

conditions() { # call with a subject name; prints the conditions block
  echo "## conditions"
  echo "date: $(date -u +%FT%TZ)"
  echo "host: $(uname -srmo) / $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | xargs)"
  echo "mem: $(free -h | awk '/^Mem/{print $2}')  cpus: $(nproc)"
  echo "qemu: $(qemu-system-x86_64 --version | head -1)"
  echo "accel: tcg (kvm unavailable, see experiments/10-probe-environment.log)"
  echo "subject: $1"
}

vfetch() { # vfetch <name> <url> <sha256> — pinned download into work/
  local name="$1" url="$2" sha="$3"
  local dst="$VR_WORK/$name"
  if [ -f "$dst" ] && echo "$sha  $dst" | sha256sum -c --quiet 2>/dev/null; then
    echo "[vfetch] $name cached, hash ok"
    return 0
  fi
  echo "[vfetch] $name <- $url"
  curl -sL --max-time 300 -o "$dst" "$url"
  echo "$sha  $dst" | sha256sum -c --quiet || { echo "[vfetch] HASH MISMATCH $name"; return 1; }
}

vr_run() { # vr_run <label> <timeout_s> <cmd...> — runs, tees to logs/, keeps rc
  local label="$1" to="$2"; shift 2
  local log="$VR_LOGS/$label.log"
  echo "### cmd: $*" > "$log"
  { echo "## conditions"; conditions "${label%%-*}"; } >> "$log"
  timeout "$to" "$@" >> "$log" 2>&1
  local rc=$?
  echo "### exit: $rc (124 = timeout hit)" >> "$log"
  echo "[vr_run] $label rc=$rc log=$log"
  return $rc
}
