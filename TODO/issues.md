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
| #89 | [T-1317] interposer execve maps the path but not the loader, so every dynamic grandchild fails 127: promote clause 17 to a gate | closed | T-1646 |
| #90 | [T-1317] a 128 MiB payload-read ceiling refuses node:alpine (149 MB) and any large runtime | closed | T-1647 |
| #91 | [T-1409] a static Go payload is declined and then run into a GOROOT error instead of a named refusal | closed | T-1648 |
| #92 | [QOL] node:alpine is unusable without an undocumented --entrypoint; the #! refusal should name the way out | closed | T-1649 |
| #93 | [CLI] --strict does not refuse the userland grandchild limit, while --strict=all refuses every run | closed | T-1650 |
| #94 | [T-1317] static payloads have no rung: add an emulate tier on a prefix-preserving qemu-user | closed | T-1651 |
| #109 | [T-1317] static payloads can have a native rung with no ptrace and no qemu: userland-execve + Syscall User Dispatch + openat2(RESOLVE_IN_ROOT) | closed | T-1652 |
| #114 | [Reference] Dossier: podbox on the constrained errand host: every limit -> mechanism -> workaround (measured) | closed (lesson taken in T-1643 to T-1653) | T-1643 to T-1653 |
| #127 | [Recovered #95] a dynamic grandchild re-enters through the host loader: the loader substitution must cover children | closed | T-1646 |
| #128 | [Recovered #96] the absolute lib64/ld-linux-x86-64.so.2 symlink resolves on the host and decides the libc | closed | T-1644 |
| #129 | [Recovered #97] the libc ABI union omits ld-linux-x86-64.so.2 and libdl.so.2, so every glibc payload is refused | closed | T-1644 |
| #130 | [Recovered #104] no -v volumes on a host with no /dev/fuse and no mount: a two-root view, not an overlay | closed | T-1653 |
| #131 | [Recovered #105] a relative path gets the rootfs prefix twice when the cwd is already inside the rootfs | closed | T-1643 |
| #132 | [Reference] 89luca89/lilipod @ 872755a: precheck and exec lessons; refused as seed | closed (lesson taken in T-0502, T-0603, T-0806) | none: a reference (references/README.md) |
| #133 | [Reference] Azathothas/TEMPLATE @ 6206166: methodology docs, notice retained | closed (lesson taken: docs/ methodology; notice in THIRD_PARTY.md (T-1606)) | none: a reference (references/README.md) |
| #134 | [Reference] Azathothas/bit-cli @ cce8131: the TODO work model | closed (lesson taken: the TODO/ work model) | none: a reference (references/README.md) |
| #135 | [Reference] Azathothas/container-research @ 0f155e3: product spec and target measurements | open | none: a reference (references/README.md); open: `AGENTS.md`, section 13, routes the product contract to it |
| #136 | [Reference] Azathothas/memfd-ng @ 5da5803: memfd crate filed under T-0909 | closed (lesson taken in T-0909) | none: a reference (references/README.md) |
| #137 | [Reference] Azathothas/pg-toolkit @ 0da2ec9: build-plan digests and loader routes | closed (lesson taken in T-1349; the loader table is an unfiled candidate) | none: a reference (references/README.md) |
| #138 | [Reference] Azathothas/sandbox-insights @ bcf415c: seven-plane model and open questions | closed (lesson taken in T-0415, T-1308) | none: a reference (references/README.md) |
| #139 | [Reference] Obirvalger/vml @ 688496c: machine-as-a-directory shape | closed (ruled in T-1307) | none: a reference (references/README.md) |
| #140 | [Reference] RuriOSS/ruri @ 711673a: probe-then-refuse discipline | closed (lesson taken in T-0107, T-0804; podcrates pod-hostprobe) | none: a reference (references/README.md) |
| #141 | [Reference] RuriOSS/rurima @ 30a0637: refused: tar extraction as root | closed (refused; lesson taken in T-0301; podcrates pod-extract) | none: a reference (references/README.md) |
| #142 | [Reference] VHSgunzo/memfd-exec @ 9708cb7: memfd fallback and probe-before-use | closed (lesson taken in T-0203, T-0909; not vendored) | none: a reference (references/README.md) |
| #143 | [Reference] VHSgunzo/pathmap @ 98b3d2a: path half of the interposer | closed (lesson taken in T-0701 to T-0705) | none: a reference (references/README.md) |
| #144 | [Reference] VHSgunzo/runimage @ f1512f2: filed: fails where bubblewrap fails | closed (filed: no use; decision in TODO/reference-map.md) | none: a reference (references/README.md) |
| #145 | [Reference] VHSgunzo/sharun @ b1ef744: filed: relocation, not virtualization | closed (filed: no use; decision in TODO/reference-map.md) | none: a reference (references/README.md) |
| #146 | [Reference] VHSgunzo/ulexec @ 00934f8: two exec rungs driven from one tool | closed (lesson taken in T-0905, T-0909) | none: a reference (references/README.md) |
| #147 | [Reference] VHSgunzo/userland-execve @ none: no such repository; 404 evidence | closed (no repository; 404 evidence in TODO/reference-map.md) | none: a reference (references/README.md) |
| #148 | [Reference] apptainer/apptainer @ 6099bb1: confirms wall 1 for a Go runtime | closed (lesson taken in T-0706; podcrates pod-enter, pod-extract) | none: a reference (references/README.md) |
| #149 | [Reference] carlbomsdata/winquick @ 095dd47: doctor command shape | open | none: a reference (references/README.md); open: the Source of T-1112 (partial) |
| #150 | [Reference] compforge/pathshim @ 8bcc34e: probe three-channel contract | closed (lesson taken in T-0101, T-0108, T-0110) | none: a reference (references/README.md) |
| #151 | [Reference] containers/bubblewrap @ 26bb788: clone-vs-mount refusal witness | closed (lesson taken in T-0102; podcrates pod-hostprobe) | none: a reference (references/README.md) |
| #152 | [Reference] containers/podman @ 7d39ce8: where the layer applier is reached | closed (lesson taken in T-0205) | none: a reference (references/README.md) |
| #153 | [Reference] containers/storage @ 83cf574: wall 1 at the line, and its option | closed (lesson taken in T-0205, T-0302; podcrates pod-extract) | none: a reference (references/README.md) |
| #154 | [Reference] cubic-vm/cubic @ 4f21a70: verb set and an argument-splitting defect | closed (lesson taken in T-1301, T-1302, T-1307) | none: a reference (references/README.md) |
| #155 | [Reference] dex4er/fakechroot @ b42d1fb: path model and exclude list | closed (lesson taken in T-0707) | none: a reference (references/README.md) |
| #156 | [Reference] dtolnay/faketty @ ce3e201: pty test shape for T-1402 | closed (lesson taken in T-1402) | none: a reference (references/README.md) |
| #157 | [Reference] fritzw/ld-preload-open @ 422b2bf: LD_PRELOAD rulings and musl recipe | closed (lesson taken in T-0701, T-0702, T-1002) | none: a reference (references/README.md) |
| #158 | [Reference] garywill/treesandbox @ 71acbee: refused: dies at getpwuid(0) | closed (refused; lesson taken in T-0404) | none: a reference (references/README.md) |
| #159 | [Reference] gevico/tcg-rs @ 88c020b: filed: pure-Rust TCG engine | closed (filed; ruled in T-1307) | none: a reference (references/README.md) |
| #160 | [Reference] hust-open-atom-club/Vex @ ebee4c7: Docker-like verbs over QEMU configs | closed (ruled in T-1307) | none: a reference (references/README.md) |
| #161 | [Reference] indigo-dc/udocker @ 638bc42: ownership-neutral extraction and modes | closed (lesson taken in T-0301 to T-0307, T-0404; podcrates pod-extract) | none: a reference (references/README.md) |
| #162 | [Reference] io12/userland-execve-rust @ 02ef0e0: userland exec loader, retained unlinked | open | none: a reference (references/README.md); open: T-1652 and T-1684 need the vendored upstream |
| #163 | [Reference] mhx/dwarfs @ 9062b57: short write -20 is a libarchive status | closed (lesson taken in T-0203; podcrates pod-hostprobe) | none: a reference (references/README.md) |
| #164 | [Reference] multikernel/sandlock @ 841265d: anti-pattern: filtered read, false report | closed (lesson taken in T-0107, T-0606, T-0804, T-0903; podcrates pod-hostprobe) | none: a reference (references/README.md) |
| #165 | [Reference] novafacing/memfd-exec @ 0a15efe: upstream baseline for the memfd fork | closed (lesson taken in T-0909; not vendored) | none: a reference (references/README.md) |
| #166 | [Scope] Cage recipes: podvm profiles in strict TOML with extends and -s overrides | closed | T-1654 |
| #167 | [Scope] Container cages: docker/podman run parity for the sandbox hardening flags | closed | T-1655 |
| #168 | [Scope] MicroVMs: fast disposable guests from an OCI image, on KVM or TCG | closed | T-1656 |
| #169 | [Scope] User-space kernels (gVisor): a syscall-interpreting tier without user namespaces | closed | T-1657, T-1729 |
| #170 | [Scope] Wrapper sandboxes (bwrap, landrun, bailey): unprivileged Landlock and seccomp confinement | open | T-1658 |
| #171 | [Reference] pkgforge-dev/cross-libc-dlopen @ 34482c7: cross-libc loading rules | closed (lesson taken in T-0701, T-0702, T-0706) | none: a reference (references/README.md) |
| #172 | [Scope] Provider shapes and bare userlands: Lambda, Cloudflare, Vercel, Nimbus, busybox, distroless profiles | open | T-1659 |
| #173 | [Reference] proot-me/proot @ 65f3f7d: refused: ptrace is filtered | closed (refused; lesson taken in T-0706) | none: a reference (references/README.md) |
| #174 | [Scope] Notebooks: Jupyter and marimo runs, and a session that wipes and keeps one directory | closed | T-1660 |
| #175 | [Reference] QaidVoid/onelf @ 158b4af: launch ladder, exec lock, symlink check | closed (lesson taken in T-0202, T-0304, T-0305, T-0803, T-1001; podcrates pod-enter, pod-extract) | none: a reference (references/README.md) |
| #176 | [Scope] Language runtimes: Node, Deno and Bun permission models, wasmtime and workerd | open | T-1661 |
| #177 | [Reference] qemu-rs/qemu-rs @ 4854135: refused: GPL plugin bindings | closed (refused (GPL); ruled in T-1307) | none: a reference (references/README.md) |
| #178 | [Scope] Network labs: NAT, IPv6, policy and impairment labs inside a guest | open | T-1662 |
| #179 | [Reference] clint/fakeroot (salsa.debian.org) @ 860de25: ownership model and the errno test | closed (lesson taken in T-0704, T-0710, T-0711) | none: a reference (references/README.md) |
| #180 | [Scope] Tunnels and live uplinks: WireGuard kill switch, WARP uplink, proxy-forced programs | open | T-1663 |
| #181 | [Reference] sigoden/fakepty @ a265016: pty trap catalogue for T-1402 | closed (lesson taken in T-1402) | none: a reference (references/README.md) |
| #182 | [Scope] Observability: packet capture, counters and connection logs without CAP_NET_RAW | open | T-1664 |
| #183 | [Reference] talaria0101/dropssh @ 0aafa21: concurrent relay and SSH server | closed (lesson taken by PodUtils/podssh) | none: a reference (references/README.md) |
| #184 | [Scope] Egress broker: a policy proxy and the guest's view of a loopback-only proxy | open | T-1665 |
| #185 | [Reference] talaria0101/nix-experiment @ d8835f2: litmus payload and version forensics | closed (lesson taken in T-0414, T-1111) | none: a reference (references/README.md) |
| #186 | [Scope] Security probe and compare: an inside-the-payload report diffed against a reference | closed | T-1666 |
| #187 | [Reference] talaria0101/sandbox-insights @ 0889f5f: probe discipline and the four walls | closed (lesson taken in T-0415, T-0712, T-1308) | none: a reference (references/README.md) |
| #188 | [Scope] Doctor and installers: host readiness, verified downloads, disk guard | open | T-1667 |
| #189 | [Scope] Findings ledger: tool defects and host traits that change how a result reads | closed | T-1668 |
| #190 | [Reference] talaria0101/sandssh @ 4fc7f8c: one-pair relay design | closed (lesson taken by PodUtils/podssh) | none: a reference (references/README.md) |
| #191 | [Reference] talaria0101/vm-research @ 7697b9b: machine tier and TCG benchmarks | closed (lesson taken in T-1301 to T-1306, T-1308) | none: a reference (references/README.md) |
| #192 | [Reference] ylang-ylang/dockless @ ed35b5d: fail-fast CLI guardrails | closed (lesson taken in T-0801, T-0804) | none: a reference (references/README.md) |
| #193 | [Scope] podvm does what sand-net does, without root: index | open | T-1654 to T-1668 |
| #194 | COORD-001: the crates podvm hands over and takes, and the access of podcrates@T-040 (to podcrates) | open | T-1675 to T-1681 |
| #195 | COORD-002: watch-org.sh sees no repository but the last on Windows: CRLF in the names (to podcrates) | closed (podvm took the pin 54427f6) | none: a COORD issue (to podcrates); podcrates fixed it in `54427f6`, and podvm closes it when it takes that pin |
| #196 | COORD-003: podvm's inventory of the hosts and services of other parties, for pod-services (to podcrates) | closed (podcrates podcrates@T-054 and podcrates@T-066 done; the data of podvm is in pod-services from v0.5.0) | T-1679; the take stays under #194 |
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
| #200 | COORD-006: the Alpine netboot pins: podvm fetches the 3.22.6 archive (to podcrates) | closed | T-1687 |
| #212 | doctor: no row answers whether a memfd can be executed (vm.memfd_noexec), and the memfd rung refuses late | closed | T-1688 |
| #213 | probe: the non-goal `remedy` field contradicts the `stance` in the same JSON object | closed | T-1689 |
| #214 | doctor: the disk-free row reads the hard `RLIMIT_FSIZE` while every other consumer reads the soft one | closed | T-1690 |
| #215 | supervise: `stop -t 0` reports a container as stopped while it is still running, because a bound-setting failure reads as the launcher having gone | closed | T-1691 |
| #216 | windows, parity: the `--podvm-mem` row claims an `RLIMIT_FSIZE` check that neither flavor of the verb performs | closed | T-1692 |
| #217 | release: README and SECURITY put the podbox identity boundary one tag early, and the verifier disagrees with both documents | closed | T-1693 |
| #218 | COORD-012: podvm took v0.7.0, and filed the asks of COORD-015 (to podcrates) | closed | T-1681; none: coordination |
| #219 | COORD-013: podvm took v0.8.0 and pod-supervise (to podcrates) | closed | T-1681 |
| #220 | COORD-014: podvm took v0.9.0, pod-guest and pod-enter (to podcrates) | closed | T-1681 |
| #221 | COORD-015: the whiteout escape is fixed in podvm-extract (T-1698) (to podcrates) | closed (pod-extract carries the tests, podcrates podcrates@T-141; taken 0682c0c) | T-1698 |
| #222 | COORD-016: pod-enter: classify and stage a payload over 128 MiB (T-1647) (to podcrates) | closed | T-1647 |
| #223 | COORD-017: podvm took v0.10.0 and pod-oci (to podcrates) | closed | T-1681 |
| #224 | COORD-018: pod-enter and pod-hostprobe: volume pairs, workdir and first program through the table (T-1653) (to podcrates) | closed | T-1653 |
| #225 | COORD-019: podvm took v0.11.0, pod-extract and pod-rootfs (to podcrates) | closed | T-1681, T-1647 |
| #226 | COORD-020: podvm took v0.12.0, the Maps API of pod-enter (to podcrates) | closed (the asks are podcrates podcrates@T-155) | T-1653 |
| #227 | COORD-021: podrunner: the design review of #47, what the crates must give before podrunner starts (to podcrates) | closed (podcrates#51: podcrates@T-160 to podcrates@T-165) | none: the hand-off of the podrunner plan to podcrates ([decisions](../docs/decisions.md), 2026-11-10, "podrunner") |
| #228 | COORD-022: podrunner: podnet pod with the official runner, and the pod-hold hand-off (to podnet) | closed (podnet#422) | none: the asks of the podrunner plan to podnet ([decisions](../docs/decisions.md), 2026-11-10, "podrunner") |
| #229 | [podrunner] interpose: realpath of /proc/self/exe names the image's loader, while readlink names the program, so the .NET runner exits 154 | closed | T-1704 |
| #230 | [podrunner] interpose: a host path that a loaded object reports gets the rootfs prefix a second time, so CoreCLR exits 137 | closed | T-1705 |
| #231 | [podrunner] the userland rung runs the unmodified actions/runner: it exits 154, then 137, and it dies on SIGXFSZ under a finite RLIMIT_FSIZE | closed | T-1706; T-1655 (amended) |
| #232 | [podrunner] enter: a relative first program resolves against the image root, not against -w, so -w /opt/runner ./run.sh exits 127 | closed | T-1707 |
| #233 | [podrunner] supervise: stop and rm -f signal the payload only, so orphaned grandchildren keep running after the container is gone | closed | T-1708; T-1655 (amended) |
| #234 | [podrunner] image: all containers of one image share one mutable rootfs, so a write in one container is read by another and --rm is not a fresh start | closed | T-1709 |
| #235 | [podrunner] interpose: a Unix socket path in connect and bind passes the path table, so bind creates the socket on the host and a -v pair does not apply | closed | T-1710 |
| #236 | [podrunner] cli: drive the podvm calls of the four container-hook commands on the userland rung, with the runner's mount table | closed | T-1711; T-1653 and T-1655 (amended) |
| #237 | [podrunner] cli: --podvm-tier=chroot does nothing; it runs the ladder, and on a chroot-denied host it enters the userland rung with exit 0 | closed | T-1712 |
| #238 | [podrunner] system: .EnteredRung reads chroot where the probe selects supervise and chroot is denied, while run enters the userland rung | closed | T-1713 |
| #239 | [podrunner] probe: podvm probe --json validates against the published schema of pod-hostprobe (blocked on podcrates podcrates@T-107) | open | T-1714; T-1670 (amended) |
| #240 | [podrunner] supervise: the container table keeps -e values in a file that the umask leaves readable to other users | closed | T-1715; T-1696 (amended, then dropped 2026-11-11) |
| #241 | COORD-023: podvm resumes; fuzz targets for pod-oci, pod-extract and pod-enter; T-1696 is dropped (to podcrates) | closed (podcrates#56: podcrates@T-173) | T-1684 (amended), T-1696 |
| #242 | COORD-024: podvm took v0.15.0 and the pod-enter change of podcrates@T-155 (to podcrates) | closed (podcrates#52 and #54) | T-1716 |
| #243 | COORD-025: the answer to COORD-031: T-1656 moves up for podrunner; incus is a study (T-1717), then an API back end (T-1718) (to podcrates) | closed (podcrates#56) | T-1656 (amended), T-1717, T-1718 |
| #244 | COORD-026: pod-supervise: private modes for the table, the logs and each container directory (T-1715); a tag with podcrates@T-162 for T-1708 (to podcrates) | closed (podcrates v0.16.0, taken 894cf13) | T-1708, T-1715 |
| #245 | COORD-027: pod-enter: resource limits and no_new_privs in the child before the exec (T-1655) (to podcrates) | closed (podcrates v0.17.0, taken 522b437) | T-1655 (amended) |
| #246 | COORD-028: the answer to PodUtils/podrunner#49: wait reads the record again until it has the exit code (to podrunner) | closed (podrunner#49; in v0.2.0-beta.3) | T-1720 |
| #247 | COORD-029: pod-enter: lib_dirs takes /usr/local/lib and the image's loader configuration (T-1719) (to podcrates) | closed | T-1719 |
| #248 | COORD-030: the official runner starts on the userland rung; the shape of M1's stand-in for run.sh (T-1706) (to podrunner) | closed | T-1706 |
| #249 | COORD-031: podvm takes podcrates v0.19.0 (to podcrates) | closed (podcrates#62) | T-1719 |
| #250 | COORD-032: the answer to PodUtils/podrunner#51: a unix socket path takes the table on main (T-1710) (to podrunner) | closed (podrunner#51) | T-1710 |
| #251 | [reference] Azathothas/errand-box: incus without root (route A) and its costs | closed (lesson taken in T-1717 and T-1718) | T-1717, T-1718 (open while T-1718 is open) |
| #252 | COORD-033: the answer to PodUtils/podrunner#52: no aarch64 interposer yet; podvm T-1721 (to podrunner) | closed | T-1721 |
| #253 | COORD-034: a client of the incus REST API as a podcrates crate (T-1718) (to podcrates) | closed (pod-incus, taken 1ffe87e) | T-1718 |
| #254 | COORD-035: the answer to PodUtils/podrunner#53: podvm v0.2.0-beta.3 is released (to podrunner) | closed (podrunner#53) | T-1706, T-1720, T-1710 |
| #255 | COORD-036: the answer to PodUtils/podrunner#61: an absolute symlink of the rootfs resolves on the host; podvm T-1725 fixes it (to podrunner) | closed | T-1725 |
| #256 | COORD-037: the answer to PodUtils/podrunner#62: exec ignores a :ro volume; podvm T-1726 fixes it (to podrunner) | closed | T-1726 |
| #257 | COORD-038: podvm takes podcrates v0.21.0 and pod-incus (the answer to PodUtils/podcrates#63) (to podcrates) | closed (podcrates#63 and #65) | T-1727 |
| #258 | COORD-039: each container has its own rootfs (T-1709, issue 234) (to podrunner) | closed | T-1709 |
| #259 | COORD-040: ask for pod-confine and a confine step of pod-enter's child (podvm T-1658, issue 170) (to podcrates) | closed | T-1658 |
| #260 | COORD-041: ask for pod-netlab, and who reads the guest's modloop (podvm T-1662, issues 178, 180, 182) (to podcrates) | open | T-1662 |
| #261 | COORD-042: pod-extract refuses a layer whose root directory entry has an empty name (public.ecr.aws/lambda/provided:al2023) (to podcrates) | closed | T-1659 |
| #262 | COORD-043: podvm takes podcrates v0.22.0; a detached payload ends with its launcher (the answer to PodUtils/podcrates#71) (to all) | closed (podcrates#71) | T-1732 |
| #263 | COORD-044: three gaps of podvm's inventory in pod-services: gcr.io, mirror.gcr.io and the NIC modules of the guest (podvm T-1679) (to podcrates) | closed | T-1679 |
| #264 | COORD-045: podvm takes podcrates v0.24.0; pod-services is taken (the answer to PodUtils/podcrates#69 and #70) (to podcrates) | closed | T-1658, T-1659, T-1679 |
| #265 | COORD-046: podvm v0.2.0-beta.4 is released: a tree for each container, absolute links, exec under :ro (the answer to PodUtils/podrunner#62) (to podrunner) | closed | T-1709, T-1725, T-1726, T-1732 |
| #266 | COORD-047: podvm takes podcrates v0.25.0 (the answer to PodUtils/podcrates#76) (to podcrates) | closed | T-1658 |
| #267 | COORD-048: Plan.confine and the /tmp that the namespace rung mounts in the child: the rule holds the directory under the mount (to podcrates) | closed | T-1658 |
| #268 | COORD-049: real labels and one JSON report for each run (A-V-07, A-V-09 and G-06 of PodUtils/podcrates#39) (to all) | closed | T-1695 |
| #269 | COORD-050: mcr.microsoft.com for pod-services (the image of podvm's profile playwright-chromium) (to podcrates) | closed | T-1694 |
| #270 | COORD-051: podvm takes podcrates v0.26.0 (the answer to PodUtils/podcrates#77) (to podcrates) | closed | T-1658, T-1694 |
| #271 | cfb asks: tracker for the 9 asks of cfb owned by podvm (from podcrates#35) | open | tracker: T-1658, T-1665, T-1663, T-1694, T-1695, T-1714, T-1723, T-1739 |
| #272 | CI plan items: tracker for the 3 items owned by podvm (from podcrates#47) | open | tracker: T-1656, T-1723 (P3-3); #271 (P1-1) |
| #273 | COORD-052: podvm v0.2.0-beta.5 is released: confine for run -d, exec and the loader family; labels and run --report; headless Chromium (to all) | closed | T-1658, T-1694, T-1695, T-1711, T-1738 |
| #274 | COORD-053: registry.npmjs.org (workerd) for pod-services (to podcrates) | closed | T-1661 |
| #275 | COORD-054: the wasmtime release for pod-services (to podcrates) | closed | T-1661 |
| #276 | COORD-055: podvm takes podcrates v0.27.0 (the answer to PodUtils/podcrates#85) (to podcrates) | closed | T-1658 |
| #277 | COORD-056: pod-guest commit_argv: qemu-img commit takes no OUTFILE; podvm windows setup fails under KVM (to podcrates) | closed | T-1641 |
| #278 | COORD-057: podvm takes podcrates v0.28.0 (the answer to PodUtils/podcrates#86) (to podcrates) | closed | T-1641, T-1661 |
| #279 | COORD-058: podvm takes podcrates v0.29.0; the confine help; aarch64 interposer in progress (to all) | closed | T-1658, T-1721 |
| #280 | COORD-059: podvm takes podcrates v0.30.0 (the answer to PodUtils/podcrates#89) (to podcrates) | closed | T-1680 |
| #281 | COORD-060: pod-extract refuses a correct plain-tar layer: the hash stops at the end-of-archive marker (to podcrates) | closed | T-1741 |
| #282 | COORD-061: pod-enter libc_beside does not find the aarch64 glibc of a Debian image (the search is depth-first from the end) (to podcrates) | closed | T-1742 |
| #283 | COORD-062: the hand-off of pod-relay for podvm (blocking facade, reverse-v1) (to podcrates) | closed | T-1678 |
| #284 | COORD-063: podvm takes podcrates v0.31.0 (the answer to PodUtils/podcrates#90) (to podcrates) | closed | T-1741, T-1742 |
| #285 | COORD-064: podvm takes podcrates v0.32.0 and the trial of pod-relay (the answer to PodUtils/podcrates#93) (to podcrates) | closed | T-1678 |
| #286 | COORD-065: podvm v0.2.0-beta.6 carries the aarch64 interposer (for PodUtils/podrunner#52) (to podrunner) | closed | T-1721 |
| #287 | [gh action] requested | closed (gate 43345851281, secrets 43345855762, arm 43347185943, trial 43347752049, release 43348676353, all green) | none: a request of the gate, `arm.yml`, the nightly trial and the release `0.2.0-beta.7` on `2edace91` (AGENTS.md, section 9) |
| #288 | COORD-066: podvm takes podcrates v0.34.0 (the answer to PodUtils/podcrates#103 and #104) (to podcrates) | closed | T-1662 |
| #289 | COORD-067: GNU tar with an absolute -C works where chroot is denied: the fortified opens take the path table (the answer to PodUtils/podcrates#102 and PodUtils/podrunner#70) (to podcrates) (to podrunner) | closed | T-1743 |
| #290 | COORD-068: podvm's answer to the asks V-1 to V-8 of podswarm (PodUtils/podcrates#124): entries T-1745 to T-1752 (to podcrates) | open | T-1745, T-1746, T-1747, T-1748, T-1749, T-1750, T-1751, T-1752 |
| #291 | COORD-069: ask for a reader of the probe report in pod-hostprobe, so that run can use the probe cache (podvm T-1746, V-2 of podswarm) (to podcrates) | open | T-1746 |
| #292 | [gh action] requested | closed (gate 43351811176, secrets 43351815639, arm 43351820031, all green on 845faba1) | none: a request of the gate, secrets and `arm.yml` on the head of `main` of 2026-11-13 (the commit that adds this row), for T-1662, T-1744, T-1748 and podcrates v0.35.0 (AGENTS.md, section 9) |
| #293 | COORD-070: podvm takes podcrates v0.35.0 (the answer to PodUtils/podcrates#127) (to podcrates) | closed | T-1662, T-1746 |
| #294 | COORD-071: ask for an overlay root in pod-enter's namespace rung: the image read-only under a tmpfs upper layer (podvm T-1749, V-5 of podswarm) (to podcrates) | open | T-1749 |
| #295 | COORD-072: podvm takes podcrates v0.36.0 (the answer to PodUtils/podcrates#129) (to podcrates) | closed | T-1746 |
| #296 | COORD-073: ask for a proc mount on the /proc of the image in pod-enter's namespace rung (podvm T-1758) (to podcrates) | closed | T-1758 |
| #297 | COORD-074: ask for an entry of pod-enter that joins the mount namespace of a running payload, for exec (podvm T-1765) (to podcrates) | closed | T-1765 |
| #298 | COORD-075: ask for three fixes of the knob flags of pod-netlab: an empty --udp, the range of a number, the words of a flag (podvm T-1770) (to podcrates) | closed | T-1770 |
| #299 | COORD-076: podvm takes podcrates v0.37.0 (the answer to PodUtils/podcrates#134) (to podcrates) | closed | T-1758, T-1765 |
| #300 | [gh action] requested | closed (gate 43384381442 and secrets 43384387248 green on 7f1b23b6; arm 43386208714 failed: experiment 356 asked for refusals that T-1761 and T-1772 removed) | none: a request of the gate, `arm.yml`, the nightly trial and the release `0.2.0-beta.8` (AGENTS.md, section 9) |
| #301 | [gh action] requested | closed (secrets 43389382246 green on 2dde7df0; the gate 43389376336 was cancelled at the bound of a job beside a second dispatch of it) | none: a request of the gate, `arm.yml`, the nightly trial and the release `0.2.0-beta.8` (AGENTS.md, section 9) |
| #302 | [gh action] requested | closed (gate 43394989985, arm 43397131676, trial 43398052471, release 43399714037, all green on 2dde7df0) | none: a request of the gate, `arm.yml`, the nightly trial and the release `0.2.0-beta.8` (AGENTS.md, section 9) |
| #303 | COORD-077: the owners in a guest of the machine tier, and the guest with no NIC and one host channel (the answer to PodUtils/podrunner#81 and #83) (to podrunner) | open | T-1776, T-1751 |
| #304 | COORD-078: podvm v0.2.0-beta.8 is released, and podvm takes podcrates v0.38.0 (the answer to PodUtils/podcrates#136) (to all) | closed | T-1760, T-1770 |
| #305 | [gh action] requested | closed (not run to its end: ce4a561c carried a symbolic link of the cloud host, crates/podvm-interpose/target, which the next commit removes) | none: a request of the gate, `arm.yml`, the nightly trial and the release `0.2.0-beta.9` (AGENTS.md, section 9) |
| #306 | [gh action] requested | closed (gate 43403642262, arm 43406008482, trial 43406658216, release 43409438385, all green on 11008d62) | none: a request of the gate, `arm.yml`, the nightly trial and the release `0.2.0-beta.9` (AGENTS.md, section 9) |
| #307 | COORD-079: stdin for an incus instance that stays (the answer to PodUtils/podrunner#85) (to podrunner) | open | T-1777 |
| #308 | [gh action] requested | closed (gate 43420276209, arm 43421607391, trial 43422400615, release 43423493082, all green on ce56cc26) | none: a request of the gate, `arm.yml`, the nightly trial and the release `0.2.0-beta.10` (AGENTS.md, section 9) |
| #309 | COORD-080: podvm takes podcrates v0.39.0 (the answer to PodUtils/podcrates#137) (to podcrates) | open | - |
| #310 | [gh action] requested | closed (the gate 43437287427 failed on 11c0d410: the native-static rung refused the device pair /dev/null of T-1774) | none: a request of the gate, `arm.yml`, the nightly trial and the release `0.2.0-beta.11` (AGENTS.md, section 9) |
| #311 | COORD-081: the channel of a guest and a loopback proxy variable (the answer to PodUtils/podrunner#87) (to podrunner) | open | T-1778 |
