# REVIEW 4 — the "harder work" pass: KVM impossibility closed, TCG tier made real

Date: 2026-09-09 · Trigger: the verdict that everything working was "just TCG".
Response: (a) stop treating KVM's absence as a given and hunt for a path
through the seccomp filter; (b) make the non-KVM tier demonstrate actual
microVM capability. Every new claim re-verified against logs at review time.

## 1. The KVM wall was hunted, not assumed (60-)

The 10- probe had a hint: `clone3` returned EINVAL (kernel response) while
`unshare` returned EPERM (filter response) — the filter is a syscall-NUMBER
denylist. The census found and measured real gaps:

- **new mount API unfiltered**: `fsopen("tmpfs")` → fd, `fsconfig(CREATE)` → 0,
  `fsmount` → fd (all executed).
- **clone family unfiltered with namespace flags**: `clone3(NEWUSER|NEWNS)`
  and legacy `clone` both create a **nested user namespace** — a genuine
  sandbox policy gap, now documented in the repo.

Result inside the nested userns (creator-root over its own mountns): still
`open(/dev/kvm) = ENOENT`; `mknod(kvm 10:232)` on the userns-owned tmpfs =
EPERM (init-ns `CAP_MKNOD`); devtmpfs `fsconfig(CREATE)` = EPERM
(`FS_USERNS_MOUNT`); sysfs fsopen = EPERM; `open_by_handle_at` = EPERM;
`iopl(3)`/`perf_event_open`/`kexec_load` = EPERM. The impossibility is now a
measured proof-chain, and it doubles as a security finding about the sandbox
filter (gaps exist; none reach KVM).

## 2. The TCG tier now demonstrates microVM capability (62-, 63-, 64-, 65-, 66-)

| Claim | Log evidence |
|---|---|
| 8/8 concurrent guests, 4.06 boots/s aggregate | `guests booted with proof: 8/8`, `wall 1.965634757`, `aggregate: 4.06 boots/s` (62-) |
| VM-to-VM networking without TCP bind | `PING-from-A-seq1` in guest B's serial (63-) — path: A-slirp → host UDP bind → B hostfwd → B nc:7000 |
| forkd's primitive without KVM | snapshot file 76,401,119 B via `migrate exec:`; `clone-1..3 resumed_ticks=7`, `3/3` (64-) |
| firecracker's own kernel+initramfs run here | guest shell: `uid=0 gid=0`, `/ # echo VMR-FC-ARTIFACTS-OK` → `VMR-FC-ARTIFACTS-OK` (65-) |
| sizing numbers | boot 1.58–1.64 s (n=3); in-guest md5 16 MiB digest `2c7ab85a…` **identical to host**, 0.07–0.08 s vs host 0.026 s (66-) |

Engineering recorded in 65- (each found empirically, each in the log):
firecracker's initramfs ships `/dev` empty → PID1 had no console ("Warning:
unable to open an initial console") until a `/dev/console 5:1` record was
appended as a second cpio archive; `-M microvm` delivers kernel-printk over
the 16550 but userspace ttyS0 writes never surface, while `-M pc` is fully
interactive (`mknod=0 open=3`, `VMR-FD1-WORKS`); `-M microvm` also hangs in
ACPI table parsing without `acpi=off`. 63- required shipping the alpine
`virtio_net.ko` + `net_failover` module tree (`modprobe`, not `insmod` —
silently-failed insmod documented in the debug path).

## 3. Corrections this review forced on the docs

- experiments/README.md and README.md rows for 60/62/63/64/65/66 added with
  the numbers above; bottom-line rewritten ("four end-to-end runnables" now
  plus artifacts/fleet/network/fork capabilities, and the impossibility stated
  as measured proof-chain).

## Verdict

The original finding stands — **KVM is unreachable and now provably so** —
but the repo no longer leans on it: the sandbox runs a fleet, nets its VMs,
clones live machines, and boots firecracker's own artifacts. All new numbers
carry conditions; all are reproducible from the committed scripts.
