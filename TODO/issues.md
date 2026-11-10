# Issues

Each GitHub issue and open pull request of the record, with its entries.
The State is the state on GitHub: `open`, `closed` or `deleted`.
`TODO/github.txt` lists the numbers: each number from #127 on, and each
open number before #127. The record does not hold the closed issues
before #127; their text is on GitHub, and the hidden ones are in
[the hidden issues](../docs/history/github-hidden-issues-2026-11-06.md).
[RULES.md](RULES.md) gives the rules of the record.

| Issue | Subject | State | Entries |
| --- | --- | --- | --- |
| #86 | [T-1317] userland: relative paths are not virtualised, so every relative open/stat/exec fails ENOENT | closed | T-1643 |
| #87 | [T-1317] every glibc payload is refused: the ABI check resolves imports against libc alone, not the payload's NEEDED set | closed | T-1644 |
| #88 | [T-1317] interposer does not intercept dlopen/dlmopen, so a dynamic payload cannot load guest shared objects | closed | T-1645 |
| #89 | [T-1317] interposer execve maps the path but not the loader, so every dynamic grandchild fails 127: promote clause 17 to a gate | open | T-1646 |
| #90 | [T-1317] a 128 MiB payload-read ceiling refuses node:alpine (149 MB) and any large runtime | open | T-1647 |
| #91 | [T-1409] a static Go payload is declined and then run into a GOROOT error instead of a named refusal | open | T-1648 |
| #92 | [QOL] node:alpine is unusable without an undocumented --entrypoint; the #! refusal should name the way out | open | T-1649 |
| #93 | [CLI] --strict does not refuse the userland grandchild limit, while --strict=all refuses every run | open | T-1650 |
| #94 | [T-1317] static payloads have no rung: add an emulate tier on a prefix-preserving qemu-user | open | T-1651 |
| #109 | [T-1317] static payloads can have a native rung with no ptrace and no qemu: userland-execve + Syscall User Dispatch + openat2(RESOLVE_IN_ROOT) | open | T-1652 |
| #114 | [Reference] Dossier: podbox on the constrained errand host: every limit -> mechanism -> workaround (measured) | open | T-1643 to T-1653 |
| #127 | [Recovered #95] a dynamic grandchild re-enters through the host loader: the loader substitution must cover children | open | T-1646 |
| #128 | [Recovered #96] the absolute lib64/ld-linux-x86-64.so.2 symlink resolves on the host and decides the libc | closed | T-1644 |
| #129 | [Recovered #97] the libc ABI union omits ld-linux-x86-64.so.2 and libdl.so.2, so every glibc payload is refused | closed | T-1644 |
| #130 | [Recovered #104] no -v volumes on a host with no /dev/fuse and no mount: a two-root view, not an overlay | open | T-1653 |
| #131 | [Recovered #105] a relative path gets the rootfs prefix twice when the cwd is already inside the rootfs | closed | T-1643 |
| #132 | [Reference] 89luca89/lilipod @ 872755a: precheck and exec lessons; refused as seed | open | none: a reference (references/README.md) |
| #133 | [Reference] Azathothas/TEMPLATE @ 6206166: methodology docs, notice retained | open | none: a reference (references/README.md) |
| #134 | [Reference] Azathothas/bit-cli @ cce8131: the TODO work model | open | none: a reference (references/README.md) |
| #135 | [Reference] Azathothas/container-research @ 0f155e3: product spec and target measurements | open | none: a reference (references/README.md) |
| #136 | [Reference] Azathothas/memfd-ng @ 5da5803: memfd crate filed under T-0909 | open | none: a reference (references/README.md) |
| #137 | [Reference] Azathothas/pg-toolkit @ 0da2ec9: build-plan digests and loader routes | open | none: a reference (references/README.md) |
| #138 | [Reference] Azathothas/sandbox-insights @ bcf415c: seven-plane model and open questions | open | none: a reference (references/README.md) |
| #139 | [Reference] Obirvalger/vml @ 688496c: machine-as-a-directory shape | open | none: a reference (references/README.md) |
| #140 | [Reference] RuriOSS/ruri @ 711673a: probe-then-refuse discipline | open | none: a reference (references/README.md) |
| #141 | [Reference] RuriOSS/rurima @ 30a0637: refused: tar extraction as root | open | none: a reference (references/README.md) |
| #142 | [Reference] VHSgunzo/memfd-exec @ 9708cb7: memfd fallback and probe-before-use | open | none: a reference (references/README.md) |
| #143 | [Reference] VHSgunzo/pathmap @ 98b3d2a: path half of the interposer | open | none: a reference (references/README.md) |
| #144 | [Reference] VHSgunzo/runimage @ f1512f2: filed: fails where bubblewrap fails | open | none: a reference (references/README.md) |
| #145 | [Reference] VHSgunzo/sharun @ b1ef744: filed: relocation, not virtualization | open | none: a reference (references/README.md) |
| #146 | [Reference] VHSgunzo/ulexec @ 00934f8: two exec rungs driven from one tool | open | none: a reference (references/README.md) |
| #147 | [Reference] VHSgunzo/userland-execve @ none: no such repository; 404 evidence | open | none: a reference (references/README.md) |
| #148 | [Reference] apptainer/apptainer @ 6099bb1: confirms wall 1 for a Go runtime | open | none: a reference (references/README.md) |
| #149 | [Reference] carlbomsdata/winquick @ 095dd47: doctor command shape | open | none: a reference (references/README.md) |
| #150 | [Reference] compforge/pathshim @ 8bcc34e: probe three-channel contract | open | none: a reference (references/README.md) |
| #151 | [Reference] containers/bubblewrap @ 26bb788: clone-vs-mount refusal witness | open | none: a reference (references/README.md) |
| #152 | [Reference] containers/podman @ 7d39ce8: where the layer applier is reached | open | none: a reference (references/README.md) |
| #153 | [Reference] containers/storage @ 83cf574: wall 1 at the line, and its option | open | none: a reference (references/README.md) |
| #154 | [Reference] cubic-vm/cubic @ 4f21a70: verb set and an argument-splitting defect | open | none: a reference (references/README.md) |
| #155 | [Reference] dex4er/fakechroot @ b42d1fb: path model and exclude list | open | none: a reference (references/README.md) |
| #156 | [Reference] dtolnay/faketty @ ce3e201: pty test shape for T-1402 | open | none: a reference (references/README.md) |
| #157 | [Reference] fritzw/ld-preload-open @ 422b2bf: LD_PRELOAD rulings and musl recipe | open | none: a reference (references/README.md) |
| #158 | [Reference] garywill/treesandbox @ 71acbee: refused: dies at getpwuid(0) | open | none: a reference (references/README.md) |
| #159 | [Reference] gevico/tcg-rs @ 88c020b: filed: pure-Rust TCG engine | open | none: a reference (references/README.md) |
| #160 | [Reference] hust-open-atom-club/Vex @ ebee4c7: Docker-like verbs over QEMU configs | open | none: a reference (references/README.md) |
| #161 | [Reference] indigo-dc/udocker @ 638bc42: ownership-neutral extraction and modes | open | none: a reference (references/README.md) |
| #162 | [Reference] io12/userland-execve-rust @ 02ef0e0: userland exec loader, retained unlinked | open | none: a reference (references/README.md) |
| #163 | [Reference] mhx/dwarfs @ 9062b57: short write -20 is a libarchive status | open | none: a reference (references/README.md) |
| #164 | [Reference] multikernel/sandlock @ 841265d: anti-pattern: filtered read, false report | open | none: a reference (references/README.md) |
| #165 | [Reference] novafacing/memfd-exec @ 0a15efe: upstream baseline for the memfd fork | open | none: a reference (references/README.md) |
| #166 | [Scope] Cage recipes: podvm profiles in strict TOML with extends and -s overrides | open | T-1654 |
| #167 | [Scope] Container cages: docker/podman run parity for the sandbox hardening flags | open | T-1655 |
| #168 | [Scope] MicroVMs: fast disposable guests from an OCI image, on KVM or TCG | open | T-1656 |
| #169 | [Scope] User-space kernels (gVisor): a syscall-interpreting tier without user namespaces | open | T-1657 |
| #170 | [Scope] Wrapper sandboxes (bwrap, landrun, bailey): unprivileged Landlock and seccomp confinement | open | T-1658 |
| #171 | [Reference] pkgforge-dev/cross-libc-dlopen @ 34482c7: cross-libc loading rules | open | none: a reference (references/README.md) |
| #172 | [Scope] Provider shapes and bare userlands: Lambda, Cloudflare, Vercel, Nimbus, busybox, distroless profiles | open | T-1659 |
| #173 | [Reference] proot-me/proot @ 65f3f7d: refused: ptrace is filtered | open | none: a reference (references/README.md) |
| #174 | [Scope] Notebooks: Jupyter and marimo runs, and a session that wipes and keeps one directory | open | T-1660 |
| #175 | [Reference] QaidVoid/onelf @ 158b4af: launch ladder, exec lock, symlink check | open | none: a reference (references/README.md) |
| #176 | [Scope] Language runtimes: Node, Deno and Bun permission models, wasmtime and workerd | open | T-1661 |
| #177 | [Reference] qemu-rs/qemu-rs @ 4854135: refused: GPL plugin bindings | open | none: a reference (references/README.md) |
| #178 | [Scope] Network labs: NAT, IPv6, policy and impairment labs inside a guest | open | T-1662 |
| #179 | [Reference] clint/fakeroot (salsa.debian.org) @ 860de25: ownership model and the errno test | open | none: a reference (references/README.md) |
| #180 | [Scope] Tunnels and live uplinks: WireGuard kill switch, WARP uplink, proxy-forced programs | open | T-1663 |
| #181 | [Reference] sigoden/fakepty @ a265016: pty trap catalogue for T-1402 | open | none: a reference (references/README.md) |
| #182 | [Scope] Observability: packet capture, counters and connection logs without CAP_NET_RAW | open | T-1664 |
| #183 | [Reference] talaria0101/dropssh @ 0aafa21: concurrent relay and SSH server | open | none: a reference (references/README.md) |
| #184 | [Scope] Egress broker: a policy proxy and the guest's view of a loopback-only proxy | open | T-1665 |
| #185 | [Reference] talaria0101/nix-experiment @ d8835f2: litmus payload and version forensics | open | none: a reference (references/README.md) |
| #186 | [Scope] Security probe and compare: an inside-the-payload report diffed against a reference | open | T-1666 |
| #187 | [Reference] talaria0101/sandbox-insights @ 0889f5f: probe discipline and the four walls | open | none: a reference (references/README.md) |
| #188 | [Scope] Doctor and installers: host readiness, verified downloads, disk guard | open | T-1667 |
| #189 | [Scope] Findings ledger: tool defects and host traits that change how a result reads | open | T-1668 |
| #190 | [Reference] talaria0101/sandssh @ 4fc7f8c: one-pair relay design | open | none: a reference (references/README.md) |
| #191 | [Reference] talaria0101/vm-research @ 7697b9b: machine tier and TCG benchmarks | open | none: a reference (references/README.md) |
| #192 | [Reference] ylang-ylang/dockless @ ed35b5d: fail-fast CLI guardrails | open | none: a reference (references/README.md) |
| #193 | [Scope] podvm does what sand-net does, without root: index | open | T-1654 to T-1668 |
| #194 | COORD-001: the crates podvm hands over and takes, and the access of T-040 (to podcrates) | open | T-1675 to T-1681 |
| #195 | COORD-002: watch-org.sh sees no repository but the last on Windows: CRLF in the names (to podcrates) | closed (podvm took the pin 54427f6) | none: a COORD issue (to podcrates); podcrates fixed it in `54427f6`, and podvm closes it when it takes that pin |
| #196 | COORD-003: podvm's inventory of the hosts and services of other parties, for pod-services (to podcrates) | open | T-1679 |
| #197 | COORD-004: answers to Q2 and Q3 of the M6 import plan, and the pod-term take (to podcrates) | closed (pod-term taken 75ed306) | T-1675, T-1681 |
| #198 | COORD-005: GitHub Actions of podcrates: the owner's account dispatches ci.yml and release.yml (to podcrates) | closed | none: coordination; the ruling is in docs/decisions.md |
| #199 | [gh action] requested | closed (gate run 42177003497, secrets run 42177007581) | none: a request of a workflow run (AGENTS.md, section 9) |
| #201 | [gh action] requested | closed (gate run 42177673037) | none: a request of a workflow run for T-1604 (AGENTS.md, section 9) |
| #202 | [gh action] requested | closed (gate run 42179344015, green) | none: a request of a workflow run for T-1604 (AGENTS.md, section 9) |
| #203 | COORD-007: podvm took pod-tls 0.4.0 with no tokio, and pod-term moves to v0.4.0 (to podcrates) | closed (podcrates#7 closed) | T-1676 |
| #204 | [gh action] requested | closed (gate run 42209748294, green) | none: a request of a workflow run for T-1674 and T-1676 (AGENTS.md, section 9) |
| #205 | COORD-008: podvm took pod-proxy 0.4.0 (feature blocking) for the relay legs (to podcrates) | closed (podcrates#9 closed) | T-1677 |
| #206 | [gh action] requested | closed (gate run 42214046548, green) | none: a request of a workflow run for T-1674 and T-1677 (AGENTS.md, section 9) |
| #207 | COORD-009: podvm moved to podcrates v0.5.0; the registry patch is yours (to podcrates) | closed (registry patch taken 06f2427) | T-1677 |
| #208 | COORD-010: podvm took the registry patch; ureq leaves, and C10 takes pod-proxy's runtime (to podcrates) | closed (podcrates#13 closed) | T-1677 |
| #209 | [gh action] requested | closed (gate run 42227206423, green) | none: a request of a workflow run for T-1677 and T-1686 (AGENTS.md, section 9) |
| #210 | COORD-011: podvm moved to v0.6.0 and took pod-hostprobe; the freeze of the 29 files ends (to podcrates) | closed (podcrates#11 and #16 closed) | T-1681 |
| #211 | [gh action] requested | closed (gate run 42265769964, green) | none: a request of a workflow run for T-1681 (AGENTS.md, section 9) |
| #200 | COORD-006: the Alpine netboot pins: podvm fetches the 3.22.6 archive (to podcrates) | open | T-1687 |
| #212 | doctor: no row answers whether a memfd can be executed (vm.memfd_noexec), and the memfd rung refuses late | closed | T-1688 |
| #213 | probe: the non-goal `remedy` field contradicts the `stance` in the same JSON object | closed | T-1689 |
| #214 | doctor: the disk-free row reads the hard `RLIMIT_FSIZE` while every other consumer reads the soft one | closed | T-1690 |
| #215 | supervise: `stop -t 0` reports a container as stopped while it is still running, because a bound-setting failure reads as the launcher having gone | closed | T-1691 |
| #216 | windows, parity: the `--podvm-mem` row claims an `RLIMIT_FSIZE` check that neither flavor of the verb performs | closed | T-1692 |
| #217 | release: README and SECURITY put the podbox identity boundary one tag early, and the verifier disagrees with both documents | closed | T-1693 |
| #218 | COORD-012: podvm took v0.7.0, and filed the asks of COORD-015 (to podcrates) | closed | T-1681; none: coordination |
| #219 | COORD-013: podvm took v0.8.0 and pod-supervise (to podcrates) | closed | T-1681 |
| #220 | COORD-014: podvm took v0.9.0, pod-guest and pod-enter (to podcrates) | open | T-1681 |
| #221 | COORD-015: the whiteout escape is fixed in podvm-extract (T-1698) (to podcrates) | open | T-1698 |
| #222 | COORD-016: pod-enter: classify and stage a payload over 128 MiB (T-1647) (to podcrates) | open | T-1647 |
