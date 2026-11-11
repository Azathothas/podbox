# net

Record semantics: [task rules](RULES.md#5-entry-closure).


This page holds the network work of the widened scope: network labs,
tunnels and live uplinks, packet capture and counters, and the egress
broker. The owner ruled on 2026-11-06 that podvm does what sand-net did,
without root. Issue 193 is the index of that scope.

No other area owns the subject. [machine](machine.md) owns the tier
that hosts a lab, and [cli](cli.md) owns the verbs. The labs, the
broker and the capture are a network subject on both tiers, and the
podcrates crates that serve them (`pod-netlab`, `pod-pcap`, `pod-egress`,
`pod-broker`, `pod-netstack`) are network crates.

The target host has no network namespace, no `/dev/net/tun`, no root and
no `CAP_NET_RAW`. Its only way out is an HTTP CONNECT proxy. A lab
therefore runs inside one machine-tier guest, where the guest kernel is
root, and a rootfs-tier payload shares the host network.

The current work order is [PROGRESS.md](PROGRESS.md).

---

### T-1662 Network labs: NAT, IPv6, policy and impairment labs inside one guest

Source:      issue 178; #193; the owner's ruling of 2026-11-06
Category:    net
Priority:    P2
Effort:      XL
Status:      open

Problem:     sand-net built 39 lab scenarios from three network namespaces
             and checked each with a self-check that derived every expected
             verdict from the lab's settings. The target host has no
             network namespace and no `/dev/net/tun`, so podvm cannot build
             a lab on the host.
Premise:     A machine-tier guest kernel is root inside, so namespaces,
             nftables, `tc netem`, tayga and WireGuard work there. The
             machine tier runs under TCG on the target host
             (`crates/podvm/src/tier.rs:144`), at 3x to 21x the native cost
             (T-1308). User-mode networking carried a guest in
             `experiments/361-guest-usernet.sh:299`. Running a guest from
             `run` is T-1656. The issue names `pod-dns` and `pod-proxy`;
             podcrates names `pod-netlab` (wave M7) on `pod-netstack`.
Approach:    Add `podvm lab up|down|check|try|matrix` that boots one guest
             with a lab image and kernel options (netns, nf_tables, netem,
             wireguard, tun). Port the lab layout and the knobs unchanged.
             The guest only measures; the host derives each expected
             verdict from the lab's settings and compares, and reads each
             nftables rule counter before and after. Refuse contradictory
             knobs. For a policy-only lab, offer a lighter variant on a
             user-space stack with no kernel, from podnet's `podnet net`.
Decision:    The check design ports as it is: measure in the guest, derive
             on the host, read counters, plant a failure. A rootfs-tier
             payload as the lab's guest is measured or refused with the
             reason. Take `pod-netlab` and `pod-netstack` when podcrates
             releases them; the world's DNS and proxy come from podcrates
             crates or the entry records why not.
Prove:       `cargo test -p podvm` exits 0 with tests that derive the
             expected verdicts of two scenarios from their knobs.
             `sh experiments/418-net-lab.sh` exits 0 on a TCG-only host:
             `podvm lab try dual-nat` prints PASS rows, `podvm lab matrix`
             runs every offline scenario with its time, and a flushed
             policy chain inside the guest makes the check fail.

**Filed 2026-11-06.** Quoted from issue 178:

```text
namespaces sn-NAME-g|r|w: guest 10.77.0.2 fd00:77::2; router 10.77.0.1,
  fd00:77::1 and 198.51.100.1, 2001:db8:100::1; world 198.51.100.10
knobs: --stack dual|v4|v6, --nat none|nat44|nat66|both, --policy
  open|web|web-nodns|only443|blackout, --broken, --mtu 1280..1499,
  --icmp drop, --delay/--loss/--rate (netem), --dnat, --hairpin, --nat64,
  --cgnat
verdicts: ALLOW, DROPPED, REFUSED, DENY, UNRESOLVED, TIMEOUT
L1: the kernel rate-limits ICMP errors; reject read as silent drops until
  icmp_ratelimit=0
L2: an unconnected UDP socket never sees port-unreachable
L5: `ct state new ct status != dnat drop` dropped forwarded DNAT flows
F18: tayga with 64:ff9b::/96 refused non-global IPv4 (RFC 6052);
  2001:db8:64::/96 works
F20: ping_group_range must be one contiguous gid range
F21: a wildcard-bound UDP server answers from the route's preferred
  address; reply with IPV6_PKTINFO
```

---

### T-1663 Tunnels and live uplinks: WireGuard kill switch, WARP uplink, proxy-forced programs

Source:      issue 180; #193; the owner's ruling of 2026-11-06;
             A-V-02 of PodUtils/podcrates#39
Category:    net
Priority:    P2
Effort:      L
Status:      open

Problem:     sand-net ran WireGuard labs with a kill switch, a live WARP
             uplink, and a mode that forced a proxy-ignorant program
             through a proxy. podvm has none of the three, and the target
             host has no kernel facility for the first two.
Premise:     WireGuard and the kill switch need a kernel, which the lab
             guest of T-1662 has. podnet's `warp` needs no TUN in its proxy
             and `run` modes; its TUN mode needs `CAP_NET_ADMIN`, which the
             guest has. podnet's `pod` (crate `podnet-netns`, seccomp
             backend) forces a program's traffic through a proxy with no
             root. The issue names `pod-proxy` and `pod-tls`.
