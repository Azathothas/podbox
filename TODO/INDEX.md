# TODO

Every entry, one line each, sorted by id. The entry itself lives in the
`TODO/<category>.md` the row links to, and it closes there with its own
acceptance command, actually run, with the output recorded underneath.

**What to work on next is not here.** [PROGRESS.md](PROGRESS.md) carries the
work order and is the only place that has one. This file carries the list, definitions, and derived counts.

[RULES.md](RULES.md) is how this repository is worked on.
[ROADMAP.md](ROADMAP.md) says where the milestones are: each is an entry.
[reference-map.md](reference-map.md) is the corpus and its licence
determinations.
[issues.md](issues.md) maps each GitHub issue of the record to its
entries, and `TODO/github.txt` lists the numbers it holds.

`crates/podvm-gate/src/main.rs` checks this file against the entries: a status that
disagrees, a row with no entry, an entry with no row, a count that does not add
up, a missing field, a `Prove` that is not a command, a reference that does not
resolve, a dead link, and a cited path or line that does not exist.

```sh
./target/x86_64-unknown-linux-musl/release/podvm-gate
```

## Priority

- **P0** breaks correctness, loses data, or takes the process down.
- **P1** a documented capability does not work, or a flag does nothing.
- **P2** worth doing, nothing is wrong without it.
- **P3** worth recording so it is not rediscovered.

## Effort

Effort is an estimate of scope: S is small, M is medium, L is large,
and XL requires a scope review. It is not a measured duration.

## Status

