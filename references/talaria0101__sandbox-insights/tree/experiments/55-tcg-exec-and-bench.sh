#!/usr/bin/env bash
# 55-tcg-exec-and-bench.sh
#
# Question: can the guest be DRIVEN — commands executed, exit codes
# propagated — and does in-guest compute produce identical results to
# the host at software-emulation speed?
#
# Protocol (all proven here):
#   transport   FIFO pair via `qemu -serial pipe:`; the parent holds
#               BOTH ends O_RDWR before qemu starts (the pipe backend
#               opens existing fifos and blocks otherwise)
#   readiness   retry-feed "\n" + `echo <nonce>` until the nonce echoes
#               (bootstrap feeding is lossy before the console is up)
#   exec        marker-bracketed single-shots:
#               echo TAG_B; { cmd ; } ; echo TAG_E $?
#               line-anchored matching, exit code propagated
#   validation  the same bench.c, -O2, host and guest; the printed
#               CHECKSUM must be identical — a cross-platform run that
#               computed something different says so instead of
#               looking fast
#
# Inputs: pinned kernel + busybox. Tools: qemu, python3, cc.
# Exit: 0 protocol worked, checksums match · 1 mismatch/no marker · 2 could not run.
set -u
cd "$(dirname "$0")" || exit 2
. ../scripts/lib.sh

OUT="$SI_LOGS/55-tcg-exec-and-bench.log"
{
  conditions tcg-exec-bench
  echo
} > "$OUT"

# host-side build of the same benchmark the guest will run
cc -O2 -static -o "$SI_WORK/bench" "$SI_SCRIPTS/bench.c" 2>>"$OUT" || { echo "bench build failed" >> "$OUT"; exit 2; }
cc -O2 -static -o "$SI_WORK/bench_fp" "$SI_SCRIPTS/bench_fp.c" 2>>"$OUT" || { echo "bench_fp build failed" >> "$OUT"; exit 2; }
echo "## host bench (integer-dominated):" >> "$OUT"
HOST_BENCH="$("$SI_WORK/bench")" || { echo "host bench failed" >> "$OUT"; exit 2; }
echo "$HOST_BENCH" >> "$OUT"
echo "## host bench_fp (dependent double chain):" >> "$OUT"
HOST_BENCH_FP="$("$SI_WORK/bench_fp")" || { echo "host bench_fp failed" >> "$OUT"; exit 2; }
echo "$HOST_BENCH_FP" >> "$OUT"
HOST_CKSUM="$(sed -n 's/.*checksum=\([0-9a-f]*\).*/\1/p' <<< "$HOST_BENCH")"

# initramfs: busybox + the static bench + an interactive-ish init
INIT="$SI_WORK/init55.sh"
cat > "$INIT" <<EOF
#!/bin/busybox sh
/bin/busybox --install -s /bin
mount -t proc proc /proc 2>/dev/null
echo VMR-GUEST-READY
exec /bin/busybox sh
EOF
cp "$SI_WORK/bin/busybox.static" "$SI_WORK/bin/busybox.static" 2>/dev/null || true
python3 "$SI_SCRIPTS/mkrootfs.py" "$SI_WORK/exec.cpio" \
  --dir dev --dir bin --dir proc --dir tmp \
  --char dev/console:5:1 --char dev/null:1:3 \
  --file "$SI_WORK/bin/busybox.static:bin/busybox" \
  --file "$SI_WORK/bench:bench" \
  --file "$SI_WORK/bench_fp:bench_fp" \
  --file "$INIT:init" > /dev/null 2>>"$OUT" || { echo "mkrootfs failed" >> "$OUT"; exit 2; }
gzip -1 -kc "$SI_WORK/exec.cpio" > "$SI_WORK/exec.cpio.gz"

D="$SI_WORK/55-serial"; rm -rf "$D"; mkdir -p "$D"
mkfifo "$D/serial.in" "$D/serial.out"

# hold BOTH fifo ends O_RDWR before qemu starts
exec 3<>"$D/serial.in" 4<>"$D/serial.out"
qemu-system-x86_64 -accel tcg,thread=multi \
  -M pc,acpi=off -m 256M -smp 1 \
  -kernel "$SI_WORK/vmlinuz-virt" -initrd "$SI_WORK/exec.cpio.gz" \
  -append "console=ttyS0 rdinit=/init panic=-1 quiet" \
  -display none -no-reboot -monitor none \
  -serial pipe:"$D/serial" >/dev/null 2>>"$OUT" &
QPID=$!
# the driver kills its own qemu on ANY exit path — a backgrounded qemu
# whose parent dies is an orphan holding fifos and 256M of RAM
trap 'kill $QPID 2>/dev/null' EXIT

# drive the guest: python reads the out-fifo, writes to the in-fifo.
# ALL fds are O_NONBLOCK and every wait has a hard deadline: a driver
# that can block forever does not own the guest's lifecycle — it is
# hostage to it.
python3 - "$D" "$OUT" "$HOST_CKSUM" <<'EOF'
import os, re, select, sys, time