Approach:    Run the vpn scenarios inside the T-1662 guest unchanged. Run
             podnet inside the guest as the router's uplink, its outer
             connection through user-mode networking and the host proxy.
             On the rootfs tiers, add a net mode that reuses
             `podnet-netns` to force a payload through a proxy URL. For a
             kill switch without a namespace, refuse in a Syscall User
             Dispatch handler (T-1652) every `connect` but the proxy.
Decision:    podvm reuses podnet's crate and does not write a second one. A
             kill switch on a rootfs tier is per payload, not per host, and
             the output says so. A dead upstream fails the connection
             visibly, not by a reset after a successful `connect` (cfrs
             issue 190).
Prove:       `cargo test -p podvm` exits 0 with tests for the net mode's
             argument parse. `sh experiments/419-net-tunnels.sh` exits 0:
             vpn-full, vpn-leak and vpn-killswitch pass in the lab guest with
             the `leak` counter moving only in the kill-switch case, a
             proxy-ignorant program reaches the world through the proxy
             mode, and a dead upstream fails that program's connection
             visibly. The live warp-uplink leg runs where the operator
             grants a live run.

**Filed 2026-11-06.** Quoted from issue 180:

```text
--vpn full|split: tunnel 10.99.0.0/24 and fd99::/64, port 51820,
  persistent-keepalive 1; full routes 0.0.0.0/1, 128.0.0.0/1, ::/1, 8000::/1
--killswitch: accept lo, wg0, IPv6 ND and UDP 51820 to the endpoint; drop the
  rest with a counter named `leak`
F22: an nftables drop in OUTPUT is silent for TCP and EPERM for UDP and
  ping sockets, so a kill switch reads DROPPED for TCP and DENY for UDP
F23: `cfrs warp tun` kept working after its TUN moved to another namespace
cfrs 0.3.0-beta.8 on a proxy-only host: `cfrs doctor` 2 of 4 edge legs;
  in the warp-uplink lab 4 of 4; warp-v6-only 2 of 4
quick-tunnel provisioning hit 429 with Retry-After: 1379 on the eighth
  provision in about 70 minutes
```

**Amended 2026-11-09.** The ask A-V-02 of PodUtils/podcrates#39 (COORD-015,
a message from a peer session) wants a payload held to one egress. The hold
is the shared mechanism of this entry and of T-1665: a Syscall User Dispatch
handler (T-1652) or podnet's `pod`. podcrates T-109 studies the hold of
podnet. T-1665 holds the clauses of the ask. This entry changes no clause.

