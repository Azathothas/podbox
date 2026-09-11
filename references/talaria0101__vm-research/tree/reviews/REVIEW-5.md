# REVIEW 5 — benchmark matrix, in-guest toolchain, seccomp-gap value, paper numbers

Date: 2026-09-09 · Trigger: the final pass (benchmarks, in-guest builds,
practical-value demonstration, paper). Every number below re-read from
committed logs at review time.

## Benchmark matrix verification (72-)

| Platform | Log line | Matches paper §5.5 |
|---|---|---|
| host native | `[host] bench 0.04s 721.4 / 754.8 Mops/s checksum=165be307` | ✓ (720–755) |
| microvm-alpine (in-guest gcc) | `[microvm-alpine] bench 0.85s 35.1 Mops/s checksum=165be307` (n=2 identical) | ✓ (35.1) |
| pc-alpine (in-guest gcc) | `0.95s 31.4` then `0.85s 35.1` | ✓ (31.4/35.1) |
| nanos | `0.91s 33.1` | ✓ |
| firecracker CI kernel | `0.87s 34.3`, `0.85s 35.3` | ✓ |
| checksum | single distinct value `165be307` across **all** platforms and both runs of each | ✓ correctness cross-check |

Boot-latency figures cited in §5.5 trace to: microvm 1.6 s (66-, n=3),
pc 3.1 s (20-), nanos ~2 s (45-: guest reaches proof in ≤2 s of guest time,
boot overhead single-digit seconds), firecracker ~25 s+ (65- wall), NetBSD
~300 s (42-, 72-). NetBSD compiler probe (72- follow-up): `ls /usr/bin/cc
/usr/bin/clang` → both `No such file or directory` — base image has no
compiler; reported as boot-only in the paper.

## In-guest toolchain (71-) — final state

`apk add gcc musl-dev` inside the guest over slirp **https**; in-guest
compile of bench.c and hello.c with gcc 14.2.0; in-guest execution:
`VMR-COMPILE-OK`, `bench 1.00s 29.9 Mops/s checksum=165be307` (71- log; the
29.9 vs 35.1 delta vs 72- is the apk-resident rootfs variant on `-M pc` vs
the prepared-rootfs runs — both in-guest-compiled). Root cause of the week-long
-looking failure chain, for the record: Alpine's gcc driver resolves its exec
prefix from `argv[0]`; a PID1 busybox-ash passes the bare name, so `../libexec`
resolves against the wrong base and `cc1`, `crtbeginS.o`, `-lgcc` all fail.
Absolute-path invocation (`/usr/bin/gcc`) fixes all three at once. Secondary
fixes, all logged: virtio_net is a module (ship the module tree + `modprobe`;
busybox `insmod` silently fails on deps), the egress policy is https-only,
and the /8 netmask default of `SIOCSIFADDR` needs an explicit
`SIOCSIFNETMASK`.

## Seccomp-gap practical value (70-) — final claims

1. **Nested userns+netns reachable** through `clone(3)` (parent: `EPERM` on
   the same operation via `unshare`) — filter gap confirmed live.
2. **Private network fabric**: in-process raw-netlink `RTM_NEWLINK` creates a
   veth pair (rc=0), addresses land on both ends, and an ICMP exchange
   completes (`rx type=8` then `type=0` — reply received on loopback and
   across the veth). The parent cannot perform the netlink op
   (`RTNETLINK answers: Operation not permitted`) — the capability is
   gap-granted, not ambient.
3. **NEWPID** private process tree: child reports PID 1.
4. **Chroot appliance**: chroot into an Alpine rootfs with its own
   `/etc/passwd` succeeds (`VMR-CHROOT-OK`) — repairs the sandbox's
   missing-passwd problem for anything running inside.
5. **Boundaries recorded honestly**: `move_mount` → EPERM (no mount
   attachment); `uid_map`/`gid_map` unwritable by anyone (procfs mount
   provenance → `file_ns_capable` fails for init-ns-provenanced mounts) →
   exec drops the capability set → the gap's value is **in-process only**.

A byte-order lesson is retained in the log (checksum stored LE was rejected
`InCsumErrors=1`; `htons()` fix → replies flow): kept because it is exactly
the kind of silent-failure detail the raw logs should preserve.

## Paper consistency check

- §3 Table 1 rows trace to 10-/11-/60-/66- logs and the TLS-egress probe
  captured in 71- (`:80` → EPERM, `:443` → 200, from host).
- §5.1 six paths: each grep-verified in `logs/60-` (ENOENT/EPERM lines
  quoted in REVIEW-4; re-confirmed unchanged).
- §5.5 table: numbers match the matrix log exactly; the ~21× native-gap
  statement: 721/35.1 = 20.5, 755/31.4 = 24.0 — "~21×" is the microvm point,
  range 20–24× overall. Paper wording "bounds, but does not measure"
  retained for the absent KVM baseline.
- §5.6: claims the uid_map EPERM observed in both write directions
  (`logs/70-` census lines) and the exec-capability drop observed via
  `RTNETLINK EPERM` from spawned `ip` vs in-process success.

## Verdict

All paper numbers trace to committed logs; the recommendation (qemu
`-M microvm` + Alpine) follows from the measured table, not preference. The
seccomp-gap practical-value claim is scoped exactly to what was demonstrated
(in-process private network fabric, private PID tree, chroot appliance) with
its boundaries stated. Ready to push.
