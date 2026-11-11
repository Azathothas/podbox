# Findings

This page records each defect of a third-party tool and each trait of a
host that changes how a result reads. Each row names what shows it, and
the host where it was seen. A result from one host class is a property of
that host, never of podvm in general (TODO/gate.md T-1203).

The gate checks each row (check 34, TODO/gate.md T-1668): the mark is
`observed` (a run, its output read) or `inferred` (from reading source);
the evidence is a command in backticks or a file under
`experiments/results/`; the conditions name a host; the podvm state is
`re-measured`, `not reproduced`, `not applicable` or `open`.

## Rules

* Probe by doing. A tool that answers `--version` can still fail at the
  thing it is for (F26).
* An exit status 0 from a pipeline proves nothing: `curl ... | sh` with no
  curl exits 0 and installs nothing.
* A timestamp is not a build identity (F33).
* An auth check is not a working credential (F34).
* A settle loop must probe something the policy lets through, or it
  measures the timeout (F35).
* A comparison between hosts is never a pass or a fail by itself: only an
  invariant section fails, and only without an observed cause (T-1666).

## Host conditions

* **S**, the host of sand-net (issue 189): Debian 13 in an unprivileged
  Incus system container, kernel 6.12.113, 8 vCPU, `/dev/kvm` present,
  root through `sudo -n`, no default route and no UDP out, an HTTP proxy on
  the loopback as the only way out, 2026-11-03. bailey 0.2.4, nftables
  1.1.3, iproute2 6.15.0, podman 5.4.2, docker 26.1.5, QEMU 10.0.13,
  Python 3.13.5. One host only.
* **C1**, the cloud host of podvm's sessions: Linux 6.18.44, x86_64, root
  in a cloud container, no `/dev/kvm` (QEMU runs TCG), no Yama, egress
  through an HTTPS proxy with an allowlist, 2026-11-11.

## The rows of sand-net

sand-net's source is gone. Its rows live in the scope issues that issue
189 maps; each row here cites its issue. The scope issues kept no mark for
most rows, so the mark is a dash where it is unknown. `open` is a row that
podvm can re-test when it reaches the capability that the row names;
`not applicable` is a row of a tool that podvm does not use.