---

### T-1664 Observability: capture, counters and connection logs without `CAP_NET_RAW`

Source:      issue 182; #193; the owner's ruling of 2026-11-06
Category:    net
Priority:    P2
Effort:      L
Status:      open

Problem:     sand-net captured packets, read per-interface and rule
             counters, and watched rates inside a lab, all through `sudo`.
             On the target host `AF_PACKET` needs `CAP_NET_RAW`, which an
             unprivileged process does not have. A rootfs-tier payload
             shares the host network, so nothing can capture its packets.
Premise:     Inside the T-1662 guest the kernel is root, so capture and
             counters run there. QEMU's `filter-dump` writes a pcap of a
             guest's user-mode network with no privilege. The interposer
             and the T-1652 handler see each `connect`, `sendto` and `bind`
             call. The issue names podcrates `pod-pcap` (wave M7) through
             the podcrates crate table.
Approach:    Add `podvm lab capture|stats|watch` that runs the capture and
             the counters in the guest and streams them out on a second
             channel. For a machine-tier run with user-mode networking,
             write a host-side pcap through `filter-dump`. For a rootfs-tier
             payload, write a connection log at the syscall layer: address,
             port, result and errno for each call.
Decision:    A connection log is not a packet capture, and its output says
             so. The counters are part of correctness: the lab check fails
             when a rule that should match did not. The proxy variables of
             the calling shell are removed in a lab shell, because they
             name a host loopback that the lab does not have. Take
             `pod-pcap` when podcrates releases it.
Prove:       `cargo test -p podvm` exits 0 with a test for the connection
             log record format. `sh experiments/420-net-capture.sh` exits 0:
             `podvm lab capture LAB --at router --iface rw -w out.pcap`
             writes a pcap that `tcpdump -r` reads and that shows the NATed
             source, `podvm lab stats LAB --json` lists the rule counters by
             name, a machine-tier run writes a host-side pcap with no
             privilege, and a rootfs-tier run logs each connect with its
             errno.

**Filed 2026-11-06.** Quoted from issue 182:

```text
sand net capture LAB [--at guest|router|world|carrier] [--iface IF] [-w FILE]
  [--seconds N] [-c N] [-q]: AF_PACKET with ETH_P_ALL; Ethernet keeps link
  type 1, wg0 is raw IP (type 101); the file is chowned to the caller
sand net stats: ip -j -s link, tc -s -j qdisc, nf_conntrack_count, and the
  router's nft counters return, open, tcp, udp, dropped, broken, inbound,
  forwarded, ping
in dual-nat, 10.77.0.2 leaves rw as 198.51.100.1
F7: `ip netns exec` remounts /sys; a capture helper is fine, runc is not
not covered: capture inside a cage's own namespace
```

---

### T-1665 An egress broker, and the guest's view of a loopback-only proxy

Source:      issue 184; #193; the owner's ruling of 2026-11-06;
             A-V-02, A-V-03 of PodUtils/podcrates#39
Category:    net
Priority:    P2
Effort:      L
Status:      partial 2026-11-11

Problem:     The target host's only way out is an HTTP CONNECT proxy, often
             on the host loopback. A machine-tier guest cannot reach a
             loopback proxy without a rewrite. podvm has no egress broker
             that holds a payload to an allowlist, as errand's broker and
             sand-net's replica did.
Premise:     A rootfs-tier payload shares the host network, so a loopback
             proxy is reachable and the variables pass through unchanged.
             QEMU user-mode networking maps the host loopback at 10.0.2.2
             unless `restrict=on`; the usernet experiment routes through
             that gateway (`experiments/361-guest-usernet.sh:223`). The
             tree reads no proxy variable today. The issue names podcrates
             `pod-proxy` and `pod-tls`; podcrates names `pod-broker` for
             the broker (podcrates T-099, T-100, call C25). `pod-egress` keeps the
             roads (podcrates T-035).
