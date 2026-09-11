# Recipes: what runs, and how

Every recipe here is exercised by a committed experiment in this
repository — the experiment numbers are cited per section. Exact
commands, pinned inputs, and the quirks that cost real time are all
inline. Nothing on this page is aspirational.

## TCG microVMs (KVM is unreachable; software emulation is not)

**Boot a real kernel to userspace in 2–6 s** (experiment 50-):

```sh
qemu-system-x86_64 -accel tcg,thread=multi \
  -M pc,acpi=off -m 256M -smp 1 \
  -kernel vmlinuz-virt -initrd root.cpio.gz \
  -append "console=ttyS0 rdinit=/init panic=-1 quiet" \
  -display none -no-reboot -monitor none \
  -serial file:serial.log
```

Quirks that will bite, each one measured:

- **`-M pc,acpi=off` cannot power off** — the guest halts and qemu
  keeps running. The *driver* owns the lifecycle: poll the serial log
  for a completion marker, then kill qemu. Waiting for qemu to exit
  means waiting for a timeout.
- **`-M microvm` needs `acpi=off`** (hangs parsing its generated ACPI
  tables otherwise), and it delivers kernel `printk` to the UART while
  *userspace* ttyS0 writes never surface — use `pc` for anything
  interactive.
- **Initramfs device nodes without mknod**: this runtime denies
  `mknod`, so the `/dev/console (5:1)` node is written BYTE-WISE into
  the newc cpio (`scripts/mkrootfs.py`); the guest kernel's own
  unpacker creates it. Without a console node, PID 1 is silent — no
  output, no shell, no diagnostics. (`scripts/mkrootfs.py` also
  documents the 110-byte/13-field newc header and that device numbers
  belong in the *rdev* fields; putting them in devmajor/devminor ships
  a character device that is not one.)
- **`RLIMIT_FSIZE` is 500 MiB per file**: keep guests and disks under
  it; the failure signature is `SIGXFSZ` at machine start.

**Drive the guest** (experiment 55-): FIFO-pair transport, readiness
by nonce, marker-bracketed single-shot execs:

```sh
mkfifo "$D/serial.in" "$D/serial.out"
exec 3<>"$D/serial.in" 4<>"$D/serial.out"     # hold BOTH ends O_RDWR
qemu-system-x86_64 ... -serial pipe:"$D/serial" &   # qemu opens EXISTING fifos
```

- Readiness: bootstrap feeding is lossy before the console is up;
  retry-feed `\n` + `echo <nonce>` until the nonce echoes.
- Exec: `echo TAGB; { cmd ; } ; echo TAGE $?` with **line-anchored**
  marker matching — unanchored matches hit the echoed typed command,
  not the output (this cost this repository one debugging cycle).
- In the seccomp-notif driver shape, **drain notifications until the
  tracee exits**: one unanswered notification blocks the tracee in
  that syscall forever.

**Cross-validate every cross-platform run**: compile one `bench.c`
everywhere, print a CHECKSUM, require it identical before comparing
speeds (55-). A platform that computed something different must say so
instead of looking fast.

**In-guest networking**: slirp (`-netdev user`) works for egress; a
TCP host-forward is impossible while TCP bind is denied, so guest↔guest
and host↔guest data move over **UDP** forwards or the serial/FIFO
channel. Guest toolchains: provision the rootfs host-side (copy-in of
a static binary is the minimal form, proven in 55-).

## chroot appliances: native speed, zero boundary

Alpine minirootfs, extracted ownership-neutrally, entered by plain
`chroot` (experiment 60-):

```sh
tar -xzf alpine-minirootfs.tar.gz --no-same-owner -C "$A"
cp /etc/resolv.conf "$A/etc/resolv.conf"
cp ./bench "$A/bench"                # copy-in: mounts are impossible
chroot "$A" /bench                   # native speed, checksum identical to host
```

Measured facts: native speed (817.1 host vs 880.0 chroot Mops/s in
the committed pairing); the appliance ships its own `/etc/passwd`, which the sandbox
lacks; `ps` shows an empty table (no `/proc` — `mount` is filtered);
`/dev/null` may not exist as a device (no mknod, no devtmpfs) so
redirection to it fails; and **there is no kernel boundary** — the
workload shares the tenant's kernel, PID space and namespaces, and the
classic double-chroot escape *precondition* holds for uid 0 (the
logged run proves the precondition: a second chroot from inside
succeeds; it does not demonstrate an escape).

