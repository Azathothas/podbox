# TODO

Every entry, one line each, sorted by id. The entry itself lives in the
`TODO/<category>.md` the row links to, and it closes there with its own
acceptance command, actually run, with the output recorded underneath.

**What to work on next is not here.** [PROGRESS.md](PROGRESS.md) carries the
work order and is the only place that has one. This file carries the list, the
definitions, the counts, and the argument behind the current ordering.

[RULES.md](RULES.md) is how this repository is worked on.
[reference-map.md](reference-map.md) is the corpus and its licence
determinations.

`scripts/check-todo.py` checks this file against the entries: a status that
disagrees, a row with no entry, an entry with no row, a count that does not add
up, a missing field, a `Prove` that is not a command, a reference that does not
resolve, a dead link, and a cited path or line that does not exist.

```sh
./scripts/check-todo.py
```

## Priority

- **P0** breaks correctness, loses data, or takes the process down.
- **P1** a documented capability does not work, or a flag does nothing.
- **P2** worth doing, nothing is wrong without it.
- **P3** worth recording so it is not rediscovered.

## Effort

S is under a day, M is a few days, L is a week, XL is longer and is almost
always two entries pretending to be one.

## Status

**open**, **partial**, **blocked** or **done**. ⛔ Nothing closes as "won't
fix", "upstream's problem" or "out of scope". A blocked entry stays open with
the blocker named and what would clear it.

## Categories

| category | crate | specification |
| --- | --- | --- |
| [probe](probe.md) | `crates/podbox-probe` | `TOOL.md` section 6.1 |
| [image](image.md) | `crates/podbox-image` | `TOOL.md` section 6.2 |
| [extract](extract.md) | `crates/podbox-extract` | `TOOL.md` section 6.3 |
| [complete](complete.md) | `crates/podbox-complete` | `TOOL.md` section 6.4 |
| [enter](enter.md) | `crates/podbox-enter` | `TOOL.md` section 6.5 |
| [supervise](supervise.md) | `crates/podbox-supervise` | `TOOL.md` section 6.6 |
| [interpose](interpose.md) | `crates/podbox-interpose` | `TOOL.md` section 6.7 |
| [cli](cli.md) | `crates/podbox-cli` | `TOOL.md` section 6.8, section 6.9 |
| [deps](deps.md) | the whole tree | `TOOL.md` section 3.5 |
| [packaging](packaging.md) | the artefact | `TOOL.md` section 3.4, section 5 M7 |
| [milestones](milestones.md) | the gates | `TOOL.md` section 5 |
| [podvm](podvm.md) | the machine tier | `https://github.com/talaria0101/vm-research`, its podvm-spec document |
| [gate](gate.md) | `scripts/` | `docs/methodology/gate.md` |

## Entries

