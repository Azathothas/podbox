# The sandbox security envelope: seccomp filter reconstruction, gaps, and
# network-policy mechanism

Companion to `docs/paper.md`. Evidence: `experiments/logs/10-`, `60-`,
`70-`, `75-`. Every line in the tables below was produced by a committed
probe script; nothing is inferred from sandbox configuration we cannot see.

## 1. The seccomp filter is a syscall-number denylist

Evidence: identical operations succeed or fail depending on *which syscall
number* performs them.

| Operation | Legacy syscall | Modern syscall | Verdict |
|---|---|---|---|
| enter user+mount ns | `unshare(2)` → EPERM | `clone(2)`/`clone3(2)` + flags → **works** | GAP |
| mount a filesystem | `mount(2)` → EPERM | `fsopen`/`fsconfig`/`fsmount` → **work** (attach: `move_mount` → EPERM, see §3) | PARTIAL GAP |
| path-based open | `open(2)` works | `openat2(2)` → executes (EINVAL on NULL = reached kernel) | unfiltered |
| async IO | — | `io_uring_setup` → executes (EFAULT on NULL) | unfiltered |

A denylist that forgot `clone3` and the mount API is the signature of a
filter written against the classic syscall table and never re-audited. This
is a sandbox-hardening finding: any tenant can enter a user namespace it
owns (and from there hold `CAP_NET_ADMIN` over namespaces it creates) using
`clone3`, while the same operation via `unshare` is denied.

## 2. What the nested user namespace can and cannot do

Measured (60-, 70-):

| Capability in nested userns+netns | Result | Kernel gate |
|---|---|---|
| own the new netns; `CAP_NET_ADMIN` over it | yes (creator) | ns-capable passes |
| raw-netlink `RTM_NEWLINK` veth pair creation | **works** (in-process) | — |
| `SIOCSIFADDR`/`IFF_UP` on the pair | **works** (in-process) | — |
| ICMP echo across the pair | **works** (raw socket, in-process) | — |
| `mknod` char device (10:232) on own tmpfs | EPERM | `CAP_MKNOD` required in *init* userns |
| mount devtmpfs | EPERM at `fsconfig(CREATE)` | `FS_USERNS_MOUNT` not set |
| mount sysfs | EPERM | userns sysfs restrictions |
| write own `uid_map` | EPERM (open) | procfs mount provenance (§4) |
| exec a binary and keep caps | **caps dropped** | uid unmapped (65534) → exec rules |

The gap therefore grants, in-process only: **private, rootless network
fabrics** (veth + addressing + ICMP) and **private PID trees**. It does not
yield a container runtime: mount attachment is denied and `execve` drops the
granted capability set because the namespace's uid map cannot be installed
(§4).

## 3. Boundaries that are *not* the seccomp filter

Two denials would be misattributed to seccomp by a careless read:

1. `move_mount` → EPERM. Attaching a mount requires `CAP_SYS_ADMIN` over the
   mount namespace's owner. Our mount namespace is owned by the parent userns
   (the runtime created it before we entered), so this fails even for the
   nested-ns child that owns *its* mount ns clone — the clone inherits mounts
   whose ownership... (measured: EPERM both in parent and nested child).
2. `uid_map`/`gid_map` writes → EPERM **at open**, from parent and child
   alike. `map_write()` checks `file_ns_capable(file, ...)`: our `/proc` was
   mounted from the initial user namespace, so the check demands
   initial-ns `CAP_SETUID`... i.e. no tenant process can ever install a uid
   map. Consequence: every nested-ns process runs with unmapped uid (65534),
   and any `execve` drops its capability set (standard exec rules) — which is
   why the §5.2 network-fabric demonstration had to be performed in-process.

## 4. The network policy mechanism is NOT seccomp

A seccomp filter evaluates syscall arguments it can read: syscall number and
register values. It **cannot dereference the `sockaddr` pointer** passed to
`bind`/`connect`, so it cannot distinguish `connect(127.0.0.1)` from
`connect(1.2.3.4)`. Our measurements (75-) show address- and port-aware
denials:

| Probe | Result |
|---|---|
| `bind` TCP any addr, any port (loopback / wildcard / host LAN IP) | EPERM |
| `bind` UDP loopback / wildcard | OK |
| `bind` AF_UNIX | OK |
| `connect` TCP `127.0.0.1` (own listener) | EPERM |
| `connect` TCP `192.168.1.64:443` (host LAN IP, closed port) | **ECONNREFUSED** — policy-allowed, kernel answered |
| `connect` TCP `10.0.2.2:443` (RFC1918) | timeout (no route) |
| `connect` TCP external `:80` | **denied early-session** → **allowed after live policy relaxation** |
| `connect` TCP external `:443` | OK |

Address-awareness (127.0.0.1 vs 192.168.1.64), port-awareness (:80 vs :443
— including a **live relaxation mid-session**), family-awareness (TCP bind
denied, UDP bind allowed), while returning `EPERM` — this is the signature
of a **cgroup-BPF `SOCK_ADDR` program family** (`BPF_CGROUP_INET4_BIND`,
`BPF_CGROUP_INET4_CONNECT`) attached to the sandbox's cgroup. Such programs
can read the full sockaddr, return `EPERM`, and be reconfigured at runtime —
matching all three observations. Plain seccomp matches none of them.

Design consequence for tools (e.g. a podman-style microVM runner): never
plan around TCP listeners; plan around **UDP endpoints, unix sockets, and
serial consoles**. The asymmetry (UDP bind allowed, TCP bind denied) is
itself a tool design input.

## 5. Hard limits relevant to any implementation

| Limit | Value | Impact |
|---|---|---|
| `RLIMIT_FSIZE` | 500 MiB/file | memory images, disks must be sized below; `SIGXFSZ` kills |
| `/tmp` | 64 MiB tmpfs | stage artifacts on `/workspace` |
| `/etc/passwd` | absent, root fs read-only | need `LD_PRELOAD` shim (`patches/libfakepasswd.c`) or chroot appliance (70-) |
| serial transport | userspace writes only on `-M pc`; `-M microvm` drops them | exec protocols: use pc |
| poweroff | `-M pc,acpi=off` cannot power off (halts) | treat "System halted" + marker as completion; kill qemu from the driver |
| guest networking | slirp only (no `/dev/net/tun`) | UDP hostfwd works; TCP hostfwd impossible |