Approach:    On the machine tier, rewrite each proxy variable whose host is
             `127.0.0.1`, `localhost` or `::1` to 10.0.2.2, scheme, port and
             credentials byte for byte. Add `--net broker` with `--ports`
             and `--allow`, built on `pod-broker` and `pod-proxy`, chained
             to the host proxy. Hold a payload to the broker with a Syscall
             User Dispatch handler that refuses every other `connect`
             (T-1652), or with podnet's `pod`.
Decision:    The broker's 407, 403, 502 and 504 vocabulary of issue 184 is
             the specification. A profile never claims `broker` where no
             holding mechanism is active. A seccomp filter cannot read a
             sockaddr, so it is not a holding mechanism. The credential
             never goes in argv or in a URL that podvm logs.
Prove:       `cargo test -p podvm` exits 0 with rewrite tests: a password
             with `@` survives, a percent-escaped password is not decoded,
             a URL with no port or no scheme is refused.
             `sh experiments/421-egress-broker.sh` exits 0 on a
             loopback-proxy host: a rootfs-tier payload fetches through the
             unchanged variables, a machine-tier guest gets the proxy at
             the gateway with credentials intact, `--net broker --ports 443`
             answers 403 for port 22 and for 127.0.0.1, 407 without the
             session credential and 200 for an allowed host, and a payload
             that ignores the variables cannot reach the network directly. A `connect` to a named upstream
             reaches the broker on its Unix socket, the name resolves to a fake
             address, and with no holding mechanism the run refuses by name.

**Partial 2026-11-11.** The rewrite half is done. `cargo test -p podvm`
exits 0 with `a_loopback_proxy_moves_to_the_gateway_byte_for_byte` (a
password with `@` survives, a percent escape is not decoded) and
`a_url_with_no_port_or_no_scheme_is_refused_without_its_value`
(`crates/podvm/src/machine/proxy.rs`). On the machine tier, each proxy
variable of the payload whose host is `127.0.0.1`, `localhost` or `::1`
names the gateway 10.0.2.2, and a proxy URL with no scheme or no port
refuses the run, naming the variable and never its value.
`PODVM_421_MIRROR=1 sh experiments/421-egress-broker.sh` exits 0
(`experiments/results/egress-broker.txt`): with a loopback proxy of the
script that wants a Basic credential with an `@` in its password, a
rootfs-tier payload fetches through the unchanged variable (200), a TCG
guest sees `http://CRED@10.0.2.2:PORT` and fetches (200), the proxy
answers 407 without the credential, and a URL with no port refuses the
guest by name with no password in the message.

Measured beside it: the guest reached this host's own loopback proxy at
10.0.2.2, and its HTTPS through that proxy failed the certificate check,
because that proxy presents its own CA and the guest has no host CA
completion (the completion of T-0407 runs on the rootfs tiers).