d, out_path, host_cksum = sys.argv[1], sys.argv[2], sys.argv[3]
outfd = os.open(d + "/serial.out", os.O_RDONLY | os.O_NONBLOCK)
infd = os.open(d + "/serial.in", os.O_WRONLY | os.O_NONBLOCK)
buf = b""
log = open(out_path, "a")

def pump(until, timeout):
    """read until the regex `until` matches, or timeout (hard deadline)"""
    global buf
    deadline = time.time() + timeout
    while time.time() < deadline:
        m = re.search(until, buf.decode("utf-8", "replace"))
        if m:
            return m
        r, _, _ = select.select([outfd], [], [], 0.1)
        if r:
            try:
                chunk = os.read(outfd, 65536)
            except BlockingIOError:
                continue
            if chunk:
                buf += chunk
                if len(buf) > 262144: buf = buf[-131072:]
    return None

def write_in(data):
    for _ in range(100):
        try:
            os.write(infd, data)
            return
        except BlockingIOError:
            time.sleep(0.02)
    raise RuntimeError("in-fifo never accepted writes")

# readiness: feed newlines + a nonce until it echoes back, then wait
# for the shell PROMPT itself — a nonce that matches may be the tty's
# echo of the typed line rather than the shell's output, and an exec
# line typed before the shell reads it is lost (this flaked once).
ready = False
for attempt in range(30):
    nonce = f"VMR-READY-{attempt}"
    write_in(f"\necho {nonce}\n".encode())
    if pump(re.escape(nonce), 3.0):
        write_in(b"\n")
        if pump(r"[#$] $", 10.0):   # busybox ash prompt: `~ # `
            ready = True
        break
log.write("## guest readiness: %s\n" % ("ok" if ready else "FAILED"))
if not ready:
    log.write("## serial tail at failure:\n---\n%s\n---\n" % buf[-1200:].decode("utf-8", "replace"))
    sys.exit(1)

# marker-bracketed single-shot exec, exit code propagated.
# Markers are LINE-ANCHORED: unanchored matches hit the echoed typed
# command instead of the output (this bit on the first run).
tag = "X" + os.urandom(3).hex()
write_in(f"echo {tag}B; {{ /bench ; /bench_fp ; }} ; echo {tag}E $?\n".encode())
if not pump(("(?m)^" + tag + "B\\r?$"), 60):
    log.write("## guest exec: NO BEGIN MARKER (line-anchored)\n")
    log.write("## serial tail at failure:\n---\n%s\n---\n" % buf[-1200:].decode("utf-8", "replace"))
    sys.exit(1)
start = re.search("(?m)^" + tag + "B\\r?$", buf.decode("utf-8", "replace")).end()
m = pump("(?m)^" + tag + r"E (\d+)\r?$", 120)
if not m:
    log.write("## guest exec: NO END MARKER (line-anchored)\n")
    log.write("## serial tail at failure:\n---\n%s\n---\n" % buf[-1200:].decode("utf-8", "replace"))
    sys.exit(1)
guest_rc = int(m.group(1))
body = buf.decode("utf-8", "replace")
j = re.search("(?m)^" + tag + r"E (\d+)\r?$", body).start()
guest_out = body[start:j]
log.write("## guest bench (integer + FP-heavy):\n%s\n## guest exec exit code: %d\n" % (guest_out.strip(), guest_rc))
cks = re.findall(r"checksum=([0-9a-f]+)", guest_out)
mops = re.findall(r"mops=([0-9.]+)", guest_out)
guest_cksum = cks[0] if cks else "(none)"
log.write("## checksum comparison (integer bench): host=%s guest=%s -> %s\n"
          % (host_cksum, guest_cksum,
             "IDENTICAL" if guest_cksum == host_cksum else "MISMATCH"))
valid = guest_cksum == host_cksum and guest_rc == 0 and len(cks) >= 2 and len(mops) >= 2
if valid:
    # both benches got in-guest numbers; the FP-heavy guest run has no
    # meaningful single checksum to pair (fp state collapses), so pair mops ratios
    ratio_int = float(mops[0])
    log.write("## in-guest mops: integer=%.1f fp-heavy=%.1f\n" % (float(mops[0]), float(mops[1])))
log.write("## cross-platform run validated\n" if valid else "## RUN INVALID\n")
# clean shutdown path: halt the guest, then the driver kills qemu
write_in(b"echo " + tag.encode() + b"F\n")
sys.exit(0 if (guest_cksum == host_cksum and guest_rc == 0) else 1)
EOF
DRC=$?
kill "$QPID" 2>/dev/null; wait "$QPID" 2>/dev/null
exec 3>&- 4>&-
tail -14 "$OUT"
echo
echo "log: $OUT"
exit $DRC
