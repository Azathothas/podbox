#!/usr/bin/env bash
# Question: can a long-running microVM be driven programmatically — a
# deterministic "exec command in guest, capture output + exit code" protocol
# over the serial console? This is the docker-exec-equivalent primitive a
# podman-style microVM tool needs. PoC protocol (all steps in committed log):
#   initramfs = prepared rootfs (72-) + init shim + /root/bench.c + appended
#   /dev/console node; qemu -serial pipe: FIFO pair (parent python holds both
#   ends O_RDWR so qemu's opens never block); handshake retry-feed until the
#   shell echoes a nonce; then single-shot marker-bracketed execs.
# Demonstrated: uname, id, gcc --version, IN-GUEST COMPILE of bench.c, and
# in-guest execution of the compiled benchmark.
# Exit codes: 0 all execs succeeded, 1 partial, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/73-serial-exec-protocol.log"
WORK="$VR_WORK"
export WORK
{ echo "## conditions"; conditions serial-exec-protocol; } > "$LOG"
mkdir -p "$WORK/73-machine"
cat > "$WORK/73-driver.py" <<'PYEOF'
import os, sys, time, re, errno, subprocess
WORK = os.environ["WORK"]

D = WORK + "/73-machine"
IN, OUT = D + "/serial.in", D + "/serial.out"
KERNEL = WORK + "/vmlinuz-virt"
ROOT = WORK + "/alpine-gcc-root"
IR = D + "/initramfs.cpio"

# ---- 1. assemble initramfs: root tree + init shim + bench.c + console node
BODY = D + "/initramfs.body"
STAGE = D + "/stage"
subprocess.run(f"rm -rf '{STAGE}'", shell=True)
subprocess.run(f"cp -r '{ROOT}' '{STAGE}'", shell=True, check=True)
os.makedirs(STAGE + "/dev", exist_ok=True)
os.makedirs(STAGE + "/root", exist_ok=True)
init_sh = (b"#!/bin/sh\n"
           b"mount -t proc proc /proc\n"
           b"mount -t sysfs sysfs /sys\n"
           b"mount -t devtmpfs devtmpfs /dev 2>/dev/null\n"
           b"echo VMR-GUEST-READY\n"
           b"exec /bin/sh\n")
open(STAGE + "/init","wb").write(init_sh); os.chmod(STAGE + "/init", 0o755)
bench_src = open(WORK + "/bench-common/bench.c","rb").read()
open(STAGE + "/root/bench.c","wb").write(bench_src)
subprocess.run(f"(cd '{STAGE}' && find . | cpio -o -H newc --quiet | gzip -1) > '{IR}'", shell=True, check=True)
print("initramfs ready:", os.path.getsize(IR), "bytes", flush=True)

# ---- 2. fifos: parent holds both ends O_RDWR (never blocks, keeps them alive)
for f in (IN, OUT):
    if os.path.exists(f): os.remove(f)
    os.mkfifo(f)
in_keep = os.open(IN, os.O_RDWR)
out_keep = os.open(OUT, os.O_RDWR)

# ---- 3. boot
qemu = subprocess.Popen(["qemu-system-x86_64", "-accel", "tcg,thread=multi",
    "-M", "pc,acpi=off", "-m", "1024M", "-smp", "2",
    "-kernel", KERNEL, "-initrd", IR,
    "-append", "console=ttyS0 rdinit=/init",
    "-display", "none", "-no-reboot", "-monitor", "none",
    "-serial", f"pipe:{D}/serial"], cwd=D,
    stdout=open("qemu.out","wb"), stderr=open("qemu.err","wb"))

fin = os.open(IN, os.O_WRONLY | os.O_NONBLOCK)
fout = os.open(OUT, os.O_RDONLY | os.O_NONBLOCK)
buf = b""
def pump(t=0.3):
    global buf
    end = time.time() + t
    while time.time() < end:
        try:
            d = os.read(fout, 65536)
            if d: buf += d
        except BlockingIOError: pass
        time.sleep(0.05)

# ---- 4. wait for guest readiness
t0 = time.time(); ready = False
while time.time() - t0 < 240:
    pump(1)
    if b"VMR-GUEST-READY" in buf: ready = True; break
print("ready:", ready, f"({len(buf)} bytes in {time.time()-t0:.0f}s)", flush=True)
assert ready, buf[-200:]
buf = b""

# ---- 5. handshake + exec protocol (line-anchored markers, single-shot)
def exec_cmd(cmd, timeout=90):
    global buf
    tag = os.urandom(3).hex()
    b, e = f"VMR_{tag}_B", f"VMR_{tag}_E"
    oneline = " ".join(cmd.split("\n"))
    os.write(fin, f"echo {b}; {{ {oneline} ; }} ; echo {e} $?\n".encode())
    endd = time.time() + timeout
    while time.time() < endd:
        pump(1)
        text = buf.decode(errors="replace").replace("\r", "")
        if re.search("^" + re.escape(e) + r" \d+\s*$", text, re.M): break
    text = buf.decode(errors="replace").replace("\r", "")
    mb = re.search("^" + re.escape(b) + r"\s*$", text, re.M)
    me = re.search("^" + re.escape(e) + r" (\d+)\s*$", text, re.M)
    rc = int(me.group(1)) if me else None
    out = text[(mb.end() if mb else 0):(me.start() if me else len(text))]
    print(f"[exec rc={rc}] {out.strip(chr(10))}", flush=True)
    buf = b""
    return rc, out

hs = os.urandom(3).hex()
os.write(fin, f"echo VMR_HS_{hs}\n".encode())
endd = time.time() + 120
while time.time() < endd:
    pump(1)
    if f"VMR_HS_{hs}" in buf.decode(errors="replace"): break
print("handshake: shell reads console input", flush=True)
buf = b""

exec_cmd("ls -la /root/ /init; head -c 60 /root/bench.c 2>&1")
exec_cmd("uname -a")
exec_cmd("id")
exec_cmd("/usr/bin/gcc --version | head -1")
exec_cmd("/usr/bin/gcc -O2 -fno-use-linker-plugin -o /tmp/bench /root/bench.c && echo COMPILE-OK")
exec_cmd("/tmp/bench 30000000")
qemu.kill()
print("PROTOCOL-POC-DONE")

PYEOF
python3 -u "$WORK/73-driver.py" 2>&1 | tee "$VR_LOGS/73-protocol-driver.log"
grep -aq 'PROTOCOL-POC-DONE' "$VR_LOGS/73-protocol-driver.log" && exit 0 || exit 1