Status values are **open**, **partial**, **blocked**, and **done**.
See [entry closure](RULES.md#5-entry-closure) for their use.

## Categories

| category | crate | specification |
| --- | --- | --- |
| [probe](probe.md) | `crates/podvm-probe` | `TOOL.md` section 6.1 |
| [image](image.md) | `crates/podvm-image` | `TOOL.md` section 6.2 |
| [extract](extract.md) | `crates/podvm-extract` | `TOOL.md` section 6.3 |
| [complete](complete.md) | `crates/podvm-complete` | `TOOL.md` section 6.4 |
| [enter](enter.md) | `crates/podvm-enter` | `TOOL.md` section 6.5 |
| [supervise](supervise.md) | `crates/podvm-supervise` | `TOOL.md` section 6.6 |
| [interpose](interpose.md) | `crates/podvm-interpose` | `TOOL.md` section 6.7 |
| [cli](cli.md) | `crates/podvm` | `TOOL.md` section 6.8, section 6.9 |
| [deps](deps.md) | the whole tree | `TOOL.md` section 3.5 |
| [packaging](packaging.md) | the artefact | `TOOL.md` section 3.4, section 5 M7 |
| [milestones](milestones.md) | the gates | `TOOL.md` section 5 |
| [machine](machine.md) | the machine tier | `https://github.com/talaria0101/vm-research`, its podvm-spec document |
| [gate](gate.md) | `scripts/` | `docs/methodology/gate.md` |
| [podssh](podssh.md) | proposed SSH crate | the SSH work in pull requests 66 and 67 |
| [net](net.md) | proposed: podcrates `pod-netlab`, `pod-pcap`, `pod-egress` (the roads), `pod-broker`, `pod-netstack` | issues 178, 180, 182 and 184 of the widened scope, index issue 193 |

## Entries

| ID | Priority | Category | Status | Item |
| --- | --- | --- | --- | --- |
| [T-0101](probe.md) | P0 | M | none | probe | done | The probe set, one disposable child per probe, errno not boolean |
| [T-0102](probe.md) | P0 | S | none | probe | done | Separate a filtered syscall from an executed one with a bogus argument |
| [T-0103](probe.md) | P1 | S | none | probe | done | Probe creation and attachment separately |
| [T-0104](probe.md) | P0 | S | none | probe | done | Probe the write allowlist by writing, for blocks and inodes |
| [T-0105](probe.md) | P1 | S | none | probe | done | Read the ID maps directly rather than inferring them |
| [T-0106](probe.md) | P2 | S | none | probe | done | The `mknod` pair, because one of them tests nothing |
| [T-0107](probe.md) | P0 | M | none | probe | done | Mode selection, and one switch that turns every degradation into a refusal |
| [T-0108](probe.md) | P0 | S | none | probe | done | The mode banner |
| [T-0109](probe.md) | P0 | S | none | probe | done | A verdict is the operation's, and "could not run" never reads as "denied" |
| [T-0110](probe.md) | P1 | S | none | probe | done | `podvm probe` exit-code and channel contract |
| [T-0111](probe.md) | P2 | S | none | probe | done | Cache the probe result, and key it on what actually decides it |
| [T-0112](probe.md) | P1 | M | none | probe | done | A virtual machine answers the two questions this host's kernel cannot |
| [T-0113](probe.md) | P1 | S | none | probe | done | Two probes in one process took each other's scratch name |
| [T-0201](image.md) | P0 | M | none | image | done | Registry client, HTTPS only, with no plain-HTTP fallback |
| [T-0202](image.md) | P0 | M | none | image | done | A content-addressed store, and digest parity with docker |
| [T-0203](image.md) | P0 | S | none | image | done | Check `statvfs` for blocks and inodes, and name the destination |
| [T-0204](image.md) | P1 | M | none | image | done | `images`, `rmi`, `tag`, and a store GC that cannot delete a running container's rootfs |
| [T-0205](image.md) | P3 | S | none | image | done | Re-test podman with `vfs` and `ignore_chown_errors` before repeating "no path exists" |
| [T-0206](image.md) | P3 | L | none | image | done | A registry fixture, so the acceptance stops depending on somebody else's quota |
| [T-0207](image.md) | P2 | L | none | image | done | Fetch layers with bounded concurrency, and measure what it buys |
| [T-0208](image.md) | P2 | L | none | image | done | `--platform`, and a store that can hold two variants of one tag |
| [T-0209](image.md) | P2 | L | none | image | done | Registry authentication, without a credential ever entering this tree |
| [T-0210](image.md) | P1 | L | none | image | done | The store's concurrency contract, written down and driven |
| [T-0211](image.md) | P1 | S | none | image | done | An image lock outlives its holder whenever anything forks |
| [T-0212](image.md) | P0 | M | none | image | done | The platform is decided at run time, and the store holds more than one |
| [T-0213](image.md) | P0 | M | none | image | done | A registry with no certificate, or one nothing trusts, and the refusal kept |
| [T-0214](image.md) | P2 | S | none | image | done | A blob body cut off mid-stream is not retried, and the bounded retry is around the wrong thing |
| [T-0215](image.md) | P0 | M | none | image | done | Four lock tests fail in two runs of five, and the gate has never said so |
| [T-0301](extract.md) | P0 | M | none | extract | done | Extract in-process, at entry level, never through system `tar` |
| [T-0302](extract.md) | P0 | M | none | extract | done | Ownership-neutral extraction plus the sidecar |
| [T-0303](extract.md) | P0 | S | none | extract | done | Whiteouts are matched on the basename, never with a path glob |
| [T-0304](extract.md) | P0 | M | none | extract | done | Refuse an entry that resolves outside the destination, including through a symlink from the same layer |
| [T-0305](extract.md) | P1 | S | none | extract | done | An absolute symlink target is rootfs-relative, not a refusal |
| [T-0306](extract.md) | P1 | S | none | extract | done | Re-permission between layers, or the second layer fails |
| [T-0307](extract.md) | P1 | S | none | extract | done | Hard links, symlinks and the layer order |
| [T-0401](complete.md) | P0 | S | none | complete | done | Device shims as regular files |
| [T-0402](complete.md) | P0 | S | none | complete | done | Always install the host's `/etc/resolv.conf` |
| [T-0403](complete.md) | P2 | S | none | complete | done | `/etc/hosts` |
| [T-0404](complete.md) | P0 | M | none | complete | done | Synthesize `/etc/passwd` and `/etc/group`, and let the payload's writes persist |
| [T-0405](complete.md) | P1 | S | none | complete | done | `/etc/mtab` is a symlink, and writing through it escapes the rootfs |
| [T-0406](complete.md) | P1 | S | none | complete | done | pacman: `DownloadUser` and the keyring |
| [T-0407](complete.md) | P1 | S | none | complete | done | apt: the sandbox user, https sources and the CA bundle |
| [T-0408](complete.md) | P2 | S | none | complete | done | zypper: fix the RIS index, not `repos.d` |
| [T-0409](complete.md) | P2 | S | none | complete | done | Ownership failures from `dpkg`, `rpm` and `xbps` are warnings |
| [T-0410](complete.md) | P0 | S | none | complete | done | Supply `/etc/nsswitch.conf`, or the supplied `/etc/passwd` is a no-op |
| [T-0411](complete.md) | P1 | M | none | complete | done | A payload whose package sources are `http://`, on a runtime where tcp/80 hangs |
| [T-0412](complete.md) | P1 | M | none | complete | done | A fixup that has to run INSIDE the rootfs, and podvm runs it from outside |
| [T-0413](complete.md) | P1 | M | none | complete | done | No `/proc` inside a chroot, and the shell feature that quietly stops working |
| [T-0414](complete.md) | P1 | M | none | complete | done | Two walls only one instance of the class has shown, and podvm has probed neither |
| [T-0415](complete.md) | P1 | S | none | complete | done | A device stand-in is checked by type, because an absent one becomes a growing file |
| [T-0501](enter.md) | P0 | M | none | enter | done | Open every descriptor before the root changes |
| [T-0502](enter.md) | P0 | S | none | enter | done | Resolve the program inside the new root, in the process that changed it |
| [T-0503](enter.md) | P1 | S | none | enter | done | Probe `/dev/ptmx`, and refuse `-t` by name where it is absent |
| [T-0504](enter.md) | P1 | S | none | enter | done | Refuse a rootfs path that is a symlink |
| [T-0505](enter.md) | P1 | S | none | enter | done | `exec` is a fresh chroot, and `inspect` says so |
| [T-0506](enter.md) | P0 | M | none | enter | done | A foreign-architecture container, and never a rung measured by the emulator |
| [T-0601](supervise.md) | P0 | M | none | supervise | done | One pidfd per direct child, `waitid` for status |
| [T-0602](supervise.md) | P0 | M | none | supervise | done | Never decide "running" by sleeping and looking |
| [T-0603](supervise.md) | P1 | S | none | supervise | done | `PR_SET_PDEATHSIG` fires on the creating thread's exit |
| [T-0604](supervise.md) | P1 | M | none | supervise | done | Running state is launcher state |
| [T-0605](supervise.md) | P2 | S | none | supervise | done | Capture logs at spawn, from the descriptors opened in step 2 |
| [T-0606](supervise.md) | P0 | L | none | supervise | done | The notification tier: probe three legs, refuse the tier, never fall back per call |
| [T-0607](supervise.md) | P0 | M | none | supervise | done | The lifecycle, twenty times, twenty passes |
| [T-0608](supervise.md) | P1 | M | none | supervise | done | A detached container that reads `exited` with no launcher, seen twice and not reproduced |
| [T-0701](interpose.md) | P0 | M | none | interpose | done | The cdylib build constraints |
| [T-0702](interpose.md) | P0 | M | none | interpose | done | One object per libc, and it must live inside the rootfs |
| [T-0703](interpose.md) | P0 | L | none | interpose | done | Path virtualization: the entry-point set and `*at` resolution |
| [T-0704](interpose.md) | P0 | L | none | interpose | done | Ownership virtualization: the half a path interposer does not have |
| [T-0705](interpose.md) | P1 | M | none | interpose | done | Reverse mapping, so the payload reads back what it wrote |
| [T-0706](interpose.md) | P0 | S | none | interpose | done | Classify the payload and decline with a named reason |
| [T-0707](interpose.md) | P1 | S | none | interpose | done | The paths that must not be rewritten |
| [T-0708](interpose.md) | P2 | M | none | interpose | done | Intercept the operations the runtime cannot provide |
| [T-0709](interpose.md) | P0 | M | none | interpose | done | Select the interposer by `DT_NEEDED`, and refuse on the version predicate |
| [T-0710](interpose.md) | P1 | L | none | interpose | done | The ownership memo lives where the payload can edit it, and answers by linear scan |
| [T-0711](interpose.md) | P1 | L | none | interpose | done | The identity calls, and podvm's honesty rules point the other way from fakeroot's |
| [T-0712](interpose.md) | P2 | S | none | interpose | done | A reach matrix holds its arguments constant, or it measures two things |
| [T-0801](cli.md) | P0 | L | none | cli | done | The verb and flag parity table |
| [T-0802](cli.md) | P0 | S | none | cli | done | docker's exit codes, unaltered |
| [T-0803](cli.md) | P1 | S | none | cli | done | Answer to `docker` and `podman` on PATH |
| [T-0804](cli.md) | P0 | M | none | cli | done | The honesty rules, and one switch that makes every degradation fatal |
| [T-0805](cli.md) | P1 | M | none | cli | done | Diagnostics that name the operation, the errno, the mechanism and the remedy |
| [T-0806](cli.md) | P0 | S | none | cli | done | Never prompt, never wait unbounded, and check space before every large write |
| [T-0807](cli.md) | P2 | S | none | cli | done | `podvm images --format` refuses a template no verb can answer, before it looks at the store |
| [T-0808](cli.md) | P1 | L | none | cli | done | Drive every row of the parity table through the shipped binary |
| [T-0809](cli.md) | P1 | M | none | cli | done | The spawn that fails with the wrong reason, and the one field that fixes it |
| [T-0810](cli.md) | P1 | S | none | cli | done | `create` fails storing the memo: the container directory is never made |
| [T-0901](deps.md) | P1 | M | none | deps | done | Sweep: syscalls |
| [T-0902](deps.md) | P2 | S | none | deps | done | Sweep: seccomp BPF |
| [T-0903](deps.md) | P3 | S | none | deps | done | Sweep: Landlock |
| [T-0904](deps.md) | P1 | M | none | deps | done | Sweep: OCI registry client and types |
| [T-0905](deps.md) | P0 | S | none | deps | done | Sweep: TLS |
| [T-0906](deps.md) | P1 | S | none | deps | done | Sweep: HTTP |
| [T-0907](deps.md) | P0 | M | none | deps | done | Sweep: tar, gzip, zstd |
| [T-0908](deps.md) | P1 | M | none | deps | done | Sweep: digests, JSON, argument parsing, ELF |
| [T-0909](deps.md) | P1 | M | none | deps | done | Vendor the memfd and userland-exec rungs, and fix the fork's regression here |
| [T-0910](deps.md) | P0 | S | none | deps | done | The `cargo bloat` baseline, committed, and checked at the gate |
| [T-0911](deps.md) | P0 | M | none | deps | done | The syscall table and the kernel structs come from a crate, per architecture |
| [T-0912](deps.md) | P2 | M | none | deps | done | The powerpc gate is the crate's and it is stale, so podvm can clear it |
| [T-1001](packaging.md) | P0 | S | none | packaging | done | A single static binary with no `PT_INTERP` |
| [T-1002](packaging.md) | P1 | M | none | packaging | done | Embed the interposer as bytes and place it inside the rootfs |
| [T-1003](packaging.md) | P2 | L | none | packaging | done | The launch ladder, and a single file with an embedded rootfs |
| [T-1004](packaging.md) | P3 | M | none | packaging | done | A reproducible build, and the artefact's own inputs recorded |
| [T-1005](packaging.md) | P1 | S | none | packaging | done | A session reaches the code in one command, and the build runs behind the reading |
| [T-1100](milestones.md) | P0 | L | none | milestones | done | M-1 the corpus, the work index and the skeleton |
| [T-1101](milestones.md) | P0 | M | none | milestones | done | M0 the probe, and nothing else |
| [T-1102](milestones.md) | P1 | M | none | milestones | done | M1 image acquisition |
| [T-1103](milestones.md) | P0 | L | none | milestones | done | M2 extraction that survives the ownership wall |
| [T-1104](milestones.md) | P0 | M | none | milestones | done | M3 `run` on the chroot rung |
| [T-1105](milestones.md) | P0 | L | none | milestones | done | M4 the lifecycle, twenty times |
| [T-1106](milestones.md) | P1 | L | none | milestones | done | M5 environment completion, ten distributions |
| [T-1107](milestones.md) | P1 | L | none | milestones | done | M6 the interposer |
| [T-1108](milestones.md) | P2 | M | none | milestones | done | M7 packaging |
| [T-1109](milestones.md) | P1 | M | none | milestones | done | The negative tests, which are tests |
| [T-1110](milestones.md) | P0 | L | none | milestones | done | M6's acceptance: a payload the interposer is the only reason works |
| [T-1111](milestones.md) | P1 | L | none | milestones | done | M8 the nix acceptance: a real payload the chroot tier is exactly the answer for |
| [T-1112](milestones.md) | P1 | L | none | milestones | done | A disposable guest that is not Linux |
| [T-1201](gate.md) | P0 | S | none | gate | done | The gate reaches every file this project wrote |
| [T-1202](gate.md) | P0 | M | none | gate | done | Every check is planted against, and a plant that stops reaching its subject says so |
| [T-1203](gate.md) | P1 | M | none | gate | done | A measurement taken on one host is a property of that host |
| [T-1204](gate.md) | P1 | S | none | gate | done | Check 17 holds the newest committed reading under the ceiling, not only the baseline |
| [T-1205](gate.md) | P1 | S | none | gate | done | The gate holds experiment numbers unique, because four Proves already collide |
| [T-1206](gate.md) | P0 | M | none | gate | done | CI installs the toolchain the build config names, and nine commits proved nobody was holding it |
| [T-1207](gate.md) | P1 | L | none | gate | done | The one crate that runs inside other people's processes is the one the gate does not check |
| [T-1208](gate.md) | P1 | M | none | gate | done | A closed entry carries its recorded run, and the gate can see it |
| [T-1209](gate.md) | P1 | M | none | gate | done | Thirty-nine `Prove` lines pull from the one registry the acceptance may not use |
| [T-1210](gate.md) | P1 | M | none | gate | done | Convert the interpose engine scripts to `experiments/lib/engine.sh` |
| [T-1211](gate.md) | P1 | M | none | gate | done | Convert the distribution and probe engine scripts to `experiments/lib/engine.sh` |
| [T-1212](gate.md) | P1 | M | none | gate | done | Convert the image, registry and CLI engine scripts to `experiments/lib/engine.sh` |
| [T-1213](gate.md) | P1 | M | none | gate | done | Convert the target-image pair and its probe consumer to `experiments/lib/engine.sh` |
| [T-1301](machine.md) | P0 | M | none | machine | done | The machine tier is probed leg by leg, and a present file is not a working one |
| [T-1302](machine.md) | P1 | M | none | machine | done | One binary, one parity table, and a VM-only flag that cannot collide |
| [T-1303](machine.md) | P1 | L | none | machine | done | The image is a rootfs directory, and an initramfs with no console is a silent machine |
| [T-1304](machine.md) | P1 | L | none | machine | done | The exec protocol is a serial pair with a nonce, and every wait is bounded |
| [T-1305](machine.md) | P2 | L | none | machine | done | The fleet and the fork, and the file-size ceiling that bounds both |
| [T-1306](machine.md) | P1 | M | none | machine | done | The non-goals are refusals the code makes, not notes in a document |
| [T-1307](machine.md) | P2 | S | none | machine | done | The five Rust VM tools, ruled one by one, so nobody surveys them again |
| [T-1308](machine.md) | P1 | M | none | machine | done | One TCG number is a claim about one benchmark, and the range is 3x to 21x |
| [T-1309](interpose.md) | P1 | M | none | interpose | done | libdnf repodata downloads fail under the preloaded interposer |
| [T-1310](image.md) | P1 | S | none | image | done | The store suite exhausts the sixteen fork-shed slots, and the victim varies |
| [T-1311](interpose.md) | P1 | S | none | interpose | done | The interposed `fchmodat` drops the `flags` argument |
| [T-1312](interpose.md) | P1 | S | none | interpose | done | The glibc interposer needs newer symbols than the payload provides |
| [T-1313](machine.md) | P1 | M | none | machine | done | Every experiment fetch carries its own ceiling, and a stalled origin proves it |
| [T-1314](packaging.md) | P1 | L | none | packaging | done | Nightly releases: one tag builds and tests every supported arch |
| [T-1315](extract.md) | P1 | S | none | extract | done | Extract re-verifies the blob digest against the manifest |
| [T-1316](deps.md) | P2 | S | none | deps | done | The committed lock matches the manifests, so a clean build leaves a clean tree |
| [T-1317](enter.md) | P1 | M | none | enter | done | A chroot-denied host gets an up-front refusal naming chroot, not a 125 after the work |
| [T-1318](supervise.md) | P3 | S | none | supervise | done | `logs -f` follows a container's log, bounded like the rest |
| [T-1319](cli.md) | P1 | S | none | cli | done | `inspect` prints exactly one document for any reference |
| [T-1320](image.md) | P2 | M | none | image | done | Images move without a registry: save, load, import |
| [T-1321](image.md) | P2 | M | none | image | done | A store-health verb and a pull-provenance record |
| [T-1322](image.md) | P1 | M | none | image | done | A container record gates every removal of the image it references |
| [T-1323](cli.md) | P2 | M | none | cli | done | `cp` reaches an image's extracted rootfs, and copies directories with the same checks |
| [T-1324](complete.md) | P2 | S | none | complete | done | The README tells the truth: build order, rung map, status, auth scope |
| [T-1325](gate.md) | P1 | M | none | gate | done | The gate checks what entries claim: reachable flags and true parity notes |
| [T-1326](gate.md) | P1 | S | none | gate | done | `check-markers.sh` builds its marker bytes portably across `/bin/sh` |
| [T-1327](interpose.md) | P2 | L | none | interpose | done | The interposer builds per architecture, and the T-0704 citations point at it |
| [T-1328](packaging.md) | P2 | M | none | packaging | done | The nightly signs its artefacts, with provenance a downloader can check |
| [T-1329](packaging.md) | P2 | M | none | packaging | done | The per-arch smoke pulls and extracts, not just versions |
| [T-1330](cli.md) | P2 | S | none | cli | done | Bundled short flags parse as docker reads them |
| [T-1331](cli.md) | P2 | M | none | cli | done | `--filter`, `restart` and `pull -a/-q` answer the docker idiom |
| [T-1332](cli.md) | P1 | M | none | cli | done | `podvm man` generates the manual from the binary, pager-aware |
| [T-1333](probe.md) | P1 | S | none | probe | done | The probe cache key carries the capability set |
| [T-1334](packaging.md) | P1 | S | none | packaging | done | The release carries its build commit's gate state and its reproducibility boundary |
| [T-1335](supervise.md) | P3 | S | none | supervise | done | The launcher forwards its own shutdown signals to the payload |
| [T-1336](cli.md) | P1 | M | none | cli | done | CLI output is plain ASCII: no emoji, no markers, on every path |
| [T-1337](cli.md) | P2 | M | none | cli | done | Doctor, disk usage, and log tail for operators |
| [T-1338](gate.md) | P2 | L | none | gate | done | A performance harness with baselines and a regression gate |
| [T-1339](enter.md) | P1 | L | none | enter | done | Enter the namespace rung where the probe permits it |
| [T-1340](gate.md) | P3 | M | none | gate | done | Isolate why early payload runs cost seconds and late ones do not |
| [T-1341](gate.md) | P3 | S | none | gate | done | Budget the warm run cost beside the cold one |
| [T-1342](image.md) | P2 | M | none | image | done | A pull refuses past the ceiling instead of dying past it |
| [T-1343](gate.md) | P1 | M | none | gate | done | The Windows lane uses toolkit 6 job inputs and checks every retained job |
| [T-1344](gate.md) | P1 | S | none | gate | done | Experiment jobs find the checkout after the Windows input change |
| [T-1345](gate.md) | P1 | S | none | gate | done | The Windows job wrapper runs a Bash caller with Bash |
| [T-1346](gate.md) | P1 | S | none | gate | done | The plant script finds the checkout from a Windows job input |
| [T-1347](gate.md) | P1 | S | none | gate | done | Check parity notes against the milestone entries |
| [T-1348](gate.md) | P1 | L | none | gate | done | Check current documents against source declarations |
| [T-1349](gate.md) | P1 | M | none | gate | done | Verify build freshness from input and output bytes |
| [T-1350](gate.md) | P1 | M | none | gate | partial | Make Windows proofs use explicit inputs and return build artifacts |
| [T-1351](gate.md) | P1 | S | none | gate | done | Keep the failed check's complete diagnostic |
| [T-1401](podssh.md) | P1 | L | none | podssh | done | Prove the SSH transport and server in the current tree |

| [T-1402](podssh.md) | P1 | L | none | podssh | done | Provide an interactive session without a pty |
| [T-1403](podssh.md) | P1 | L | none | podssh | done | Prove concurrent sessions on one relay connection |
| [T-1404](podssh.md) | P2 | L | none | podssh | done | Add the remote and machine SSH verbs after the transport holds |
| [T-1405](podssh.md) | P1 | M | none | podssh | done | Ship and smoke the SSH helper archive |
| [T-1406](podssh.md) | P1 | L | none | podssh | done | Bound SSH DNS and writes and prove node reconnect |
| [T-1407](enter.md) | P0 | L | none | enter | done | Anchor userland paths inside the rootfs or refuse |
| [T-1408](enter.md) | P1 | S | none | enter | done | Resolve the loader symlink inside the rootfs |
| [T-1409](enter.md) | P1 | M | none | enter | done | Answer the payload-visible exe on the loader family |
| [T-1410](enter.md) | P1 | L | none | enter | done | Exec through the no-chroot family |
| [T-1411](enter.md) | P1 | L | none | enter | done | A detached launcher for the no-chroot family |
| [T-1412](enter.md) | P1 | S | none | enter | done | A missing workdir refuses naming the path |
| [T-1413](enter.md) | P2 | S | none | enter | done | A script refusal names the file and its shebang |
| [T-1414](cli.md) | P1 | S | none | cli | done | The no-chroot path returns 127 and 126 |
| [T-1415](cli.md) | P1 | M | none | cli | done | Strict classifies degradations on the userland rung |
| [T-1416](cli.md) | P2 | S | none | cli | done | Quiet runs and a verbose banner |
| [T-1417](cli.md) | P2 | M | none | cli | done | Inspect carries the OCI config |
| [T-1418](image.md) | P1 | M | none | image | done | A noexec store refuses before the pull |
| [T-1419](image.md) | P1 | M | none | image | done | Pull-missing verifies the blobs it trusts |
| [T-1420](image.md) | P2 | S | none | image | done | Transport errors name the host once |
| [T-1421](supervise.md) | P1 | M | none | supervise | done | A foreground run leaves a container record |
| [T-1422](gate.md) | P2 | S | none | gate | done | Plant the kept-session check where the OS execs batch files |
| [T-1423](packaging.md) | P1 | S | none | packaging | done | The release tag names the workspace version |
| [T-1559](packaging.md) | P2 | M | none | packaging | done | Port `scripts/build-state.py` to `podvm-buildstate` |
| [T-1560](packaging.md) | P2 | S | none | packaging | done | Port `scripts/release-licenses.py` to `podvm-release-licenses` |
| [T-1561](packaging.md) | P2 | M | none | packaging | done | Port `scripts/verify-release.sh` to `podvm-verify` |
| [T-1562](deps.md) | P0 | M | none | deps | done | Port `experiments/110-bloat-delta.sh` to `podvm-size` |
| [T-1563](packaging.md) | P1 | M | none | packaging | done | Port `experiments/120-reproducible-build.sh` and `157` to `podvm-prove-t0211` |
| [T-1564](complete.md) | P2 | M | none | complete | done | Port `scripts/document-state.py` to `document-state` |
| [T-1565](packaging.md) | P2 | S | none | packaging | done | Port `scripts/release-notes.sh` to `release-notes` |
| [T-1566](packaging.md) | P2 | M | none | packaging | done | Port `experiments/395-reconcile-repository.py` to `podvm-reconcile` |
| [T-1567](packaging.md) | P2 | M | none | packaging | done | Port `experiments/399-publication.py` to `podvm-publish` |
| [T-1568](machine.md) | P2 | M | none | machine | done | Port `experiments/145-podvm-parity.sh` to `podvm-parity` |
| [T-1569](machine.md) | P2 | S | none | machine | done | Port `experiments/154-tcg-workload-spread.sh` to `podvm-workload` |
| [T-1601](interpose.md) | P0 | S | none | interpose | done | Unblock the static build under Rust 1.99 `invalid_runtime_symbol_definitions` |
| [T-1602](image.md) | P0 | M | none | image | done | The registry fixture fields are read by their consumer tests |
| [T-1603](gate.md) | P0 | M | none | gate | done | Remove `check-no-secrets` and add a pinned trufflehog scan workflow |
| [T-1604](enter.md) | P1 | M | none | enter | done | Fix the detached-stdio serial mutex and the 1-of-3 detached start failure |
| [T-1605](packaging.md) | P1 | S | none | packaging | done | Push a version tag and prove the nightly release-notes wiring live |
| [T-1606](machine.md) | P2 | L | none | machine | done | Repoint all 47 `references/` citations before deleting the corpus |
| [T-1607](gate.md) | P1 | M | none | gate | done | File the 21 unbatched plan tasks as a later queue |
| [T-1608](gate.md) | P2 | S | none | gate | done | Reword the Batch 3 Done paragraphs so one-home passes |
| [T-1609](gate.md) | P0 | M | none | gate | done | Build the KVM watchdog that makes an unattended guest run safe |
| [T-1610](gate.md) | P0 | M | none | gate | done | Repair the base's stale podman state and settle which qemu the proof uses |
| [T-1611](gate.md) | P1 | S | none | gate | done | Repoint the code maps and limits page after the Batch 3 port |
| [T-1612](gate.md) | P1 | S | none | gate | done | Give the unowned work in the record an owner |
| [T-1613](gate.md) | P1 | S | none | gate | done | The 21 unbatched plan rows are in the record, and two were already tracked |
| [T-1619](packaging.md) | P0 | M | none | packaging | done | The `10`/`20`/`130` three-way decision belongs to the wave that deletes `10` |
| [T-1620](image.md) | P1 | S | none | image | done | The lock-race clause decision belongs to the store work it cites |
| [T-1621](gate.md) | P2 | S | none | gate | done | The 11 Batch 3 rows were already tracked and are done |
| [T-1622](gate.md) | P0 | S | none | gate | done | Reconcile the ledger's 121 rows against the seven entries and record the three VC changes |
| [T-1623](milestones.md) | P3 | S | none | milestones | done | Record the 29 KEEP-SHELL rows as the retained set, re-derived from the ledger |
| [T-1624](deps.md) | P2 | S | none | deps | done | Record the 265-row parity figure and the four stale values the tree carries |
| [T-1625](image.md) | P1 | S | none | image | done | Record that `70-whiteout-contract.sh` check B needs a fixture, and where it goes |
| [T-1626](machine.md) | P1 | S | none | machine | done | Record that `149` clause 6 has no Rust arm and stays shell pending the guest fixture |
| [T-1627](gate.md) | P1 | S | none | gate | done | Record `80-interposer-abi.sh` check B and `170` clause 3 as staying shell |
| [T-1628](interpose.md) | P1 | S | none | interpose | done | Record `162-tar-symlink-modes.sh` as retained, DELETE refuted by VC-2 |
| [T-1629](probe.md) | P2 | S | none | probe | done | Record `95-podman-vfs-ignorechown.sh` as KEEP-SHELL, a deployment proof under VC-3 and VC-6 |
| [T-1630](cli.md) | P1 | S | none | cli | done | The symlink-escape script is converted, and its deletion has a condition |
| [T-1631](milestones.md) | P2 | S | none | milestones | done | Record the `10`/`20`/`130` irreducible dependency as open, beside its decision entry |
| [T-1632](gate.md) | P1 | S | none | gate | done | Record the 149-tree boundary against the 121 top-level figure |
| [T-1633](deps.md) | P2 | S | none | deps | done | Record the 13-versus-3 `plant.sh` `Prove` discrepancy and which count is right |
| [T-1634](image.md) | P1 | S | none | image | done | Record that `serve_once` is test-private and no fixture existed before T-1602 |
| [T-1635](gate.md) | P1 | S | none | gate | done | Record the `py_compile` glob at `gate.yml:186` as needing a change |
| [T-1636](gate.md) | P1 | S | none | gate | done | Record the second workflow, `nightly.yml`, as a consumer of four ported subjects |
| [T-1637](milestones.md) | P2 | S | none | milestones | done | Record the eleven converged-on-Rust counts and the 29 that stay in shell |
| [T-1638](gate.md) | P0 | S | none | gate | done | The record gate proves itself with the tool its own port deletes |
| [T-1639](interpose.md) | P1 | S | none | interpose | done | Record the six interpose members and their excluded-crate proofs as separate jobs |
| [T-1640](interpose.md) | P1 | S | none | interpose | done | Record that the interpose export check needs no `rlib` and no new test |

| [T-1641](gate.md) | P0 | M | none | windows | done | The provisioner writes into the vendor's backing image, and a timed-out run orphans its emulator |
| [T-1642](deps.md) | P1 | S | none | deps | done | Refuse a Linux-target cargo build launched on a Windows host |
| [T-1643](interpose.md) | P1 | M | none | interpose | done | A relative path resolves against the host cwd and gets the rootfs prefix twice |
| [T-1644](interpose.md) | P1 | M | none | interpose | done | Admit a glibc payload against its whole NEEDED closure, read inside the rootfs |
| [T-1645](interpose.md) | P1 | M | none | interpose | done | Intercept `dlopen` and `dlmopen` |
| [T-1646](interpose.md) | P1 | L | none | interpose | done | A dynamic grandchild re-enters through the host loader |
| [T-1647](enter.md) | P1 | M | none | enter | done | Stream a payload into a memfd: the 128 MiB read ceiling refuses large runtimes |
| [T-1648](enter.md) | P0 | S | none | enter | done | A static Go payload runs into a GOROOT error instead of a named refusal |
| [T-1649](cli.md) | P2 | S | none | cli | done | A `#!` entrypoint needs an undocumented `--entrypoint`, and the refusal must name the way out |
| [T-1650](cli.md) | P1 | M | none | cli | done | `--strict` does not refuse the userland grandchild limit, while `--strict=all` refuses every run |
| [T-1651](enter.md) | P2 | L | none | enter | done | An emulate tier on a prefix-preserving, fail-closed qemu-user for static payloads |
| [T-1652](enter.md) | P2 | L | none | enter | done | A native-static rung: userland execve, Syscall User Dispatch and openat2(RESOLVE_IN_ROOT) |
| [T-1653](enter.md) | P2 | L | none | enter | done | `-v` volumes on a host with no /dev/fuse and no mount |
| [T-1654](cli.md) | P2 | L | none | cli | done | Profiles: strict TOML recipes with `extends` and `-s` overrides |
| [T-1655](cli.md) | P2 | L | none | cli | done | The sandbox hardening line: each `run` flag works on the tier or refuses by name |
| [T-1656](machine.md) | P2 | L | none | machine | done | MicroVMs: a fast disposable guest from an OCI image, on KVM or TCG |
| [T-1657](enter.md) | P2 | L | none | enter | done | A syscall-interpreting tier: the gVisor capability without user namespaces |
| [T-1658](enter.md) | P2 | L | none | enter | done | A confine layer: Landlock, seccomp and no_new_privs on every rootfs tier |
| [T-1659](cli.md) | P2 | M | none | cli | done | Provider shapes and bare userlands as built-in profiles |
| [T-1660](cli.md) | P2 | L | none | cli | done | `podvm notebook`: Jupyter and marimo cycles that keep one directory |
| [T-1661](cli.md) | P2 | M | none | cli | done | Language runtime profiles: Node, Deno and Bun permissions, wasmtime and workerd |
| [T-1662](net.md) | P2 | L | none | net | partial | Network labs: NAT, IPv6, policy and impairment labs inside one guest |
| [T-1663](net.md) | P2 | L | none | net | blocked | Tunnels and live uplinks: WireGuard kill switch, WARP uplink, proxy-forced programs |
| [T-1664](net.md) | P2 | L | none | net | blocked | Observability: capture, counters and connection logs without `CAP_NET_RAW` |
| [T-1665](net.md) | P2 | L | none | net | done | An egress broker, and the guest's view of a loopback-only proxy |
| [T-1666](probe.md) | P2 | L | none | probe | done | A security probe inside the payload, and a compare against a reference report |
| [T-1667](cli.md) | P2 | L | none | cli | partial | Doctor and installers: one check per scope capability, verified downloads, a space floor |
| [T-1668](gate.md) | P2 | M | none | gate | done | A findings page for the scope: observed or inferred, with host conditions per row |
| [T-1669](deps.md) | P1 | M | none | deps | done | Pin the stable Rust of the day and edition 2024, before the first `pod-*` crate |
| [T-1670](probe.md) | P1 | M | none | probe | done | The first cloud session measures its host and saves the result |
| [T-1671](packaging.md) | P2 | M | none | packaging | done | Release the first podvm beta with `## Breaking` notes, from a dispatched `release.yml` |
| [T-1672](gate.md) | P2 | L | none | gate | done | Move the record to the format of `pod-todo`, with a `podvm-todo` wrapper and a todo.toml |
| [T-1673](gate.md) | P2 | M | none | gate | done | File the 29 plan rows that T-1607 owns, read from the archive branch |
| [T-1674](gate.md) | P1 | L | none | gate | done | Split every Rust source file to 500 lines, then hold the limit in the gate |
| [T-1675](deps.md) | P2 | M | none | deps | done | Take `pod-term` for the SSH line discipline |
| [T-1676](deps.md) | P2 | M | none | deps | done | Take `pod-tls` for both TLS stacks, with its synchronous face |
| [T-1677](deps.md) | P2 | L | none | deps | done | Take `pod-proxy` with feature `blocking`: HTTPS_PROXY egress for pulls and the SSH dial |
| [T-1678](deps.md) | P2 | L | none | deps | done | Take `pod-relay` with feature `blocking` for the relay, keeping `reverse-v1` |
| [T-1679](deps.md) | P2 | M | none | deps | done | Take `pod-services` for the third-party hosts compiled into podvm |
| [T-1680](deps.md) | P2 | L | none | deps | done | Take `pod-todo` for the record half of `podvm-gate` |
| [T-1681](deps.md) | P2 | L | none | deps | done | Hand the seven generic crates to podcrates and freeze their files |
| [T-1682](machine.md) | P2 | M | none | machine | done | A `kvm.yml` workflow that runs the machine tier's Linux guest on a hosted runner |
| [T-1683](gate.md) | P2 | M | none | gate | done | Retire the PowerShell twins of `scripts/common/` and `check-twins.sh` |
| [T-1684](gate.md) | P2 | L | none | gate | done | Fuzz the parsers that read untrusted input, and watch `vendor/` upstream |
| [T-1685](gate.md) | P2 | M | none | gate | done | A maintenance page and a development page in the sibling form |
| [T-1686](complete.md) | P2 | S | none | complete | done | The device tests of a stand-in fail where `mknod` is permitted |
| [T-1687](deps.md) | P2 | S | none | deps | done | Fetch the Alpine netboot kernel and modloop from the 3.22.6 archive |
| [T-1688](enter.md) | P2 | M | none | enter | done | The memfd rung measures whether a memfd executes, names `vm.memfd_noexec`, and refuses before staging |
| [T-1689](probe.md) | P2 | S | none | probe | done | The non-goal `remedy` follows the stance in the probe JSON |
| [T-1690](cli.md) | P2 | S | none | cli | done | The doctor disk-free row judges the soft `RLIMIT_FSIZE` |
| [T-1691](supervise.md) | P1 | S | none | supervise | done | `stop -t 0` is a stop: no grace sends SIGKILL at once, and a failed bound is not a gone launcher |
| [T-1692](milestones.md) | P2 | S | none | windows | done | The `windows --podvm-mem` row says what the verb does about `RLIMIT_FSIZE` |
| [T-1693](packaging.md) | P2 | S | none | packaging | done | README and SECURITY name the last podbox tag that the verifier uses |
| [T-1694](cli.md) | P2 | L | none | cli | done | Headless Chromium of Playwright runs as a proven payload with its flags in one profile |
| [T-1695](cli.md) | P1 | L | none | cli | done | One JSON report for each run, and real labels on records |
| [T-1696](cli.md) | P2 | M | none | cli | done | `--secret` places a file with no argv and no env, and a read-once mode |
| [T-1697](interpose.md) | P1 | L | none | interpose | done | A raw `statx` of libuv misses the path table, so npm cannot find its modules |
| [T-1698](extract.md) | P0 | S | none | extract | done | A whiteout of `..` deletes outside the destination |
| [T-1699](interpose.md) | P2 | S | none | interpose | done | `mkostemp` of a mapped template answers ERANGE, and npm prints a line for each cache entry |
| [T-1700](enter.md) | P1 | M | none | enter | done | A run on the userland rung sometimes exits 0 with no output |
| [T-1701](interpose.md) | P2 | S | none | interpose | done | `mkstemp64`, `mkostemp64`, `mkstemps` and `mkostemps` pass the path table |
| [T-1702](machine.md) | P2 | M | none | machine | done | The TCG guest of `podvm-workload` prints no console byte in 600 s on the cloud host |
| [T-1703](interpose.md) | P2 | L | none | interpose | done | The exec routes that T-1646 does not take still enter through the host |
| [T-1704](interpose.md) | P1 | S | none | interpose | done | `realpath` of `/proc/self/exe` names the image's loader, while `readlink` names the program |
| [T-1705](interpose.md) | P1 | M | none | interpose | done | A host path that a loaded object reports gets the rootfs prefix a second time |
| [T-1706](cli.md) | P2 | M | none | cli | done | The unmodified official `actions/runner` starts on the userland rung |
| [T-1707](enter.md) | P1 | S | none | enter | done | A relative first program resolves against the image root, not against `-w` |
| [T-1708](supervise.md) | P1 | M | none | supervise | done | `stop`, `kill` and `rm -f` end the whole tree: the launcher is a subreaper |
| [T-1709](image.md) | P1 | L | none | image | done | Each container gets its own rootfs: a write in one container is not read by another |
| [T-1710](interpose.md) | P1 | M | none | interpose | done | A Unix socket path in `connect` and `bind` passes the path table |
| [T-1711](cli.md) | P2 | L | none | cli | done | The calls of the four container-hook commands run through podvm verbs on the userland rung |
| [T-1712](cli.md) | P1 | S | none | cli | done | `--podvm-tier=chroot` does nothing: it runs the ladder |
| [T-1713](cli.md) | P1 | S | none | cli | done | `.EnteredRung` reads `chroot` where the probe selects `supervise` and chroot is denied |
| [T-1714](probe.md) | P2 | S | none | probe | blocked | `podvm probe --json` validates against the published schema of `pod-hostprobe` |
| [T-1715](supervise.md) | P2 | S | none | supervise | done | The container table keeps `-e` values in a file that other users can read |
| [T-1716](enter.md) | P2 | S | none | enter | done | Take the `pod-enter` change of podcrates podcrates@T-155: `Maps::new` takes `&Names` and names the volume of a missing `-w` |
| [T-1717](machine.md) | P2 | L | none | machine | done | Study incus without root and without heavy overhead, with errand-box, before an incus back end |
| [T-1718](machine.md) | P2 | L | none | machine | done | An incus back end: podvm drives incus over its REST API |
| [T-1719](enter.md) | P2 | S | none | enter | done | The loader's library directories leave out `/usr/local/lib` and the image's `ld.so.conf` |
| [T-1720](supervise.md) | P1 | S | none | supervise | done | `wait` gives no exit code for a payload that ends at once |
| [T-1721](interpose.md) | P2 | L | none | interpose | done | An interposer for aarch64: the userland rung runs no arm64 payload |
| [T-1722](machine.md) | P3 | L | none | machine | done | The incus back end: the verbs of an instance that stays, volumes and the ladder |
| [T-1723](machine.md) | P3 | L | none | machine | done | The microVM of run: the microvm machine, a shared /workspace and an interactive stdin |
| [T-1724](enter.md) | P2 | M | none | enter | done | A static child of the loader family, and a volume, on the native-static rung |
| [T-1725](interpose.md) | P1 | M | none | interpose | done | An absolute symlink in the rootfs resolves on the host for each path call |
| [T-1726](cli.md) | P1 | S | none | cli | done | `exec` into a container ignores its `:ro` volumes and `--read-only` |
| [T-1727](machine.md) | P2 | S | none | machine | done | Take podcrates v0.21.0: `pod-incus` replaces the incus client of T-1718 |
| [T-1728](machine.md) | P2 | S | none | machine | done | Without `-t`, a command of the machine tier writes to a terminal |
| [T-1729](enter.md) | P3 | M | none | enter | done | An optional `runsc` backend where a host grants user namespaces |
| [T-1730](enter.md) | P1 | S | none | enter | done | A detached thread of a static payload ends with SIGSEGV on the native-static rung |
| [T-1731](enter.md) | P2 | L | none | enter | done | A dynamic program on the native-static rung: its loader under the handler |
| [T-1732](enter.md) | P1 | S | none | enter | done | Take podcrates v0.22.0: a detached payload ends with its launcher |
| [T-1733](podssh.md) | P3 | S | none | podssh | done | Retire the experiment 388-interactive-shell.sh for the tests of `session_interactive.rs` |
| [T-1734](extract.md) | P3 | S | none | extract | done | Retire the experiment 220-extract-path-safety.sh, and point its eight citations at the tests |
| [T-1735](complete.md) | P2 | M | none | complete | done | The dangling-directory-link test and the reachability test, then retire `85-completion-symlink-escape.sh` |
| [T-1736](machine.md) | P3 | S | none | machine | done | A test of the refusal of `windows --dos` with no DOS base image |
| [T-1737](packaging.md) | P3 | M | none | packaging | done | Decide where the SSH packaging scripts go, and port scripts/package-ssh.sh into `podvm-release` |
| [T-1738](interpose.md) | P1 | S | none | interpose | done | A buffer of the interposer reaches the free of a program that has its own allocator |
| [T-1739](interpose.md) | P2 | M | none | interpose | done | Chromium's child processes do not start on the loader family |
| [T-1740](enter.md) | P3 | M | none | enter | partial | Raw system calls of a dynamic program on the loader family: measure the syscall tier, then choose the default |
| [T-1741](image.md) | P1 | S | none | image | done | A plain tar layer whose end padding is past the last read is refused as a digest mismatch |
| [T-1742](enter.md) | P2 | S | none | enter | done | The loader family does not find the glibc of a Debian image on arm64 |
| [T-1743](interpose.md) | P1 | S | none | interpose | done | GNU tar with an absolute `-C` opens the directory on the host: the fortified opens skip the table |
| [T-1744](interpose.md) | P2 | S | none | interpose | done | The open flags of the interposer have their x86_64 values on aarch64 |
| [T-1745](cli.md) | P3 | L | none | cli | done | A Rust call for `run` with a typed request that returns the record of `--report` |
| [T-1746](probe.md) | P2 | M | none | probe | blocked | `run` reads the cached probe, and probes again only when its key changes |
| [T-1747](cli.md) | P3 | M | none | cli | open | `--memory`, `--cpus` and `--pids-limit` by cgroup2 where a subtree is delegated |
| [T-1748](supervise.md) | P2 | S | none | supervise | done | A foreground payload dies when its `podvm run` is killed with SIGKILL |
| [T-1749](enter.md) | P3 | L | none | enter | partial | A read-only image as the lower layer of the root, with a tmpfs upper layer, and no copy |
| [T-1750](machine.md) | P3 | L | none | machine | open | `exec` into a running guest of the machine tier |
| [T-1751](machine.md) | P3 | L | none | machine | open | The guest plan of shield `guest`: no host file in the guest, no network card, one serial channel |
| [T-1752](cli.md) | P3 | M | none | cli | done | A versioned JSON Schema for the record of `run --report`, published in each release |
| [T-1753](interpose.md) | P1 | M | none | interpose | done | The userland rung reads the host's /etc/hosts, /etc/passwd and /etc/group through libc's internal opens |
| [T-1754](interpose.md) | P1 | S | none | interpose | open | `scandir64` and other 64-bit directory calls skip the path table: node lists the host's /etc, npm install fails |
| [T-1755](interpose.md) | P1 | S | none | interpose | open | `lutimes` skips the path table: dpkg fails on each package, so apt-get install fails |
| [T-1756](interpose.md) | P2 | S | none | interpose | open | A trigger of apk runs `/bin/busybox` by an absolute path that is not anchored |
| [T-1757](cli.md) | P1 | S | none | cli | open | `-u` is dropped on the namespace rung when the image has an ENTRYPOINT: the payload runs as root |
| [T-1758](enter.md) | P1 | M | none | enter | blocked | The namespace rung has no /proc in the root: go cannot find GOROOT, /proc/self/status is missing |
| [T-1759](enter.md) | P1 | M | none | enter | done | No /dev/shm on either rung: python multiprocessing fails |
| [T-1760](cli.md) | P2 | M | none | cli | open | `-t` gives no terminal on the namespace rung, and says nothing |
| [T-1761](cli.md) | P1 | M | none | cli | done | `-v`, `--tmpfs` and `--read-only` are refused on the namespace rung with a message that names the chroot rung |
| [T-1762](image.md) | P1 | M | none | image | open | The source fixup probes four HTTPS hosts on each run: about 2 s of each ubuntu run, uncached |
| [T-1763](image.md) | P2 | S | none | image | open | The `openssl rehash` step prints 108 lines on each python run and costs about 0.5 s |
| [T-1764](enter.md) | P2 | S | none | enter | open | `--podvm-confine` without /etc in the read list breaks the loader on the namespace rung |
| [T-1765](cli.md) | P1 | S | none | cli | partial | `-e NAME` with no value does not pass the host's variable, and `exec` does not take the environment of `run` |
| [T-1766](cli.md) | P2 | S | none | cli | open | `--strict` and `probe` disagree with the rung that a run enters on the userland fixture |
| [T-1767](image.md) | P2 | S | none | image | open | Quiet runs print notes: a false chown EINVAL on extraction, the probe line of `pull -q`, the base archive of the machine tier |
| [T-1768](image.md) | P2 | S | none | image | open | `cp -r` of a container path copies each hard link as a file |
| [T-1769](cli.md) | P3 | S | none | cli | open | An unknown verb or flag prints the whole usage |
| [T-1770](net.md) | P3 | S | none | net | open | `podvm lab`: a knob line with an empty value, and the words of the errors of the knobs |
| [T-1771](image.md) | P2 | S | none | image | open | node does not trust the host's CA that podvm appends, so npm fails behind an intercepting proxy |

## Counts

380 entries: 17 open, 6 partial, 5 blocked, 352 done.

Counted from the rows above by
`./target/x86_64-unknown-linux-musl/release/podvm-count` and asserted
independently by `./target/x86_64-unknown-linux-musl/release/podvm-gate`, which is
the gate. A number here that disagrees with the rows cannot reach a commit.

| Priority | Open | Partial | Blocked | Done | Total |
| --- | --- | --- | --- | --- | --- |
| P0 | 0 | 0 | 0 | 65 | 65 |
| P1 | 4 | 2 | 1 | 155 | 162 |
| P2 | 8 | 2 | 4 | 114 | 128 |
| P3 | 5 | 2 | 0 | 18 | 25 |
| **All** | 17 | 6 | 5 | 352 | 380 |

## Work order

[PROGRESS.md](PROGRESS.md) gives the current order. A result that reports
success for a wrong answer takes priority over a visible failure. Each entry
names its own proof. The old ordering text is kept in
[`docs/history/index-order-before-2026-09-28.txt`](../docs/history/index-order-before-2026-09-28.txt).