Use it when: you trust the workload and want native speed. Never call
it isolation.

## A ptrace-free tracer (seccomp user notification)

Where the ptrace class is denied, a tracer can still exist — the
tracee seccomps **itself** and hands the tracer a listener fd. All six
primitives are proven live by experiment 45- against
`scripts/senotif_probe.c`:

1. self-install of `SECCOMP_FILTER_FLAG_NEW_LISTENER`;
2. listener handoff over a unix socket (`SCM_RIGHTS`);
3. `NOTIF_RECV`/`NOTIF_SEND` with `USER_NOTIF_FLAG_CONTINUE` shepherds
   every syscall, unprivileged;
4. forged return values (send without CONTINUE, `val` set) — the
   logged tracee read `getuid() == 4242`;
5. `SECCOMP_IOCTL_NOTIF_ADDFD` installs a *real* fd into the tracee —
   the logged tracee read an fd the tracer opened;
6. `/proc/<pid>/mem` serves memory arguments while the tracee is
   blocked in the notification — stable by construction (the tracee
   cannot run), remote pointer = offset, no TOCTOU window.

Hard-won implementation rules, each paid for in this repository's own
debugging:

- **The handoff path must be exempt from the tracee's own filter.** A
  filter that notifies on `sendmsg` deadlocks the handoff: the message
  never leaves a tracee blocked in the notification queue.
- **`TSYNC` + `NEW_LISTENER` is refused (EINVAL)** — a listener
  belongs to one thread's filter. Drop TSYNC.
- **`PR_SET_NO_NEW_PRIVS` must be set** (on this instance it is
  ambient, but a tool must not assume that).
- **Drain until exit.** One notification left unanswered blocks the
  tracee in that syscall forever; the driver's loop ends at
  `RECV → ENOENT`, never at "I got what I came for".
- **The support probe needs proof of liveness**: require the handoff
  AND an actual notification within a timeout. A filter that falls
  through to `RET ALLOW` (architecture mismatch) silently traces
  nothing.
- **Kernel action precedence**: an outer `ERRNO` filter outranks
  `USER_NOTIF`, so syscalls the sandbox denies (ptrace, here) never
  notify — a tracer cannot mediate what an outer layer already refuses,
  and must say so rather than claim coverage.
- open/openat emulation via tracer-side open + ADDFD makes results
  *real* (the tracee's later reads see the tracer's opened file);
  dirfd/`AT_FDCWD` are **tracee-space** values — resolve through
  `/proc/<pid>/fd` and `/proc/<pid>/cwd`, never the tracer's own table.

## Environment completion: living without `/etc/passwd`

Subjects that resolve the invoking user at startup die before doing
anything. Two mechanisms, both in `scripts/`:

- **`libfakepasswd.c`** — `LD_PRELOAD` shim serving a virtual
  `/etc/passwd` and `/etc/group`: hooks `getpwuid_r`/`getpwnam_r` and
  redirects `open`/`openat` of the real paths. Covers dynamically
  linked consumers and cgo-Go user lookups; static and pure-Go raw-
  syscall readers are out of reach by construction (wall 3).
- **An appliance with its own `/etc/passwd`** — the chroot route's
  rootfs ships one (60-); for guest images the file is just another
  cpio entry.

This is environment *completion*, not isolation: kernel checks are
untouched, and reporting a fake uid changes nothing the kernel sees.

## What not to attempt on this class

- TCP listeners of any kind (mechanism P; policy today, but the
  durable design avoids them).
- KVM-accelerated anything (the wall is closed at every measured
  path).
- `runc`-style OCI containers (UTS/mount namespace `unshare` denied).
- ptrace-based anything (tracers, PRoot-class tools, UML guests).
- uid-map-based identity switching (`uid_map` unwritable by anyone —
  procfs provenance).
- Guests with memory images or disks > 500 MiB per file (`SIGXFSZ`).
- In-container overlayfs layering (mount attachment denied) — extract
  ownership-neutrally instead.