| ID | Priority | Category | Status | Item |
| --- | --- | --- | --- | --- |
| [T-0101](probe.md) | P0 | probe | done | The probe set, one disposable child per probe, errno not boolean |
| [T-0102](probe.md) | P0 | probe | done | Separate a filtered syscall from an executed one with a bogus argument |
| [T-0103](probe.md) | P1 | probe | done | Probe creation and attachment separately |
| [T-0104](probe.md) | P0 | probe | done | Probe the write allowlist by writing, for blocks and inodes |
| [T-0105](probe.md) | P1 | probe | done | Read the ID maps directly rather than inferring them |
| [T-0106](probe.md) | P2 | probe | done | The `mknod` pair, because one of them tests nothing |
| [T-0107](probe.md) | P0 | probe | done | Mode selection, and one switch that turns every degradation into a refusal |
| [T-0108](probe.md) | P0 | probe | done | The mode banner |
| [T-0109](probe.md) | P0 | probe | done | A verdict is the operation's, and "could not run" never reads as "denied" |
| [T-0110](probe.md) | P1 | probe | done | `podbox probe` exit-code and channel contract |
| [T-0111](probe.md) | P2 | probe | done | Cache the probe result, and key it on what actually decides it |
| [T-0112](probe.md) | P1 | probe | done | A virtual machine answers the two questions this host's kernel cannot |
| [T-0113](probe.md) | P1 | probe | done | Two probes in one process took each other's scratch name |
| [T-0201](image.md) | P0 | image | done | Registry client, HTTPS only, with no plain-HTTP fallback |
| [T-0202](image.md) | P0 | image | done | A content-addressed store, and digest parity with docker |
| [T-0203](image.md) | P0 | image | done | Check `statvfs` for blocks and inodes, and name the destination |
| [T-0204](image.md) | P1 | image | done | `images`, `rmi`, `tag`, and a store GC that cannot delete a running container's rootfs |
| [T-0205](image.md) | P3 | image | done | Re-test podman with `vfs` and `ignore_chown_errors` before repeating "no path exists" |
| [T-0206](image.md) | P3 | image | done | A registry fixture, so the acceptance stops depending on somebody else's quota |
| [T-0207](image.md) | P2 | image | done | Fetch layers with bounded concurrency, and measure what it buys |
| [T-0208](image.md) | P2 | image | done | `--platform`, and a store that can hold two variants of one tag |
| [T-0209](image.md) | P2 | image | done | Registry authentication, without a credential ever entering this tree |
| [T-0210](image.md) | P1 | image | done | The store's concurrency contract, written down and driven |
| [T-0211](image.md) | P1 | image | done | An image lock outlives its holder whenever anything forks |
| [T-0212](image.md) | P0 | image | done | The platform is decided at run time, and the store holds more than one |
| [T-0213](image.md) | P0 | image | done | A registry with no certificate, or one nothing trusts, and the refusal kept |
| [T-0214](image.md) | P2 | image | done | A blob body cut off mid-stream is not retried, and the bounded retry is around the wrong thing |
| [T-0215](image.md) | P0 | image | done | Four lock tests fail in two runs of five, and the gate has never said so |
| [T-0301](extract.md) | P0 | extract | done | Extract in-process, at entry level, never through system `tar` |
| [T-0302](extract.md) | P0 | extract | done | Ownership-neutral extraction plus the sidecar |
| [T-0303](extract.md) | P0 | extract | done | Whiteouts are matched on the basename, never with a path glob |
| [T-0304](extract.md) | P0 | extract | done | Refuse an entry that resolves outside the destination, including through a symlink from the same layer |
| [T-0305](extract.md) | P1 | extract | done | An absolute symlink target is rootfs-relative, not a refusal |
| [T-0306](extract.md) | P1 | extract | done | Re-permission between layers, or the second layer fails |
| [T-0307](extract.md) | P1 | extract | done | Hard links, symlinks and the layer order |
| [T-0401](complete.md) | P0 | complete | done | Device shims as regular files |
| [T-0402](complete.md) | P0 | complete | done | Always install the host's `/etc/resolv.conf` |
| [T-0403](complete.md) | P2 | complete | done | `/etc/hosts` |
| [T-0404](complete.md) | P0 | complete | done | Synthesize `/etc/passwd` and `/etc/group`, and let the payload's writes persist |
| [T-0405](complete.md) | P1 | complete | done | `/etc/mtab` is a symlink, and writing through it escapes the rootfs |
| [T-0406](complete.md) | P1 | complete | done | pacman: `DownloadUser` and the keyring |
| [T-0407](complete.md) | P1 | complete | done | apt: the sandbox user, https sources and the CA bundle |
| [T-0408](complete.md) | P2 | complete | done | zypper: fix the RIS index, not `repos.d` |
| [T-0409](complete.md) | P2 | complete | done | Ownership failures from `dpkg`, `rpm` and `xbps` are warnings |
| [T-0410](complete.md) | P0 | complete | done | Supply `/etc/nsswitch.conf`, or the supplied `/etc/passwd` is a no-op |
| [T-0411](complete.md) | P1 | complete | done | A payload whose package sources are `http://`, on a runtime where tcp/80 hangs |
| [T-0412](complete.md) | P1 | complete | done | A fixup that has to run INSIDE the rootfs, and podbox runs it from outside |
| [T-0413](complete.md) | P1 | complete | done | No `/proc` inside a chroot, and the shell feature that quietly stops working |
| [T-0414](complete.md) | P1 | complete | done | Two walls only one instance of the class has shown, and podbox has probed neither |
| [T-0415](complete.md) | P1 | complete | done | A device stand-in is checked by type, because an absent one becomes a growing file |
| [T-0501](enter.md) | P0 | enter | done | Open every descriptor before the root changes |
| [T-0502](enter.md) | P0 | enter | done | Resolve the program inside the new root, in the process that changed it |
| [T-0503](enter.md) | P1 | enter | done | Probe `/dev/ptmx`, and refuse `-t` by name where it is absent |
| [T-0504](enter.md) | P1 | enter | done | Refuse a rootfs path that is a symlink |
| [T-0505](enter.md) | P1 | enter | done | `exec` is a fresh chroot, and `inspect` says so |
| [T-0506](enter.md) | P0 | enter | done | A foreign-architecture container, and never a rung measured by the emulator |
| [T-0601](supervise.md) | P0 | supervise | done | One pidfd per direct child, `waitid` for status |
| [T-0602](supervise.md) | P0 | supervise | done | Never decide "running" by sleeping and looking |
| [T-0603](supervise.md) | P1 | supervise | done | `PR_SET_PDEATHSIG` fires on the creating thread's exit |
| [T-0604](supervise.md) | P1 | supervise | done | Running state is launcher state |
| [T-0605](supervise.md) | P2 | supervise | done | Capture logs at spawn, from the descriptors opened in step 2 |
| [T-0606](supervise.md) | P0 | supervise | done | The notification tier: probe three legs, refuse the tier, never fall back per call |
| [T-0607](supervise.md) | P0 | supervise | done | The lifecycle, twenty times, twenty passes |
| [T-0608](supervise.md) | P1 | supervise | done | A detached container that reads `exited` with no launcher, seen twice and not reproduced |
| [T-0701](interpose.md) | P0 | interpose | done | The cdylib build constraints |
| [T-0702](interpose.md) | P0 | interpose | done | One object per libc, and it must live inside the rootfs |
| [T-0703](interpose.md) | P0 | interpose | done | Path virtualization: the entry-point set and `*at` resolution |
| [T-0704](interpose.md) | P0 | interpose | done | Ownership virtualization: the half a path interposer does not have |
| [T-0705](interpose.md) | P1 | interpose | done | Reverse mapping, so the payload reads back what it wrote |
| [T-0706](interpose.md) | P0 | interpose | done | Classify the payload and decline with a named reason |
| [T-0707](interpose.md) | P1 | interpose | done | The paths that must not be rewritten |
| [T-0708](interpose.md) | P2 | interpose | done | Intercept the operations the runtime cannot provide |
| [T-0709](interpose.md) | P0 | interpose | done | Select the interposer by `DT_NEEDED`, and refuse on the version predicate |
| [T-0710](interpose.md) | P1 | interpose | done | The ownership memo lives where the payload can edit it, and answers by linear scan |
| [T-0711](interpose.md) | P1 | interpose | done | The identity calls, and podbox's honesty rules point the other way from fakeroot's |
| [T-0712](interpose.md) | P2 | interpose | done | A reach matrix holds its arguments constant, or it measures two things |
| [T-0801](cli.md) | P0 | cli | done | The verb and flag parity table |
| [T-0802](cli.md) | P0 | cli | done | docker's exit codes, unaltered |
| [T-0803](cli.md) | P1 | cli | done | Answer to `docker` and `podman` on PATH |
| [T-0804](cli.md) | P0 | cli | done | The honesty rules, and one switch that makes every degradation fatal |
| [T-0805](cli.md) | P1 | cli | done | Diagnostics that name the operation, the errno, the mechanism and the remedy |
| [T-0806](cli.md) | P0 | cli | done | Never prompt, never wait unbounded, and check space before every large write |
| [T-0807](cli.md) | P2 | cli | done | `podbox images --format` refuses a template no verb can answer, before it looks at the store |
| [T-0808](cli.md) | P1 | cli | done | Drive every row of the parity table through the shipped binary |
| [T-0809](cli.md) | P1 | cli | done | The spawn that fails with the wrong reason, and the one field that fixes it |
| [T-0810](cli.md) | P1 | cli | done | `create` fails storing the memo: the container directory is never made |
| [T-0901](deps.md) | P1 | deps | done | Sweep: syscalls |
| [T-0902](deps.md) | P2 | deps | done | Sweep: seccomp BPF |
| [T-0903](deps.md) | P3 | deps | done | Sweep: Landlock |
| [T-0904](deps.md) | P1 | deps | done | Sweep: OCI registry client and types |
| [T-0905](deps.md) | P0 | deps | done | Sweep: TLS |
| [T-0906](deps.md) | P1 | deps | done | Sweep: HTTP |
| [T-0907](deps.md) | P0 | deps | done | Sweep: tar, gzip, zstd |
| [T-0908](deps.md) | P1 | deps | done | Sweep: digests, JSON, argument parsing, ELF |
| [T-0909](deps.md) | P1 | deps | done | Vendor the memfd and userland-exec rungs, and fix the fork's regression here |
| [T-0910](deps.md) | P0 | deps | done | The `cargo bloat` baseline, committed, and checked at the gate |
| [T-0911](deps.md) | P0 | deps | done | The syscall table and the kernel structs come from a crate, per architecture |
| [T-0912](deps.md) | P2 | deps | done | The powerpc gate is the crate's and it is stale, so podbox can clear it |
| [T-1001](packaging.md) | P0 | packaging | **done** | A single static binary with no `PT_INTERP` |
| [T-1002](packaging.md) | P1 | packaging | done | Embed the interposer as bytes and place it inside the rootfs |
| [T-1003](packaging.md) | P2 | packaging | done | The launch ladder, and a single file with an embedded rootfs |
| [T-1004](packaging.md) | P3 | packaging | done | A reproducible build, and the artefact's own inputs recorded |
| [T-1005](packaging.md) | P1 | packaging | done | A session reaches the code in one command, and the build runs behind the reading |
| [T-1100](milestones.md) | P0 | milestones | **done** | M-1 the corpus, the work index and the skeleton |
| [T-1101](milestones.md) | P0 | milestones | done | M0 the probe, and nothing else |
| [T-1102](milestones.md) | P1 | milestones | done | M1 image acquisition |
| [T-1103](milestones.md) | P0 | milestones | done | M2 extraction that survives the ownership wall |
| [T-1104](milestones.md) | P0 | milestones | done | M3 `run` on the chroot rung |
| [T-1105](milestones.md) | P0 | milestones | done | M4 the lifecycle, twenty times |
| [T-1106](milestones.md) | P1 | milestones | done | M5 environment completion, ten distributions |
| [T-1107](milestones.md) | P1 | milestones | done | M6 the interposer |
| [T-1108](milestones.md) | P2 | milestones | done | M7 packaging |
| [T-1109](milestones.md) | P1 | milestones | partial | The negative tests, which are tests |
| [T-1110](milestones.md) | P0 | milestones | done | M6's acceptance: a payload the interposer is the only reason works |
| [T-1111](milestones.md) | P1 | milestones | done | M8 the nix acceptance: a real payload the chroot tier is exactly the answer for |
| [T-1112](milestones.md) | P3 | milestones | done | A disposable guest that is not Linux |
| [T-1201](gate.md) | P0 | gate | done | The gate reaches every file this project wrote |
| [T-1202](gate.md) | P0 | gate | done | Every check is planted against, and a plant that stops reaching its subject says so |
| [T-1203](gate.md) | P1 | gate | done | A measurement taken on one host is a property of that host |
| [T-1204](gate.md) | P1 | gate | done | Check 17 holds the newest committed reading under the ceiling, not only the baseline |
| [T-1205](gate.md) | P1 | gate | done | The gate holds experiment numbers unique, because four Proves already collide |
| [T-1206](gate.md) | P0 | gate | done | CI installs the toolchain the build config names, and nine commits proved nobody was holding it |
| [T-1207](gate.md) | P1 | gate | done | The one crate that runs inside other people's processes is the one the gate does not check |
| [T-1208](gate.md) | P1 | gate | done | A closed entry carries its recorded run, and the gate can see it |
| [T-1209](gate.md) | P1 | gate | done | Thirty-nine `Prove` lines pull from the one registry the acceptance may not use |
| [T-1210](gate.md) | P1 | gate | done | Convert the interpose engine scripts to `experiments/lib/engine.sh` |
| [T-1211](gate.md) | P1 | gate | done | Convert the distribution and probe engine scripts to `experiments/lib/engine.sh` |
| [T-1212](gate.md) | P1 | gate | done | Convert the image, registry and CLI engine scripts to `experiments/lib/engine.sh` |
| [T-1213](gate.md) | P1 | gate | done | Convert the target-image pair and its probe consumer to `experiments/lib/engine.sh` |
| [T-1301](podvm.md) | P0 | podvm | done | The machine tier is probed leg by leg, and a present file is not a working one |
| [T-1302](podvm.md) | P1 | podvm | done | One binary, one parity table, and a VM-only flag that cannot collide |
| [T-1303](podvm.md) | P1 | podvm | done | The image is a rootfs directory, and an initramfs with no console is a silent machine |
| [T-1304](podvm.md) | P1 | podvm | done | The exec protocol is a serial pair with a nonce, and every wait is bounded |
| [T-1305](podvm.md) | P2 | podvm | done | The fleet and the fork, and the file-size ceiling that bounds both |
| [T-1306](podvm.md) | P1 | podvm | done | The non-goals are refusals the code makes, not notes in a document |
| [T-1307](podvm.md) | P2 | podvm | done | The five Rust VM tools, ruled one by one, so nobody surveys them again |
| [T-1308](podvm.md) | P1 | podvm | done | One TCG number is a claim about one benchmark, and the range is 3x to 21x |
| [T-1309](interpose.md) | P1 | interpose | done | libdnf repodata downloads fail under the preloaded interposer |
| [T-1310](image.md) | P1 | image | done | The store suite exhausts the sixteen fork-shed slots, and the victim varies |
| [T-1311](interpose.md) | P1 | interpose | done | The interposed `fchmodat` drops the `flags` argument |
| [T-1312](interpose.md) | P1 | interpose | done | The glibc interposer needs newer symbols than the payload provides |
| [T-1313](podvm.md) | P1 | podvm | done | Every experiment fetch carries its own ceiling, and a stalled origin proves it |
| [T-1314](packaging.md) | P1 | packaging | done | Nightly releases: one tag builds and tests every supported arch |
| [T-1315](extract.md) | P1 | extract | done | Extract re-verifies the blob digest against the manifest |
| [T-1316](deps.md) | P2 | deps | done | The committed lock matches the manifests, so a clean build leaves a clean tree |
| [T-1317](enter.md) | P1 | enter | done | A chroot-denied host gets an up-front refusal naming chroot, not a 125 after the work |
| [T-1318](supervise.md) | P3 | supervise | done | `logs -f` follows a container's log, bounded like the rest |
| [T-1319](cli.md) | P1 | cli | done | `inspect` prints exactly one document for any reference |
| [T-1320](image.md) | P2 | image | done | Images move without a registry: save, load, import |
| [T-1321](image.md) | P2 | image | done | A store-health verb and a pull-provenance record |
| [T-1322](image.md) | P1 | image | done | A container record gates every removal of the image it references |
| [T-1323](cli.md) | P2 | cli | done | `cp` reaches an image's extracted rootfs, and copies directories with the same checks |
| [T-1324](complete.md) | P2 | complete | done | The README tells the truth: build order, rung map, status, auth scope |
| [T-1325](gate.md) | P1 | gate | done | The gate checks what entries claim: reachable flags and true parity notes |
| [T-1326](gate.md) | P1 | gate | done | `check-markers.sh` builds its marker bytes portably across `/bin/sh` |
| [T-1327](interpose.md) | P2 | interpose | open | The interposer builds per architecture, and the T-0704 citations point at it |
| [T-1328](packaging.md) | P2 | packaging | open | The nightly signs its artefacts, with provenance a downloader can check |
| [T-1329](packaging.md) | P2 | packaging | open | The per-arch smoke pulls and extracts, not just versions |
| [T-1330](cli.md) | P2 | cli | done | Bundled short flags parse as docker reads them |
| [T-1331](cli.md) | P2 | cli | done | `--filter`, `restart` and `pull -a/-q` answer the docker idiom |
| [T-1332](cli.md) | P1 | cli | done | `podbox man` generates the manual from the binary, pager-aware |
| [T-1333](probe.md) | P1 | probe | done | The probe cache key carries the capability set |
| [T-1334](packaging.md) | P1 | packaging | open | The release carries its build commit's gate state and its reproducibility boundary |
| [T-1335](supervise.md) | P3 | supervise | open | The launcher forwards its own shutdown signals to the payload |
| [T-1336](cli.md) | P1 | cli | open | CLI output is plain ASCII: no emoji, no markers, on every path |