| Id | Tool or trait | Mark | Finding | Evidence | Conditions | podvm |
| --- | --- | --- | --- | --- | --- | --- |
| F1 | bailey | - | bailey v0.2.4's musl release binary cannot start an isolated run on relatime host mounts in a user namespace: remount with MS_STRICTATIME gets EPERM. | issue 170: `nix 0.29 defines ST_RELATIME only for non-musl targets; glibc build of the same tag works; installer self-test` | S | not applicable |
| F1a | bailey | - | Fix paired with F1: the installer self-tests the bailey release and builds the tag from source when the test fails. | issue 170: `installer self-test, then build of the tag` | S | not applicable |
| F2 | bailey (pasta) | - | bailey's --egress-proxy and --proxy-net refuse to start without an IPv4 default route, because pasta copies the default-route interface. | issue 170: `Couldn't set IPv4 route(s) in guest; tools/fake-uplink.sh up (dummy sandup0, 192.0.2.2/24)` | S | not applicable |
| F3 | bailey | - | bailey refuses a policy under a group-writable parent directory without the sticky bit. | issue 170 | S | not applicable |
| F4 | bailey/errand policy | - | cc did not resolve because /usr/bin/cc points into /etc/alternatives, which errand's read list omits, so rustc's linker failed; adding /etc/alternatives fixed it. | issue 170: `/usr/bin/cc -> /etc/alternatives` | S | not applicable |
| F5 | docker | - | The docker daemon could not pull through the loopback proxy; rootless podman pulled using the shell's proxy variables. | issue 167: `lookup registry-1.docker.io ... server misbehaving` | S | not applicable |
| F6 | podman | - | podman build needed --network host to reach a proxy on the host loopback. | issue 167: `podman build --network host` | S | not applicable |
| F7 | iproute2 (ip netns exec) / runc | - | ip netns exec remounts /sys and hides the cgroup mount, so runc failed; nsenter --net=/run/netns/NS works. | issue 167: `no cgroup mount found in mountinfo` | S | not applicable |
| F8 | podman | - | Rootless podman under ip netns exec plus setpriv --reuid failed. | issue 167: `no user mapping found` | S | not applicable |
| F9 | nftables | - | fwd is an nft keyword and cannot name a chain. | issue 178 | S | not applicable |
| F10 | kernel (yama ptrace_scope) | - | kernel.yama.ptrace_scope is 0 here and 1 in the reference host, so /proc/1/mem opens here. Re-measured: the cloud host has no Yama: `/proc/sys/kernel/yama/ptrace_scope` is absent. | issue 186: `kernel.yama.ptrace_scope 0 vs 1; /proc/1/mem` | S; C1 | re-measured |
| F11 | errand (proxy) | - | The reference report showed the errand proxy URL without a credential and CONNECT answering 200 without one, though errand's document says 407; the replica sets auth = false. | issue 184: `CONNECT 200 without credential; auth = false` | S | not applicable |
| F12 | apparmor / pasta | - | apt-get install tayga pulled apparmor, whose pasta and passt profiles in enforce mode made pasta --version exit 139; unloading the two profiles restored pasta. | issue 188: `pasta --version exited 139` | S | not applicable |
| F13 | podman | - | Rootless podman refused every cgroup limit on this host, so any limit forces rootful; rootful --memory 128m gave memory.max 134217728. | issue 167: `rootless needs no limits + no cgrouppath when no permission is granted for cgroups` | S | not applicable |
| F14 | gVisor runsc | - | runsc do -volume wrote into gVisor's in-memory overlay, never the host; and runsc refuses rootless create, so images need rootful podman. Re-test with T-1729 (runsc). | issue 169: `Rootless mode not supported with "create"; runsc do -volume SRC:/workspace` | S | open |
| F15 | QEMU microvm (virtkit 6.18 kernel) | - | The kernel under -M microvm,rtc=on crashed or hung in 5 of 6 boots; with pit=off,pic=off and tsc=reliable 24 of 24 booted. Re-test with the microVM scope (#193). | issue 168: `divide error in kmem_cache_init_late; 24/24 boots` | S | open |
| F16 | QEMU microvm (virtkit 6.18 kernel) | inferred | The kernel prints nothing with console=ttyS0 alone; earlyprintk=serial,ttyS0 gives all output; cause inferred: no 8250 console driver built in. Re-test with the microVM scope (#193). | issue 168: `console=ttyS0 vs earlyprintk=serial,ttyS0` | S | open |
| F17 | bwrap | - | A bwrap bind under /workspace is hidden when the project is bound at /workspace afterwards; tools moved to /opt/sand-bin with SRC:DST. | issue 170: `execvp wasmtime: No such file` | S | not applicable |
| F18 | tayga | inferred | tayga with the well-known prefix 64:ff9b::/96 answered every packet with ICMP unreachable (RFC 6052 forbids non-global IPv4); 2001:db8:64::/96 works. Re-test with T-1662 (network labs). | issue 178: `ICMP unreachable for 64:ff9b::/96` | S | open |
| F19 | tayga | - | apt-get install tayga starts a host service and a nat64 tun device; disable it. Re-test with T-1662. | issue 178: `apt-get install tayga` | S | open |
| F20 | kernel ping_group_range | - | ping_group_range must be one contiguous gid range; in an unprivileged container caller gid and root map far apart, so the lab sets the probing gid. Re-test with T-1662. | issue 178 | S | open |
| F21 | lab world (UDP server) | - | A wildcard-bound UDP server answers from the route's preferred address; the world replies with IPV6_PKTINFO. Re-test with T-1662. | issue 178 | S | open |
| F22 | nftables | - | An nftables drop in OUTPUT is silent for TCP (times out) and returns EPERM to UDP and ping sockets, so a kill switch reads DROPPED for TCP, DENY for UDP. | issue 180 | S | not applicable |
| F23 | cfrs (warp tun) | - | cfrs warp tun worked through the host HTTP proxy, and the TUN kept working after moving to another namespace because the process keeps its descriptor. | issue 180: `cfrs warp tun` | S | not applicable |
| F24 | podman | - | A 0700 project directory made rootless podman keep-id fail to stat it. | issue 167 | S | not applicable |
| F25 | landrun, wasmtime, workerd | - | landrun, wasmtime and workerd releases publish no checksum, and gh release view --json assets returns no digest for them. Not reproduced 2026-11-11: wasmtime v50.0.1 (39 assets) and landrun v0.1.17 (2 assets) give a digest for each asset; workerd v1.20261111.1 publishes no asset on GitHub. | issue 188: `gh release view --json assets (no digest)` and `curl -qfsS https://api.github.com/repos/bytecodealliance/wasmtime/releases/latest` (count `.assets[].digest`) | S; C1 | not reproduced |
| F26 | pasta | - | pasta can print --version and still fail to start a guest when a host route sits on a no-carrier link (docker0 linkdown); intermittent; slirp4netns fallback. | issue 167: `Couldn't set IPv4 route(s) in guest: Invalid argument` | S | not applicable |
| F27 | podman | - | podman 5 copies host proxy variables into every container by default, so a guest held HTTPS_PROXY at 127.0.0.1:3128 and failed; cages pass --http-proxy=false. | issue 184: `HTTPS_PROXY=http://...@127.0.0.1:3128; --http-proxy=false` | S | not applicable |
| F28 | podman | - | --user U:G without a mapping made project and state read-only for uids 993, 1000, 0; fix is rootless --userns keep-id:uid=U,gid=G. | issue 167: `touch: Permission denied` | S | not applicable |
| F29 | pasta / slirp4netns | - | A guest cannot use a host-loopback proxy; pasta does not map it, slirp4netns reaches it at 10.0.2.2 only with allow_host_loopback=true. Re-measured: QEMU user networking of the machine tier reaches a host-loopback proxy at 10.0.2.2, and podvm rewrites the variable (experiment 421, clause 2). | issue 184: `curl -x http://10.0.2.2:3128 https://api.github.com -> 407; refused without it` and experiments/results/egress-broker.txt | S; C1 | re-measured |
| F30 | landrun | - | landrun: ls / and /etc/hostname give EACCES, project stays at host path, /tmp not writable so mktemp fails, no TCP unless net.ports. Re-test with T-1658 (the confine layer). | issue 170: `ls /; /etc/hostname: EACCES; mktemp fails without TMPDIR` | S | open |
| F31 | podman | - | A full disk surfaced as no space left on device in podman's ID-mapped copy, at 188 MiB free of 185 GiB. | issue 167: `no space left on device` | S | not applicable |
| F32 | QEMU microvm (MicroVMInLab test) | - | In a full test suite MicroVMInLab failed twice (outbound IPv4 rows after the VM) and passed in every other run; cause not found. Re-test with the microVM scope (#193). | issue 168: `MicroVMInLab` | S | open |
| F33 | podman (image build) | - | An image built before a change under images/ stays listed and fails; Created never moves on a cache-hit rebuild, so builds carry a sand-net.built label. | issue 167: `can't open file .../sand_nb.py` | S | not applicable |
| F34 | gh / apt-get | - | gh auth status exits 0 for a suspended token while gh release calls return 403; also a fresh image has no apt lists, so bare apt-get install fails. Re-test with T-1667 (the gh fallback). | issue 188: `gh auth status exit 0; gh release 403; Unable to locate package` | S | open |
| F35 | QEMU microvm (detach settle poll) | - | The detach poll probed TCP 22, which web policy drops, so detach waited 60 s per family; probing an allowed port cut MicroVMInLab from 120 s to 56-59 s. Re-test with the microVM scope (#193). | issue 168: `MicroVMInLab 120 s -> 56 to 59 s` | S | open |
| L1 | lab trait | - | The kernel rate-limits ICMP errors; with reject most refusals became silent drops until the router set the ICMP ratelimits to 0. Re-test with T-1662. | issue 178: `net.ipv4.icmp_ratelimit=0; net.ipv6.icmp.ratelimit=0` | S | open |
| L2 | lab trait | - | An unconnected UDP socket never sees port-unreachable, so probes connect first. Re-test with T-1662. | issue 178 | S | open |
| L3 | lab trait | observed | A one-shot resolver failed right after up and passed a minute later; up now sends one packet along each hop; neighbour resolution suspected. Re-test with T-1662. | issue 178 | S | open |
| L4 | lab trait | - | bailey's Landlock port rule refuses bind() to a source port outside net.ports as well as connect(), both with EACCES. Re-test with T-1658. | issue 170: `bind()/connect() EACCES` | S | open |
| L5 | lab trait | observed | The rule ct state new ct status != dnat drop dropped forwarded DNAT flows; accepting ct status dnat first fixed it. Re-test with T-1662. | issue 178: `ct state new ct status != dnat drop` | S | open |
| L6 | lab trait | - | NAT hides nothing inbound unless new flows are dropped. Re-test with T-1662. | issue 178 | S | open |
| L7 | lab trait | observed | After a VM used the veth, IPv6 through the lab stayed dead for tens of seconds in about 1 run in 5; detach now flushes neighbours and polls. Re-test with the microVM scope (#193). | issue 168 | S | open |

## The rows of podvm

| Id | Tool or trait | Mark | Finding | Evidence | Conditions | podvm |
| --- | --- | --- | --- | --- | --- | --- |
| P1 | Bun 1.4.4 (oven/bun:slim) | observed | Bun makes its own system calls past an `LD_PRELOAD` interposer: on the userland rung, `bun b.js` read the host's own `/workspace`, not the volume, and said "Module not found '/workspace/b.js'". | `podvm run --rm --profile bun-runtime -s fs.backend=auto -s fs.image=mirror.gcr.io/oven/bun:slim -- b.js` | C1, the fixture of experiment 356 | re-measured |
| P2 | CPython 3.13 in a venv (ghcr.io/marimo-team/marimo) | observed | Python derives `sys.executable` and its virtual environment from `argv[0]`: with the loader's host path as `argv[0]`, `#!/opt/venv/bin/python3` lost the venv (ModuleNotFoundError: marimo). podvm passes `--argv0` since T-1660. | experiments/results/notebook-cycles.txt (clause 5) | C1, the fixture of experiment 356 | re-measured |
| P3 | marimo 0.25 `export html` | observed | The HTML export holds the last expression of a cell, not what the cell prints: `print(...)` goes to the console, which the export leaves out. | `marimo export html --no-include-code m.py -o out.html` | C1 | re-measured |
| P4 | QEMU (TCG), initrd and RAM | observed | An initramfs larger than the guest's RAM gives no error: the kernel runs out of memory while it unpacks, the guest resets, and QEMU exits 0 with no output. Only an initrd over QEMU's own limit names itself ("initrd is too large"). | `qemu-system-x86_64 -m 512M -kernel K -initrd BIG.cpio -append 'console=ttyS0 panic=-1'` with a 507 MB archive | C1 | re-measured |
| P5 | procps `ps -p PID` | observed | `ps -p` lists a zombie, so "the process is alive" read from it is wrong after a kill: read the state column (Z). | experiments/results/payload-dies-with-launcher.txt | C1 | re-measured |
| P6 | busybox:musl image | observed | The image holds one binary under about 400 hard links; a copy that gives each name its own file grows from 1.2 MB to 500 MB. | `podvm run --rm --podvm-tier=machine mirror.gcr.io/library/busybox:musl echo hi` (before T-1709's fix) | C1 | re-measured |