The broker half, 2026-11-11: `run --network broker --ports P --allow HOST`
hosts `pod-broker` for the run (`crates/podvm/src/broker.rs`): one job with
a token of 32 random bytes, a policy that allows each host of `--allow` on
the ports of `--ports` (443 by default), and the host's proxy as the way
out (the target's name goes to it). The payload's proxy variables name the
broker with the token as the password; the token is in no argv and in no
line that podvm prints. Two holding mechanisms:

* the syscall tier (T-1657): the handler lets `connect` reach the broker's
  port on the loopback and Unix sockets only, and refuses a datagram with
  an address and `io_uring_setup`, with EACCES
  (`crates/podvm-dispatch/src/hold.rs`, the test
  `the_hold_lets_the_payload_reach_the_broker_only`);
* the machine tier: the guest network is `restrict=on`, and its one way
  out is `guestfwd` at 10.0.2.100:3128 to a relay of podvm for each
  connection (`--podvm-relay`; a `tcp:` chardev is one connection for the
  whole run, and the first try hung on it).

Each other rung refuses `--network broker` by name: a payload that ignores
the variables would reach the network there. A profile takes `net.mode =
"broker"`, `net.ports` and `net.allow`. Experiment 421 clauses 5 to 7 (exit
0): on the syscall tier (the static busybox, on the fixture) and in a TCG
guest, the allowed host answers 200, port 22 and 127.0.0.1 answer 403, no
credential answers 407, and a direct connect fails (EACCES; refused in the
guest); with no holding mechanism the run refuses (exit 125). busybox:musl
has no TLS client, so the clauses allow port 80 beside 443 and fetch the
allowed host over plain HTTP.

Remaining clause: a `connect` of a payload that ignores the variables to a
named upstream reaches the broker (its Unix socket), the name resolving
to a fake address that maps to the name. Today such a payload is refused,
not served.

**Filed 2026-11-06.** Quoted from issue 184:

```text
broker: Basic Proxy-Authorization with a per-run 32-byte hex secret, else
  407; port or host off the lists: 403 not on the egress allowlist; name
  not resolved in 5 s: 504; not a public host (loopback, private,
  link-local, reserved, 168.63.129.16): 403; dial over 10 s: 502
F29: 127.0.0.1 in the guest is the guest; slirp4netns reaches the host
  loopback only with allow_host_loopback=true, at 10.0.2.2:
  `curl -x http://10.0.2.2:3128 https://api.github.com` gave 407; without it
  the connection was refused
F27: podman 5 copies the host proxy variables into every container
  (--http-proxy defaults to true), so a guest failed naming itself
F11: the reference report showed CONNECT answering 200 with no credential,
  while errand's document says 407
busybox wget sent plain HTTP to the proxy and got 404
```

**Amended 2026-11-09.** The asks A-V-02 and A-V-03 of PodUtils/podcrates#39
(COORD-015, a message from a peer session) and the call C25 of podcrates
change this entry. C25: the broker is the new crate `pod-broker` (podcrates
T-099, T-100), not `pod-egress`. The Premise and the Approach now name
`pod-broker`, and `--net broker` uses it. A-V-03 wants the policy of the
egress proxy of cfb. That policy is an input of `pod-broker`: the allowed
hosts and ports, the per-run credential, the rule for public hosts and the
407, 403, 502 and 504 answers. `--net broker` loads it and does not write a
second copy. A-V-02 wants a payload held to one egress. Each `connect` to a
named upstream goes to the broker on a Unix socket. A fake DNS answers each
name with one stable fake address, and the hold maps that address to the
upstream. When no holding mechanism works, the run refuses by name before
the start. The Decision already says this.

**Amended 2026-11-10, for podrunner.** The podrunner design review (the
owner's design review of PodUtils/podcrates#47; the ruling "podrunner",
2026-11-10; SPEC sections 9.1, 9.5 and its unknown U9) holds a podvm unit with
`podnet pod` around `podvm run`, and calls that hold advisory. podnet owns
the measurement of the pod with the runner (COORD-022, issue 228). One
clause is podvm's: the experiment adds a run of `podnet pod
--default-action reject -x http://unix:/PATH -- podvm run IMAGE`, where
the payload's TCP reaches the Unix upstream, a direct connect is refused,
podvm's own pull before the payload works or refuses by name, and the
banner names the rung. The review writes the upstream as
`socks5://unix:PATH`; `pod-broker` speaks HTTP CONNECT only, and podnet
parses `http://unix:/PATH` (COORD-021, issue 227, section 0, item 4).
Under the seccomp listener of the pod the kernel refuses a second
listener, so the probe rejects `supervise` there (Inferred, not run); the
experiment records the rung that it selects. The report of T-1695 names an
outer hold only where podvm measures it, else `null`.