## Counts

165 items: 6 open, 1 partial, 0 blocked, 158 done.

Counted from the rows above by `scripts/todo-count.py` and asserted
independently by `scripts/check-todo.py`, which is the gate. A number here
that disagrees with the rows cannot reach a commit.

| Priority | Open | Partial | Blocked | Done | Total |
| --- | --- | --- | --- | --- | --- |
| P0 | 0 | 0 | 0 | 52 | 52 |
| P1 | 2 | 1 | 0 | 74 | 77 |
| P2 | 3 | 0 | 0 | 26 | 29 |
| P3 | 1 | 0 | 0 | 6 | 7 |
| **All** | **6** | **1** | **0** | **158** | **165** |

## How the current ordering is derived

Four questions, asked in this order, because a later answer never outranks an
earlier one. This is the argument, not the list: [PROGRESS.md](PROGRESS.md)
carries the ordered work.

### 1. Is anything wrong that reports success?

⭐ A wrong answer that exits 0 outranks a visible failure, because nothing in
the output says the wrong answer happened, and podbox's audience is automated
and cannot notice.

Nothing is in this shape yet, because nothing is implemented. Two entries exist
to keep it that way, and they are why they are P0 rather than P1:
[T-0109](probe.md) makes "could not run" a third state rather than a denial, and
[T-0606](supervise.md) refuses a tier rather than falling back per call. The
corpus supplies a worked example of the failure at file and line, in
`references/multikernel__sandlock`, and its own audit missed it.

### 2. Does anything downstream branch on it?

The probe does, and everything branches on the probe. That is why M0 is first
and why [probe.md](probe.md) carries ten entries against
[image.md](image.md)'s five.

### 3. Is the risk in the component or in the specification?

Extraction is the highest-risk component: four separate tools in the corpus stop
there, and three of its four requirements are now measured rather than read
(`experiments/results/whiteout-contract.txt`). It outranks image acquisition,
which is ordinary HTTPS and file I/O, even though acquisition comes first in
dependency order.

### 4. What is blocked, and on what?

Nothing is blocked. T-0702 is open rather than blocked: the required
musl tooling and per-libc objects now exist, and the remaining work is
integration.
