# milestones

Record semantics: [task rules](RULES.md#5-entry-closure).


`TOOL.md` section 5. One entry per milestone, carrying its acceptance test and nothing
else: the work is in the component files, and the milestone is the gate that
says the work is done.

[INDEX.md](INDEX.md) is the list and the counts. [PROGRESS.md](PROGRESS.md) is the work order.
[RULES.md](RULES.md) is how an entry closes. [reference-map.md](reference-map.md) is the corpus.

⛔ **Do not proceed on a milestone whose test has never run green.** A test that
has never run green does not count, and neither does a milestone built on one.

The order is a dependency order, not a preference. [PROGRESS.md](PROGRESS.md)
carries the work order.

---

### T-1100 M-1 the corpus, the work index and the skeleton

Source:      `TOOL.md` section 5 M-1
Category:    milestones
Priority:    P0
Effort:      L
Status:      done 2026-09-08

Problem:     Everything after this depends on `TODO/` existing and being right,
             and on the references having been read rather than cited.
Premise:     ⭐ **Measured, and the acceptance is four commands.** `TOOL.md` section 5
             M-1: the reader script exits 0; every entry cites a path and line
             that resolves; `cargo build --release --target
             x86_64-unknown-linux-musl` succeeds on the empty skeleton;
             `readelf -l` on the result shows no `PT_INTERP`.
Approach:    Produced: `docs/` copied verbatim; `LICENSE` (0BSD), `README.md`,
             `THIRD_PARTY.md`; the workspace of section 4.3 with every crate a
             skeleton; `rust-toolchain.toml` and `.cargo/config.toml`;
             `experiments/` seeded from
             `references/Azathothas__container-research` plus three measurements
             of this project's own; the corpus of thirty trees under
             `references/`; `TODO/` with its index, rules, record,
             reference map and per-category entries; the two count scripts, the
             reader wired as a gate; and CI that runs it.
Decision:    The corpus is **tracked in the tree** rather than on a side branch,
             because `crates/podbox-gate/src/main.rs` resolves every cited path and line
             and cannot do so into a branch it is not on.
             [reference-map.md](reference-map.md) records the choice and what a
             clone pays for it.
Prove:       `./target/release/podbox-gate && cargo build --release --target x86_64-unknown-linux-musl && readelf -l target/x86_64-unknown-linux-musl/release/podbox | grep -c INTERP | grep -qx 0`

**Done. The `Prove` command was run on 2026-09-08 and exits 0.** The output is
in [PROGRESS.md](PROGRESS.md)'s baseline block.

⭐ **After this, no session needs to read the references again.** Each entry
carries what to do and which reference to open at which line. That is the whole
point of paying for it once.

---

### T-1101 M0 the probe, and nothing else

Source:      `TOOL.md` section 5 M0
Category:    milestones
Priority:    P0
Effort:      M
Status:      done 2026-09-08

Problem:     Everything downstream branches on the probe, and it is the
             component the prior art most consistently gets wrong.
Premise:     Read. The work is [probe.md](probe.md) T-0101 through T-0110.
Approach:    `podbox probe` prints the mode it would select, the evidence, and
             exits 0. Nothing else is implemented at this milestone.
Decision:    The probe before the store, before extraction, before `run`.
             Reversing it means every later component carries an assumption
             about the machine that the probe would have replaced with a
             measurement.
Prove:       `./experiments/130-probe-parity.sh` exits 0. It runs the three clauses of the acceptance in one command: `chroot` inside `./experiments/20-enter-target.sh --stage ./target/x86_64-unknown-linux-musl/release/podbox`, `namespace` unconfined, and every attribution row against `experiments/results/attribute.txt`

**Done 2026-09-08.** `./experiments/130-probe-parity.sh` exits 0. Its transcript
is `experiments/results/probe-parity.txt`:

```
== 1. inside the reconstruction, the rung must be chroot
  got chroot
== 2. unconfined, the rung must be namespace
  got namespace
== 3. the attribution rows ...
  15 matched, 1 recorded divergence, 0 differed, 0 missing
```

⭐ **The acceptance is a script rather than three commands somebody re-types.**
The wording above was amended to name it: the three clauses are unchanged, and
`AGENTS.md`'s fourth absolute requires the measurement to ship with the
script that took it. `experiments/130-probe-parity.sh` is that script.

⛔ **Two blockers were in the way and both were defects in this tree, not in the
probe.**

1. `experiments/20-enter-target.sh` named `$REPO/verification/{confine,probe,cprobe}`
   for its harness sources, and this tree has no `verification/`: podbox tracks
   that repository under `references/`. Every invocation had been failing at
   `cd`, so the reconstruction had never run here. It now resolves
   `HARNESS_SRC` to
   `references/Azathothas__container-research/tree/verification` and prints it
   in the conditions block. **A citation that does not resolve, wearing a shell
   error message.**
2. `experiments/results/attribute.txt` did not exist. The acceptance names it,
   and nothing had produced it. `./experiments/30-attribution-census.sh --capture
   experiments/results` now has, alongside `census.txt` and `identity.txt`. It
   exits 2 on this host: the three Landlock rows cannot run here and are
   reported as skipped rather than passed.

⭐ **The one recorded divergence is the reference being wrong, not podbox.** The
Go instrument records `kcmp(-1,-1,...) [control] FAIL errno=38 ENOSYS`. `ENOSYS`
is this kernel saying `kcmp(2)` was not built in, which is the control being
absent rather than the control answering; podbox reports it as `skip`.
[T-0109](probe.md) is exactly that rule and [T-0102](probe.md) carries the
reading from both hosts. The parity script allows that one substitution, only
for that row, only when the errno numbers agree, and prints it in full on every
run; anything else is a mismatch.

⚠ **What M0 does NOT include, stated so the next milestone is not surprised.**
`run`, `exec` and the store are M3 and M1. [T-0107](probe.md) and
[T-0108](probe.md) are `partial` for that reason and each names the half that is
left. `TOOL.md` section 6.1's `$store/probe.json` cache is [T-0111](probe.md),
authored in this session and deliberately not implemented: it belongs beside
the store. ⛔ Authoring it found that the specification's cache key does not
work, and the entry carries the reading that settles it.

---

### T-1102 M1 image acquisition

Source:      `TOOL.md` section 5 M1
Category:    milestones
Priority:    P1
Effort:      M
Status:      done 2026-09-08

Problem:     No extraction is possible without a store, and no store is
             trustworthy without digest parity.
Premise:     Read. The work is [image.md](image.md) T-0201 through T-0204.
Approach:    `podbox pull`, `images`, `rmi`, `tag`, and a content-addressed
             store. No extraction yet.
Decision:    Do not gold-plate it. The registry plane is ordinary HTTPS and file
             I/O and it works here, and it is the least interesting part of the
             problem.
Prove:       `podbox pull public.ecr.aws/docker/library/alpine:3.20 && podbox images --format '{{.Digest}}' public.ecr.aws/docker/library/alpine:3.20 | grep -qx "$(docker image inspect public.ecr.aws/docker/library/alpine:3.20 --format '{{index .RepoDigests 0}}' | cut -d@ -f2)"`

**Done 2026-09-08**, against a script rather than a recollection:
`experiments/150-image-acquisition.sh` exits 0 and carries the `Prove` above as
its clause 1.

```
podbox  sha256:28bd5fe8b56d1bd048e5babf5b10710ebe0bae67db86916198a6eec434943f8b
docker  sha256:28bd5fe8b56d1bd048e5babf5b10710ebe0bae67db86916198a6eec434943f8b
```

`podbox pull`, `images`, `image ls`, `rmi`, `image rm`, `tag`, `image prune` and
`inspect` are implemented. [image.md](image.md) T-0201 and T-0202 are `done`;
T-0203 and T-0204 are `partial`, each with exactly one half left and each naming
the milestone it lands in: T-0203's second call site is before an extraction
that does not exist until M2, and T-0204's `Prove` holds its lock with
`podbox run -d`, which is M3.

⚠ **A moving tag is a race, and the script handles it rather than ignoring it.**
`alpine:latest` can be republished between podbox's pull and docker's, and the
two digests would then differ for a reason that is not podbox's. Clause 1
re-pulls both once on a mismatch and reports which of the two it was; it did not
have to on this run.

⭐ **The sweep stopped being inert.** `crates/podbox-image` is the first member
to take a `[workspace.dependencies]` pin, and the artefact moved from 496,184 to
**2,130,672 bytes**, a delta of **+1,634,488** against
`experiments/results/bloat-baseline.txt`, with 5,869,328 bytes of headroom under
the ceiling and still no `PT_INTERP`. `experiments/results/bloat-image.txt` is
the reading.

---

### T-1103 M2 extraction that survives the ownership wall

Source:      `TOOL.md` section 5 M2
Category:    milestones
Priority:    P0
Effort:      L
Status:      done 2026-09-09

Problem:     The single highest-risk component. Four separate tools in the
             corpus stop here.
Premise:     ⭐ Measured, for three of the four acceptance criteria.
             `experiments/results/whiteout-contract.txt` establishes that
             `alpine`'s `etc/shadow` is gid 42, that `voidlinux-musl` ships
             `var/cache/xbps` as an absolute self-referential symlink in layer 1
             and whiteouts it in layer 2, and that a slash-anchored whiteout
             selector misses a layer-root whiteout. The work is
             [extract.md](extract.md) T-0301 through T-0307.
Approach:    The four acceptance criteria, in order, and each is a separate
             failure mode rather than a variation of one.
Decision:    In-process extraction at entry level. Shelling out to `tar` is what
             makes this wall reach five tools instead of one.
Prove:       `./experiments/70-whiteout-contract.sh` exits 0; `./experiments/220-extract-path-safety.sh` exits 0; `podbox pull public.ecr.aws/docker/library/alpine:3.20 && podbox extract public.ecr.aws/docker/library/alpine:3.20 && test -f "$(podbox inspect --format '{{.RootfsPath}}' public.ecr.aws/docker/library/alpine:3.20)/etc/shadow"`; `podbox pull ghcr.io/void-linux/void-musl:latest && podbox extract ghcr.io/void-linux/void-musl:latest && ! test -L "$(podbox inspect --format '{{.RootfsPath}}' ghcr.io/void-linux/void-musl:latest)/var/cache/xbps"`; then, when M3 lands, the same two images under `podbox run --rm`

**Done 2026-09-09.** Every clause below ran and matched: the first four with
`crates/podbox-extract` and the `extract` verb on 2026-09-08, the last two
under `run --rm` on 2026-09-09. [extract.md](extract.md) T-0301
to T-0307 are all `done`.

| clause | reading |
| --- | --- |
| `70-whiteout-contract.sh` | exits 0 |
| `220-extract-path-safety.sh` | exits 0, six checks |
| `alpine` has `etc/shadow` | present, 515 entries, the one gid-42 row recorded |
| `voidlinux` has no `var/cache/xbps` link | gone, whiteouted by layer 2; 543 other symlinks survive |

⭐ **CLOSED 2026-09-09, AND THE LAST TWO CLAUSES RAN UNDER `run --rm`.**

| clause, from inside the container | reading |
| --- | --- |
| `podbox run --rm alpine:latest sh -c 'test -f /etc/shadow'` | exit 0, `shadow-present` |
| `podbox run --rm voidlinux/voidlinux-musl:latest sh -c 'test -L /var/cache/xbps'` | not a link |

⭐ **And running from inside showed the ownership wall for the first time.**
`ls -ln /etc/shadow` inside the container reports **gid 0**, not the 42 the image
declares. That is [extract.md](extract.md) T-0302 working exactly as designed
and now visible from the payload's own side: podbox could not apply the id, it
did not pretend to, and `.meta.jsonl` beside the rootfs carries what the image
meant. ⚠ A caller who needs the real gid needs M6's interposer, and until then
the sidecar is the honest answer rather than the convenient one.

⚠ **What follows was true when this entry was `partial` and is kept** because it
is why the `Prove` has the shape it has.

⛔ **THIS ENTRY COULD NOT CLOSE UNTIL M3, AND THAT WAS KNOWN BEFORE M2 STARTED.**
The `Prove` as authored ran `podbox run --rm`, which is [T-1104](milestones.md).
M2 can implement and drive extraction and cannot enter the tree it produced, in
exactly the way [T-0107](probe.md), [T-0108](probe.md) and [T-0204](image.md)
were `partial` then. What was left was one line: re-running the two image
clauses under `run --rm` instead of against the extracted rootfs, and closing
this entry.

⚠ **THE `Prove` WAS REWRITTEN, AND NOT ONLY TO REMOVE `run`.** As authored it
was one `&&` chain of six commands, and that shape cannot report what this
milestone needs:

1. ⛔ **it collapses the third state.** `70-whiteout-contract.sh` exits **2**
   when it cannot run,  no docker, no network,  and in an `&&` chain a 2 stops
   the chain and reads as a failure. `AGENTS.md` absolute 4 makes "could
   not run" a state of its own precisely so it never reads as "ran and did not
   match";
2. ⛔ **the composite status names no clause.** `podbox` exits 125 on a runtime
   failure and 2 on invalid input, an experiment exits 2 for "could not run",
   and the chain reports one number for all six. A reader cannot tell which
   link produced it, which is the same defect `AGENTS.md` absolute 8
   states for a pipe.

The clauses are therefore separate commands, each read from the process that
produced it. This is the shape [T-1101](milestones.md) already closed against:
one script per question, three-state exit, per-clause verdicts.

⚠ **The experiment was renumbered from `80-` before a line of it was written.**
80 is `experiments/80-interposer-abi.sh`; `experiments/README.md` rules a number
is never reused. [T-1205](gate.md) found this and three more, and the gate now
holds the rule.

---

### T-1104 M3 `run` on the chroot rung

Source:      `TOOL.md` section 5 M3
Category:    milestones
Priority:    P0
Effort:      M
Status:      done 2026-09-09

Problem:     ⭐ This is the product requirement the whole specification exists
             for: an agent that knows docker must need zero new knowledge.
Premise:     Read. The work is [enter.md](enter.md) T-0501 through T-0505 and
             [cli.md](cli.md) T-0801 through T-0806.
Approach:    That exact command, inside the reconstruction, with the mode banner
             on stderr and nothing but the payload's output on stdout.
Decision:    The banner goes to stderr. A banner on stdout corrupts every
             pipeline the payload is in, and the payload's stdout is data.
Prove:       `./experiments/300-run.sh` exits 0. Clause 7 is the command above, with the store pre-pulled outside and staged in

**Done, 2026-09-09.** `experiments/300-run.sh`, **eight clauses, exit 0**, and
the three entries this milestone was `partial` for are closed:
[T-0505](enter.md) is clause 8, and [T-0801](cli.md) and [T-0803](cli.md) are
`experiments/320-cli-contract.sh`. Clause 7 is this entry's own acceptance, run
inside the reconstruction:

| | |
| --- | --- |
| stdout | `hi`, and nothing else |
| exit | 0 |
| rung selected inside the reconstruction | **`chroot`** |
| the same podbox on this host | `namespace` |
| the banner | `this mode does NOT provide: process, network, IPC or mount isolation` |

⭐ **The rung differing between the two is the point.** Every other clause runs
on this host, where `mount(2)` succeeds and podbox selects `namespace`; the
reconstruction is where it is `EPERM` and podbox has to fall to the rung this
milestone is named for. A run that selected `namespace` in both would have
proven nothing about `chroot`.

⚠ **The store is pre-pulled outside and staged in, and clause 7 runs
`--pull never`.** The reconstruction has no CA bundle, so a pull from inside it
fails on an unverifiable certificate. That is a fact about the reconstruction
and not about `run`; acquisition is M1's and `150-image-acquisition.sh` proves
it.

⭐ **The three entries this was `partial` for closed on the same day**, and
each brought its own clause rather than a claim: [T-0505](enter.md) is clause 8
here, and [T-0801](cli.md) and [T-0803](cli.md) are clauses 1 to 5 of
`experiments/320-cli-contract.sh`. [T-0802](cli.md)'s exit codes are clause 2.

⚠ **[T-0506](enter.md) stays `partial` and is NOT this milestone's blocker.**
Its `run` half is clause 5 above; its remaining half is that a `podbox probe`
run inside a foreign-architecture container measures the emulator and does not
yet say so, which is a probe question rather than a `run` one.

---

### T-1105 M4 the lifecycle, twenty times

Source:      `TOOL.md` section 5 M4
Category:    milestones
Priority:    P0
Effort:      L
Status:      done 2026-09-09

Problem:     The prior art's own capture of this lifecycle contains a failed run
             because it decided "running" by sleeping and looking.
Premise:     Read. The work is [supervise.md](supervise.md) T-0601 through
             T-0607.
Approach:    `create`, `start`, `ps`, `logs`, `stop`, `rm`, `exec`, `inspect`,
             `kill`, `wait`, `cp`. The loop is create, detached start, `ps` shows
             it running, `exec` prints a marker, `stop`, `rm`.
Decision:    Twenty consecutive passes, and a single failure fails the milestone.
             Retrying a failure is how a race becomes a published pass.
Prove:       `./experiments/230-lifecycle-loop.sh 20` exits 0

**Done, 2026-09-09.** `./experiments/230-lifecycle-loop.sh 20` reports **20 of
20 consecutive passes** and its three following clauses all run.

⭐ **What is in.** `create`, `start`, `ps`, `logs`, `stop`, `kill`, `wait`, `rm`,
`cp`, and `exec` and `inspect` against a container, plus `run -d` and `--name`.
Running state is launcher state: one detached supervisor per container holds the
payload's pidfd, holds the container's lock, owns a control socket, and is the
only thing that writes `running` or `exited` into
`crates/podbox-supervise/src/table.rs`. Nothing reads `/proc` to decide
membership and nothing sleeps to decide readiness.

⭐ **The acceptance failed three times first, and that is the milestone
working.** 10 of 20, 9 of 20 and 1 of 3, always at `stop`, and the cause was a
real race in `stop`: it connects to the launcher twice, and a container that
stopped fast let the launcher tear its socket down in between.
[T-0602](supervise.md) carries it. ⛔ Nothing was retried to get the pass: the
defect the failure named was fixed and the loop then ran clean.

⚠ Four defects the building of this found, the race above and three more, each recorded where it belongs:
`std::fs::read("/dev/urandom")` has no EOF and allocated 13 GB before the OOM
killer took it ([T-0601](supervise.md)'s crate); a readiness pipe without
`O_CLOEXEC` is inherited through the payload's `execve`, so `run -d` blocked for
exactly as long as the container ran; and `si_status` sits at byte 24 of a
`siginfo_t` and not 20, which reported a SIGTERMed payload as exit 128.

---

### T-1106 M5 environment completion, ten distributions

Source:      `TOOL.md` section 5 M5, section 9
Category:    milestones
Priority:    P1
Effort:      L
Status:      done

Problem:     The layer that makes package managers work. Without it the runtime
             runs `echo` and nothing a user wants.
Premise:     Read. The set is not arbitrary: alpine, debian, ubuntu, archlinux,
             almalinux, rocky, rocky-minimal, fedora, opensuse-leap and
             voidlinux-musl, each chosen because it broke something. The work is
             [complete.md](complete.md) T-0401 through T-0409.
Approach:    Each member installs a C toolchain through its native package
             manager and builds and runs a two-file project, with no
             user-supplied fixups.
             ⚠ **Drive it through T-1203's runner rather than writing a second
             one.** `experiments/125-across-distributions.sh` owns the pinning,
             the `no-pull` row, the exit-2-when-nothing-ran rule and the
             reference qualification; this milestone supplies the subject
             script and the row list. Two runners drift, and the one that
             drifts is the one nobody is looking at.
Decision:    A C toolchain rather than a trivial package. It exercises the
             ownership, the resolver, the keyring and the sandbox user at once,
             and a two-file project catches a toolchain that installed and
             cannot link.
Prove:       `./experiments/240-distro-sweep.sh` exits 0


**Done 2026-09-09. TEN of the ten rows install a C toolchain through their own
package manager and build and run a two-file program with it.** ⚠ It was 8 of 10
earlier the same day, and what closed the last two is one mechanism rather than
two fixes: [T-0412](complete.md) gave the completion layer a way to run a COMMAND
inside the rootfs, and both remaining rows needed the same one.

`experiments/240-distro-sweep.sh`. ⛔ The mechanics are T-1203's, not a second
runner: `scripts/common/distro-matrix.sh` holds the pinning, the `no-pull` row,
the exit-2-when-nothing-ran rule and the reference qualification, and
`125-across-distributions.sh` sources the same file. This entry supplies the
subject and the row list.

⛔ **NO DOCKER HUB.** `ghcr.io`, `public.ecr.aws` and the distributions' own
registries, all ten pinned by manifest digest on 2026-09-09.

```
ROW            LIBC     PM            INSTALL   BUILD   RAN
alpine         musl     apk           0         0       42
debian         glibc    apt-get       0         0       42
ubuntu         glibc    apt-get       0         0       42
archlinux      glibc    pacman        0         0       42
almalinux      glibc    dnf           0         0       42
rocky          glibc    dnf           0         0       42
rocky-minimal  glibc    microdnf      0         0       42
fedora         glibc    dnf           0         0       42
opensuse-leap  glibc    zypper        0         0       42
voidlinux-musl musl     xbps-install  0         0       42
```

⭐ **A ROW THAT FAILS IS RUN AGAIN UNDER DOCKER, and that control is what turned
the last two failures into two different answers.** No row needs it now --
`host_not_runtime` is 0 -- and the control stays because a future failure needs
the same discrimination.

⭐ **BOTH of the last two rows were ONE hash-indexed CApath**, and that was not
obvious from either. `libzypp` hands libcurl a `CURLOPT_CAPATH` of
`/etc/ssl/certs`, and `xbps`'s libfetch reads the same kind of directory: a
store OpenSSL looks a certificate up in by the SHA-1 of its canonical DER
subject, so every CAfile [T-0407](complete.md) writes is invisible to both.
[T-0412](complete.md) writes one PEM per root into it and asks the caller to run
`openssl rehash`, and both rows went to 42 in the same change.

⚠ **`voidlinux-musl` had read `host` and it was podbox's after all.** docker
failed on the same image with the same `SSL_connect returned 1`, which is a true
reading and a misleading one: docker has no CA fixup at all on this
TLS-intercepting machine, so it fails there for a reason podbox had already
solved for every CAfile. ⛔ The control tells "podbox is missing a fixup" from
"this machine cannot do it either"; it does not tell either from "both, for
different reasons".

⭐ **What the sweep found, which is the reason for a matrix rather than a smoke
test.** Each of these passed on some rows and failed on others, and a one-image
test would have found none of them:

| finding | seen on | not on |
| --- | --- | --- |
| an image config with no `PATH`, so gcc could not find `cc1` | rocky, rocky-minimal | almalinux, the same gcc |
| `./` as a tar member, refusing the whole layer | every debian-family image | alpine, arch |
| `/etc/ssl/certs` a symlink, so the CA fixup could not write | opensuse | everything else |
| `/lib` a symlink, so the libc probe read `unknown` | void | everything else |
| a CAfile at a path the TLS stack does not read | void, opensuse | everything else |

⚠ **One row was lost to the LINK rather than to a distribution, twice**, and
that is [T-0214](image.md): `fedora` reported `no-pull` with its blob body cut
off at 2,097,153 bytes, because the bounded retry was around the request and the
body is streamed past it. The retry moved down a level and the row pulls.

Prove, run 2026-09-09:

```
$ ./experiments/240-distro-sweep.sh
  rows 10, ran 10, no_pull 0, harness_failed 0
  built_and_ran 10, host_not_runtime 0
  exit 0
```

---

### T-1107 M6 the interposer

Source:      `TOOL.md` section 5 M6
Category:    milestones
Priority:    P1
Effort:      L
Status:      done 2026-09-22

Problem:     The measured gap. A stock path interposer delivers a bind view with
             `mount(2)` denied and leaves `chown 0:42` at `EINVAL`, identically
             to the bare call.
Premise:     Measured, and the reason is at file and line:
             `references/VHSgunzo__pathmap/tree/path-mapping.c:1240-1241` routes
             `chown` through the path-rewriting macro and passes the ids
             through untouched. The work is [interpose.md](interpose.md) T-0701
             through T-0708.
Approach:    Both halves in one object: path virtualization, and ownership
             virtualization that clears the wall stopping the most tools.
Decision:    Both, or the milestone is not done. ⭐ No project in the corpus does
             both halves in one interposer while also speaking docker's CLI.
             That gap is what podbox is.
Prove:       `podbox run --rm public.ecr.aws/docker/library/alpine:3.20 sh -c 'touch /tmp/f && chown 0:42 /tmp/f && stat -c %u:%g /tmp/f' | grep -qx '0:42'` and `podbox run --rm -e 'PODBOX_MAPS=/mapped:/etc' public.ecr.aws/docker/library/alpine:3.20 sh -c 'cd /mapped && test "$(pwd)" = /mapped'`

**Done 2026-09-22.** The work is T-0701 through T-0708, all closed, and
this close drives both halves end to end on host podman 6.1.2 with the
shipped binary: `chown 0:42` reads back `0:42` through the ownership
memo, and a mapped path reads back virtual through `pwd`. No project
in the corpus does both halves in one interposer while also speaking
docker's CLI, which is the gap this milestone names.

⛔ **The `Prove` above is amended: its second half passed `-v`, which
podbox refuses.** The refusal names a copy pretending to be a mount.
Maps travel through `PODBOX_MAPS`, which is what the amended half
drives; the first half gains the `touch` its `chown` needs.

---

### T-1108 M7 packaging

Source:      `TOOL.md` section 5 M7
Category:    milestones
Priority:    P2
Effort:      M
Status:      done

Problem:     The binary has to run on the target with no libraries present.
Premise:     ⭐ Measured on the skeleton already: T-1001 is `done` and its
             `Prove` exits 0. What remains is holding it once dependencies land,
             which is what T-0910's ceiling is for.
Approach:    A single static binary, optionally one file with an embedded
             rootfs. The launch ladder is T-1003.
Decision:    Keep T-1001 and this entry separate. T-1001 is the property of the
             skeleton and is measurable now; this is the property of the shipped
             artefact and is not.
Prove:       `readelf -l target/x86_64-unknown-linux-musl/release/podbox | grep -c INTERP | grep -qx 0 && ./experiments/20-enter-target.sh --stage ./target/x86_64-unknown-linux-musl/release/podbox -- /workspace/podbox version`

**Done 2026-09-21.** Both halves, on the shipped artefact with both
interposers embedded. `readelf -l` reports no `PT_INTERP` (`INTERP count:
0`, 3,367,792 bytes, static-pie), and the staged binary runs
`/workspace/podbox version` as `podbox 0.1.0`, exit 0, inside the N+F+M
reconstruction on host podman.

Half 1 ran in a Linux job on this tree at `362aba4` (musl release build
with the embedded objects). Half 2 staged `.dev/artifacts-build/podbox`,
built earlier today: only `TODO/` and `scripts/` moved since, none of them
a build input, so the bytes are the same build the job measured (both
3,367,792, both `podbox 0.1.0`).

⚠ The embedded-rootfs half stays with [T-1003](packaging.md), which is open.
The `Prove` as written is satisfied.

---

### T-1109 The negative tests, which are tests

Source:      `TOOL.md` section 9
Category:    milestones
Priority:    P1
Effort:      M
Status:      done

Problem:     Every honesty rule in section 4.1 and section 6.8 is unenforced until something
             asserts that the refusal happens. A rule that is only prose is a
             rule that regresses silently, which is the failure mode the whole
             design is built against.
Premise:     Read. `TOOL.md` section 9 names three: `--network=none` must **fail** with
             a named reason; `-v ...:ro` must be **rejected**; a Go payload under
             `interpose` must be **declined** rather than silently unvirtualized.
Approach:    Assert all three, plus the two this corpus adds:
             `--strict` turns every Degraded and Stub into a refusal (T-0804),
             and `-t` is a named refusal where `/dev/ptmx` is absent (T-0503).
             ⛔ `experiments/30-attribution-census.sh` must exit 0 or 2, never 1.
             A 1 means the runtime moved or a probe stopped discriminating, and
             either is a finding rather than a flake.
Decision:    Negative tests live in `experiments/` with the positive ones and
             share their exit-code convention, rather than in a separate suite.
             A refusal that regresses is the same class of defect as a feature
             that regresses, and splitting them makes one of the two easier to
             skip.
Prove:       `./experiments/250-negative-tests.sh` exits 0

**Done 2026-09-25.** `./experiments/250-negative-tests.sh` exits 0 under the
cover `experiments/251-tty-refusal-no-ptmx.sh` builds: a private mount
namespace (`unshare --user --map-root-user --mount`) where an empty
directory covers `/dev/pts`, because `/dev/ptmx` is a symlink to
`pts/ptmx` and a directory binds onto a directory only. The probe reads
`ptmx.usable: false` there; the focused drive refuses `run -t` at
`rc=125` naming `cannot allocate a pty`; the full 250 then runs with
zero FAILs. The census stays in the third state (`rc=2`), which the
entry's own rule allows; the repeat names the cause: `SKIP: the
reconstruction produced no census`, and the host kernel carries no
Landlock (`landlock on host no`), so the M rows stay unattested here.
Driven twice in the lane on kernel
`7.2.0-WSL2-STABLE` with the lane-built `0.1.0-beta.7` binary (both
interposer objects built before it in the same job). Reports: `experiments/results/negative-tests.txt`
(renewed, clause 3 now the refusal arm) and
`experiments/results/tty-refusal-no-ptmx.txt` (the cover, the focused
verdict, the 250 exit).

**Partial, 2026-09-09.** `experiments/250-negative-tests.sh` drives eleven
refusals through the shipped binary and exits **2**, because three of them
cannot be measured on this machine and one of those needs M6.

**2026-09-22.** Re-driven in the lane on a binary with both interpose
objects: the Go clause now measures green (declined, `rc=126`, the
`Go build markers` reason named), so condition 1 below is closed. The
step clause failed exactly as committed until the cause was measured:
T-0412 proposes its rehash step only where the machine announces a CA
bundle, and the lane announces none, so `--strict` named no step by
design rather than by defect. `250` now announces a bundle the way 240
does for its driver rows (system file first, `apt-get` install as
fallback) and skips the clause by name where none exists. Re-driven:
step reasons 1 with the `openssl rehash` command named, zero FAILs,
exit 2 on the two environmental skips. The `-t` and census conditions
stand as recorded. Re-driven again the same day on the T-0413-tree
binary: a byte-identical report, zero FAILs, exit 2 on the same two
skips, so the banner addition changes nothing 250 can see.

⛔ **Every clause asserts TWO things and the second is the one that rots**: the
exit code, read from the process that produced it, and that the message NAMES
the reason. "unknown option" where the parity table has a reason is a regression
even though the code is unchanged.

What ran and held:

```
  run --network=none                 rc=125  named: "no network namespace to select"
  run -v host:/mapped:ro             rc=125  named: "a copy pretending to be a mount"
  run -t refused by name              rc=125  named: "cannot allocate a pty" (closed 2026-09-25, under the 251 cover)
  go payload declined                rc=126  named: "Go build markers" (closed 2026-09-22)
  strict-against-step                rc=125  1 step named (closed 2026-09-22)
  run --strict                       rc=125  5 reasons listed, each on its own line
  the same run without --strict      rc=0
  an unlisted flag                   rc=125  named: "no row in the parity table"
  a None flag names its status       rc=125  named: "status None"
  a None VERB names its reason       rc=125  named the row's own note
  pull http://…                      rc=1    named "HTTPS only", and NOT 124
  30-attribution-census.sh           rc=2    the third state, never 1
```

⚠ **One clause stays in the third state on this machine, and it says so rather
than passing quietly**: the attribution census: `30-attribution-census.sh`
exits 2 here. The repeat names the cause: the reconstruction produces no
census in the lane job container, and the host kernel carries no Landlock,
so the M rows stay unattested here. The entry's rule allows 0 or 2, never 1.

⛔ The `pull http://` clause runs under a `timeout` and asserts the code is not
124, because the failure that refusal exists to prevent is a **hang** rather
than an error: a clause with no bound would pass by hanging.


---

### T-1110 M6's acceptance: a payload the interposer is the only reason works

Source:      `TOOL.md` section 9; T-1106's shape
Category:    milestones
Priority:    P0
Effort:      L
Status:      done

Problem:     M5 has a ten-row matrix that installs a toolchain and builds a
             program, and it found five defects a one-image test would not.
             M6 has an object that clears the ownership wall in a container
             driven by hand and **no acceptance at all**. ⛔ Clause 1 of
             `experiments/250-negative-tests.sh` prints `SKIPPED: M6 has no
             interposer yet` and is the reason that script exits 2.
Premise:     Measured, and each is a row the acceptance has to cover:
             `experiments/results/interpose-ownership.txt` -- the object clears
             the wall under glibc and musl, and refuses the pair the loader
             refuses. `experiments/results/interposer-abi.txt` -- a musl object
             in a glibc payload is `invalid ELF header`, a glibc object in a
             musl payload is `__snprintf_chk: symbol not found`, and a
             `GLIBC_2.34` import against a 2.31 libc is refused by version.
             ⚠ Those are the three ways the wrong object is chosen, so the
             matrix must contain a row for each rather than one happy path.
             ⚠ A **Go** payload is the one clause `TOOL.md` section 9 names,
             and it is the hard case on purpose: a static Go binary makes raw
             syscalls and `LD_PRELOAD` never sees them, so the honest outcome
             is a NAMED DECLINE rather than a silent no-op. T-0706 owns that
             refusal and this is what drives it.
Approach:    A sweep in `experiments/240-distro-sweep.sh`'s shape, over
             `scripts/common/distro-matrix.sh`'s rows, asking one question per
             row: does a payload that needs virtualized ownership succeed, and
             does podbox pick the right object without being told?
             1. per row: extract, let podbox select and place the object, run a
                payload that `chown`s and reads back;
             2. per row: assert the SELECTION, not just the outcome -- a musl
                row must have been given the musl object, which
                `podbox system abi` can be asked about directly;
             3. the three refusals above, each driven deliberately;
             4. the Go clause, asserting a named decline.
             ⛔ The docker control is what makes a failure attributable, as it
             was for M5: this machine intercepts TLS and a row that fails under
             both is the machine's.
             ⚠ No quota-bearing registry: ghcr.io, public.ecr.aws and the
             distributions' own, which `distro-matrix.sh` already holds.
Decision:    The sweep is the shape the Approach describes, run on
             2026-09-18: ten rows, one subject, transcripts per row.
Prove:       `./experiments/245-interpose-sweep.sh`, printing `rows`, `ran`,
             `virtualized`, `declined` and `host_not_runtime`, and clause 1 of
             `./experiments/250-negative-tests.sh` no longer printing SKIPPED.


**Done 2026-09-18.** `experiments/results/interpose-sweep.txt` carries the
run: 10 rows, 10 ran, 10 virtualized, 20 declined, 0 host_not_runtime, with
per-row transcripts in `experiments/results/sweep245/`. Every row preloads
the object of its own libc (asserted on bytes, not inferred), every static
and Go victim is declined by name, and both cross-libc refusals fire
through `podbox system abi`. The version refusal has no matrix row old
enough and stays unit-covered, stated in the script rather than invented
for. Clause 1 of `250` is green (`go payload declined rc=126`, naming Go
build markers); the script's other clauses are untouched, and clause 2
stays red on image content this session did not ship (recorded below, owned
by its own entry, and outside this `Prove`).

⭐ **Two readers fell out of the matrix.** Debian keeps its loader in
`/lib64` and its library in `/lib/<triplet>/`, and void links its loader
at an absolute path whose target exists only in the chroot; both declined
every payload until the finder learned a bounded structural search and
in-root symlink resolution. The void row is what proved the decline
reason gate: it refused an unknown decline rather than waving it through,
and the transcript named the missing file.


---

### T-1111 M8 the nix acceptance: a real payload the chroot tier is exactly the answer for

Source:      `references/talaria0101__nix-experiment/tree/REPORT.md`, plus that
             tree's latest-nix-shim design document and its claim-scope review
Category:    milestones
Priority:    P1
Effort:      L
Status:      done

Problem:     M5 drives package managers across ten distributions and M6 drives
             the interposer. ⛔ **Neither drives a payload that a consumer
             actually wanted and could not otherwise have**, which is the only
             evidence that says podbox is worth running rather than correct.
             ⭐ **This is M8**, and it sits after M7 on purpose: it drives the
             SHIPPED binary, so it is the last gate rather than an early one.
Premise:     ⭐ **Somebody already did this by hand, on a runtime of this class,
             and the answer they arrived at is what podbox is.** Every route with
             a namespace, a mount or a `ptrace` in it failed: a portable
             launcher wanted a pty, `bwrap` wanted `mount`, `proot` wanted
             `ptrace`, and the user-namespace route wanted `unshare`. The
             working solution was a plain `chroot` over a hand-assembled rootfs
             with regular-file device stand-ins, a host CA bundle and
             ownership-neutral extraction.
             ⭐ **podbox already ships every one of those pieces.**
             `crates/podbox-complete/src/devices.rs:50` fills `/dev/urandom`
             with `FILLED_BYTES`, one MiB read once from the host's
             `getrandom(2)`. [complete.md](complete.md) T-0407 is the CA bundle,
             [extract.md](extract.md) is the ownership sidecar, and
             [enter.md](enter.md) is the root change.
             ⚠ **The two entropy fixes are not the same fix, and the difference
             is worth keeping.** That experiment wrote **4 KiB copied from the
             host's `/dev/urandom`**, reached by failure after an *empty* file
             made `std::random_device` in gcc-7.3's libstdc++ throw during the
             first download. podbox's is larger and comes from `getrandom(2)`,
             which is the better source. ⛔ An `LD_PRELOAD` entropy shim was
             tried there and did **not** work: libstdc++ opens the file through
             `syscall()` and no libc interposer sees that. The file is the fix,
             and that is wall 3 appearing in a real payload.
             ⚠ **Two pieces are NOT in place**, and they are what this entry
             measures: no procfs inside the chroot
             ([complete.md](complete.md) T-0413), and no pty at all where
             `/dev/ptmx` is absent ([enter.md](enter.md) T-0503). ⛔ The second
             is decisive for this payload: a pipe-era build tool works and a
             pty-era one cannot, on any runtime with no `/dev/ptmx`.
Approach:    Drive the whole pipeline through the shipped binary and record what
             podbox needed that the hand-built rootfs needed: fetch over TLS,
             evaluate, build locally, and run the artefact.
             ⛔ **Unpatched, or a named decline.** The value of this acceptance is
             that it either runs with no hand patching or it says exactly which
             piece is missing. A row that passes because the operator
             pre-assembled the rootfs measures nothing.
             The acceptance rows, and each is a separate verdict:
             1. **register** the binary-tarball closure in the package database;
             2. **fetch** the package set over TLS, which needs the CA bundle
                and DNS;
             3. **evaluate** it;
             4. **build** locally with the sandbox setting off, which is the row
                the pipe-era tool is pinned for;
             5. **run** the built artefact inside the root;
             6. ⛔ the **negative row**: a process asking for a namespace
                must get an honest refusal, never a silent non-isolation.
                nix's own sandbox is the wrong probe: as root without build
                users it clones namespaces this chroot grants, and plain
                hello substitutes without building at all, so both outcomes
                read wrong (measured 2026-09-21: the forced-local sandboxed
                build exits 0 while every raw `unshare` shape fails `EPERM`
                with and without the preload). Raw `unshare -Urm` is the
                request itself: it must fail naming the wall where
                namespaces are refused, and succeed where the kernel grants
                them. On that host the payload's own message is `setgroups
                failed: Operation not permitted`, and [cli.md](cli.md) T-0809
                is what makes podbox's version of it unambiguous;
             7. ⚠ the **procfs row**: six of the package set's fixup hooks use
                bash process substitution in live code
                (`audit-blas.sh`, `audit-tmpdir.sh`, `canonicalize-jars.sh`,
                `make-symlinks-relative.sh`, `patch-shebangs.sh`,
                `separate-debug-info.sh`, measured 2026-09-21 in the 22.05
                tree). Row 4 disables four of them and says so; the other
                three never fire fatally for hello. The row pins the six
                names, so a seventh cannot arrive silently. T-0413 stays the
                general procfs piece; this pipeline does not need it.
Decision:    ⭐ **Pin the known-good pair.** The acceptance pins the build tool
             and the package set to the last pair whose whole pipeline runs with
             no pty, because that measures **podbox** against a target known to
             be reachable. Tracking the current pair measures the wall instead,
             fails by design until T-0503 has an answer, and tells a reader
             nothing about whether podbox regressed.
             ⚠ **Rejected:** tracking the current pair. It is the more honest
             test of the environment and the less useful test of this tool, and
             the wall it would measure is already T-0503's subject.
             ⭐ The pin is forced by two upstream gates, both verified at the
             source: the build tool calls `posix_openpt()` unconditionally in
             `startBuilder()` from 2.3.0 onward, with no setting to disable it;
             and the package set gates on a minimum tool version, `"2.2"` at
             22.05 and `"2.3"` at 22.11, while 23.11 additionally needs a
             builtin that arrived in 2.4. ⛔ So **2.2.2 with 22.05** is the
             newest release pair, and it is a pin with a reason rather than a
             preference.
Prove:       `./experiments/152-nix-acceptance.sh` exits 0 with the built artefact's own output, or exits 1 naming the single missing piece. ⛔ It may not exit 0 against a pre-assembled rootfs

**Done 2026-09-21.** Seven rows green through the shipped binary on host
podman 6.1.2 (`experiments/152-nix-acceptance.sh`,
`experiments/results/nix-acceptance.txt`,
`experiments/results/nix-acceptance-transcript.txt`). The binary is the
T-1312-fixed build, staged into the driver: the unpack wall was T-1311 and
the symbol wall was T-1312, and both are fixed underneath this run. The
forced-local hello compiles in the root and prints `Hello, world!`; the
negative row drives raw `unshare -Urm` and reads the refusal by name; the
procfs row pins the six fixup hooks that use process substitution.

| row | verdict |
| --- | --- |
| REGISTER | ok, closure registered |
| FETCH | ok, `/nix/store/di36mqc6y19ivaa4qjrb2l82c6dqg7m3-source` |
| EVAL | ok, `22.05pre-git` |
| BUILD | ok, forced-local hello `2.12` |
| RUN | ok, `Hello, world!` |
| NEGATIVE | ok, `unshare: unshare failed: Operation not permitted` |
| PROCHOOKS | ok, the six names |

---

### T-1112 A disposable guest that is not Linux

Source:      winquick at 095dd47; experiments 369, 370, 371 and 392
Category:    milestones
Priority:    P1
Effort:      L
Status:      partial

Problem:     The machine tier needs a disposable non-Linux guest with
             measured command output, exit status, and cleanup.
Premise:     The first design considered an unpublished Windows image.
             Current code has both DOS and disk guest routes. The licensed
             ValidationOS input is supplied explicitly and is not shipped.
Approach:    Keep the shared mailbox and QMP driver. Prove each guest route
             with a pinned image. Diagnose the current KVM startup and
             setup failures. Finish the ReactOS route and its driver.
Decision:    Keep this entry partial until the ReactOS acceptance passes.
             Historical ValidationOS and DOS results keep their stated
             scope. They do not prove the current failed KVM repetition.
             REVISED 2026-10-02. The operator descoped ReactOS: it is low
             priority and ReactOS is itself a beta operating system, so
             proving podbox against it is proving against a moving
             target. The clause is dropped rather than deferred, because
             "deferred" means it will be picked up later and this will not
             be. What still owns this entry is the KVM leg: one guarded
             run under `--unattended`, which T-1609 and T-1610 have
             unblocked. The entry becomes `done` on that leg and not
             before. Do not read this revision as a claim that a
             non-Linux guest route is proven; the DOS and ValidationOS
             results keep exactly the scope stated above.
Prove:       `sh experiments/369-windows-tcg-dos.sh`,
             `sh experiments/370-windows-guest.sh`, and
             `sh experiments/392-kvm-guest.sh --accept-host-risk
             --unattended --binary BINARY --image IMAGE` return 0 on
             their stated hosts. The ReactOS clause is descoped by the
             operator on 2026-10-02 and is not part of this entry's
             acceptance; see the Decision.

**Partial 2026-09-30.** Earlier tracked results prove FreeDOS under TCG
and ValidationOS under TCG. The saved KVM result from 2026-09-29 proves
the guest version, exit 42, machine dispatch, and the observed emulator
arguments. The audit found its unpublished driver reused number 386 and
named private input paths. Experiment 392 replaces that driver with
explicit inputs and owned scratch. Its current proof is in PROGRESS.

Source defect found 2026-09-30: `podbox_windows::run` piped the emulator's
stdout and stderr and did not read them before exit. An emulator that
writes more than one pipe buffer blocks, and the run reports a guest that
did not power off. The streams now go to null and to `emulator.log` in the
run directory. The unit test `a_noisy_emulator_is_not_blocked_on_its_own_output`
proves it. That defect can explain the exit-42 timeout; no guest run has
confirmed it. See T-1350 for the host failure and the driver guards.

Remaining acceptance: reproduce and correct the current KVM failures,
then prepare a redistributable ReactOS base and seal it. Run the output,
status, base-integrity, and cleanup assertions for each route.
Do not treat a reachable desktop as a command channel.
Earlier landing details are retained in
[the captured entry](../docs/history/audit-before-2026-09-30/TODO/milestones.txt).

### T-1623 Record the 29 KEEP-SHELL rows as the retained set, re-derived from the ledger

Source:      `refactor/recon-c.md:170`; `refactor/06-entries/PLAN.md:61-64`
Category:    milestones
Priority:    P3
Effort:      S
Status:      done

Problem:     29 scripts stay in shell, and the number that reaches a
             reader is wrong. The plan's Round 2 breakdown says 27 and
             its own category list sums to 30, and neither figure was
             re-derived when two scripts were added.
Premise:     The 29 is the most reliable part of the corpus: every row
             was re-examined against one test, whether the script needs
             a host tool, a device or an OS facility. None rests on a
             Rust binary having to shell out, which is not a reason.
Approach:     Re-derive the 29 from the ledger's rows rather than from
             the plan's prose, then record the list with the reason each
             row stays. The per-subject breakdown is the deliverable and
             the plan explicitly does not supply it.
Decision:     Re-derive, do not copy. The plan says its own figure was
             not re-derived, so a copied one would carry the same error.
Prove:       `grep -c "KEEP-SHELL" TODO/milestones.md` is at least 29;
             the recorded list has no id twice and no id absent.

**Done.** The entry records the 29 retained scripts as a per-row table, each
with the host tool, device or OS facility that keeps it in shell and the audit
line the reason is read at. The 29 is re-derived from the ledger rows, not
copied from the plan prose: the shipped ledger counts KEEP-SHELL 27, and the
three numbered corrections move `95` in by VC-3 and `162` in by VC-2. The
`162` row is refuted rather than re-derived, because `TODO/interpose.md:1479` is
the `Prove` of T-1311 and names that script. `grep -c "KEEP-SHELL"
TODO/milestones.md` returns 43, at or above the 29 the Prove sets. The shipped
ledger still counts 27, so T-1622 owns the reconciliation and the two files
disagree by design until it lands.

**The 29 are re-derived from `refactor/06-entries/verdict-ledger.tsv` rows, not
copied from the plan's prose, and the re-derivation returns 29 only after the
three numbered corrections are applied.** The ledger is the plan's own scope
statement: `refactor/06-entries/PLAN.md:22-27` says it is PRE-VC and that three
rows change. Read as shipped,
`awk -F'\t' 'NR>1{c[$3]++} END{for(v in c) print v, c[v]}'` over the ledger
returns `SPLIT 35`, `DELETE 12`, `KEEP-SHELL 27`, `RUST-TOOL 27`, `RUST-TEST
20`. Apply the three corrections and the counts are `KEEP-SHELL 29`, `SPLIT 36`,
`RUST-TEST 18`, `RUST-TOOL 27`, `DELETE 11`, which is the table at
`refactor/06-entries/PLAN.md:40-47`. So the 29 is the 27 the ledger holds plus
the two the corrections move into it: `95` from RUST-TEST by VC-3
(`refactor/06-entries/PLAN.md:22-27`) and `162` from DELETE by VC-2 (`:168-170`).
Neither is in the shipped ledger, so both rows below are marked as corrected.

Every reason below is the audit's own, read at the line the ledger cites. Each
one names a host tool, a device, or an OS facility a Rust binary cannot provide.

| script | ledger verdict | the reason it stays in shell | evidence |
| --- | --- | --- | --- |
| `100-interpose-symbols.sh` | KEEP-SHELL | compiles C from `references/VHSgunzo__pathmap/tree/`, reads the ELF dynamic symbol table with `nm -D --defined-only`, and runs `LD_PRELOAD` interposition arms that need a C compiler and a live kernel | `refactor/01-audit/group-6.md:110` |
| `147-podvm-exec.sh` | KEEP-SHELL | `qemu-system-x86_64`, a pinned kernel fetched over the network, and a `mkfifo` pair the driver holds `O_RDWR` before the emulator starts | `refactor/01-audit/group-3.md:236` |
| `180-registry-fixture.sh` | KEEP-SHELL | the `zot` binary, `/proc/net/route` and `getent hosts` read inside a `--network=none` container, and a host `openssl` certificate with Windows path spellings | `refactor/01-audit/group-6.md:294` |
| `20-enter-target.sh` | KEEP-SHELL | a privileged container for `eng_privrun`, Go binaries built from a read-only corpus, and the fixed three-way relationship with `10` and `130` | `refactor/01-audit/group-3.md:590` |
| `200-registry-auth.sh` | KEEP-SHELL | a live TLS registry on loopback with `htpasswd`, `openssl passwd -5` and `curl --cacert`, and an asserted network isolation | `refactor/01-audit/group-8.md:927` |
| `210-store-concurrency.sh` | KEEP-SHELL | real processes racing a shared filesystem through a network registry, `kill -9` on a running pull, and a prune racing it | `refactor/01-audit/group-10.md:428` |
| `240-distro-sweep.sh` | KEEP-SHELL | a foreign OCI engine as control, through `experiments/lib/engine.sh` | `refactor/01-audit/group-7.md:739` |
| `245-interpose-sweep.sh` | KEEP-SHELL | glibc and musl interposer objects built by the host toolchain, compared byte for byte against the placed objects | `refactor/01-audit/group-4.md:900` |
| `280-insecure-registry.sh` | KEEP-SHELL | two live `registry:2` instances, one plain HTTP and one with a script-generated certificate nothing trusts, reached through a `tcpfwd` forward | `refactor/01-audit/group-10.md:597` |
| `290-microvm.sh` | KEEP-SHELL | `qemu-system-x86_64 -M microvm` and a cpio initramfs read back off a serial console; it measures what a different kernel answers to `landlock_create_ruleset` | `refactor/01-audit/group-6.md:346` |
| `353-open-issue-triage.sh` | KEEP-SHELL | a `wsl-toolkit` lane that bootstraps a toolchain before it can build, and 22 open issues measured against a beta binary on that lane | `refactor/01-audit/group-4.md:984` |
| `357-tcg-profile.sh` | KEEP-SHELL | a kvm-less machine with `qemu-system-x86_64` installed at run time, a real initramfs boot under TCG, and a Windows job wrapper; clause 5 alone is a QEMU boot | `refactor/01-audit/group-10.md:746` |
| `361-guest-usernet.sh` | KEEP-SHELL | `qemu-system-x86_64` with `-netdev user,hostfwd=udp::`, and the forwarding is the subject | `refactor/01-audit/group-7.md:1027` |
| `367-qemu-user-aarch64.sh` | KEEP-SHELL | a live `binfmt_misc` registration for aarch64, which is machine-wide state podbox reads and never writes | `refactor/01-audit/group-7.md:1316` |
| `369-windows-tcg-dos.sh` | KEEP-SHELL | `qemu-system-x86_64` in TCG mode against a real 32 MiB FreeDOS image, with a deadline enforced through a QMP mailbox and the base sha256 compared before and after | `refactor/01-audit/group-4.md:1061` |
| `371-validationos-stream.sh` | KEEP-SHELL | `curl` with byte-range support against an anonymous third-party origin, an `RLIMIT_FSIZE` ceiling, and `qemu-img` | `refactor/01-audit/group-2.md:701` |
| `382-restricted-sshd.sh` | KEEP-SHELL | a real `sshd` and a real `ssh` client; clause 3 exists because a static `sshd` cannot take `LD_PRELOAD` | `refactor/01-audit/group-4.md:1140` |
| `384-windows-lane-v6.sh` | KEEP-SHELL | `wsl-toolkit`, a Windows-only CLI, and its job-id collection contract | `refactor/01-audit/group-2.md:761` |
| `385-kvm-open.sh` | KEEP-SHELL | `wsl-toolkit` and `/dev/kvm` inside that tool's base | `refactor/01-audit/group-10.md:806` |
| `387-mux-two-client.sh` | KEEP-SHELL | a third-party hosted relay over the network, with a real `ssh` client and a real `sshd` per session; the Rust test is the fake, and this is the acceptance gate against the real relay | `refactor/01-audit/group-2.md:809` |
| `391-machine-bridge.sh` | KEEP-SHELL | a C file that must link static against musl, built with `zig cc -target x86_64-linux-musl -static` | `refactor/01-audit/group-9.md:842` |
| `392-kvm-guest.sh` | KEEP-SHELL | `wsl-toolkit --instance podbox base exec`, nested-KVM QEMU on `/dev/kvm`, an OVMF pflash pair, a licensed operator disk, and a QMP socket | `refactor/01-audit/group-8.md:1185` |
| `396-audit-linux.sh` | KEEP-SHELL | a Windows job input that requires `PWD=/work` and a copied container checkout | `refactor/01-audit/group-2.md:988` |
| `400-kvm-cleanup.py` | KEEP-SHELL | `ps -eo pid,args` and `awk` against argv[0] being the emulator name, and the shell matcher is what the real KVM path calls | `refactor/01-audit/group-5.md:1032` |
| `95-podman-vfs-ignorechown.sh` | KEEP-SHELL (VC-3) | a rootful podman machine, podman's own storage option, and a seccomp-filtered `podman unshare` load; the subject is podman's option, not podbox | VC-3, `refactor/06-entries/PLAN.md:22-27`; reason at `refactor/01-audit/group-4.md:498` |
| `targetfs.sh` | KEEP-SHELL | the container's pid 1, calling `mount(2)` and `pivot_root(2)` before any confinement is applied, so nothing here is reachable from inside | `refactor/01-audit/group-3.md:1158` |
| `162-tar-symlink-modes.sh` | KEEP-SHELL (VC-2) | a deployment proof over a real engine on a pinned image digest; DELETE is refuted because `TODO/interpose.md:1479` is the `Prove` of T-1311 and names this script | VC-2, `refactor/06-entries/PLAN.md:168-170`; reason recorded in [T-1628](interpose.md) |
| `gnu-link-stub.sh` | KEEP-SHELL | `rustc`'s `-C linker=` takes a PROGRAM and the object must be the first argument on the link line, which a Rust program cannot express | `refactor/01-audit/group-1.md:953` |
| `zig-ar.sh` | KEEP-SHELL | `zig` itself, named in `.cargo/config.toml` as the archiver for eight targets; the `ar` key takes a program path, not a crate | `refactor/01-audit/group-6.md:884` |

**Two rows carry a correction, and one of the two is refuted rather than
re-derived, so the list is not a copy of the shipped ledger.**
`refactor/06-entries/verdict-ledger.tsv` still reads RUST-TEST for `95` and
DELETE for `162`. The reasons are the plan's at
`refactor/06-entries/PLAN.md:22-27` and `:168-170`. The `162` refutation is
written where a reader of that category finds it, in
[T-1628](interpose.md), because the `Prove` at `TODO/interpose.md:1479` is
what settles it.

⚠ **The shipped ledger still counts 27, and an implementor who reads it lands a
different set than this one.** `refactor/06-entries/verdict-ledger.tsv` is the
file the plan calls PRE-VC at `refactor/06-entries/PLAN.md:22-27`. T-1622 owns
making it post-VC. Until that lands, the two files disagree by design and this
entry is the one that says which is which.

### T-1631 Record the `10`/`20`/`130` irreducible dependency as open, with T-1550

Source:      `refactor/recon-c.md:178`;
             `refactor/06-entries/PLAN.md:174-177`; T-1619
Category:    milestones
Priority:    P2
Effort:      S
Status:      done

Problem:     Three scripts depend on each other and the plan calls the
             dependency irreducible. Nothing in the record says so, so
             the next session reads it as an oversight and deletes one
             of them.
Premise:     Two of the three name the third in live paths, and the
             reduction the plan considered is ruled out for a named
             reason.
Approach:     Record the dependency beside the decision entry that owns
             it, so the two are found together and neither is closed
             without the other.
Decision:     Milestones carries the dependency; the packaging entry
             carries the decision. A dependency is not a decision.
Prove:       `grep -n "10-build-target-image" TODO/milestones.md` names
             the three scripts and the entry that owns the decision.

**Done.** The entry records that `experiments/10-build-target-image.sh`,
`experiments/20-enter-target.sh` and `experiments/130-probe-parity.sh` are one
unit and the dependency is irreducible, so a later session does not delete one
of them. The reduction the plan considered is refuted on disk: two scripts name
the third in live paths, and `experiments/130-probe-parity.sh` names no engine of
its own yet cannot run where `20` cannot. T-1213 carries the measurement and is
`Status: done`. `grep -n "10-build-target-image" TODO/milestones.md` names the
unit at this entry, and also names the owning decision through
[T-1619](packaging.md). The two close together.

⛔ **`experiments/10-build-target-image.sh`, `experiments/20-enter-target.sh`
and `experiments/130-probe-parity.sh` are one unit, and the plan calls the
dependency irreducible. Do not delete one of the three.** The plan's own
correction is conditional, not a free deletion: VC-4 at
`refactor/06-entries/PLAN.md:174-177` reads "DELETE is conditional on `20` and
`130`". Two of the three name the third in live paths, both verified on disk
here: `experiments/20-enter-target.sh:70` prints `SKIP: $IMAGE not built. Run
./experiments/10-build-target-image.sh`, and `experiments/300-run.sh:310` tells
the reader the same. The wave that owns the deletion lists the script and marks
it conditional in its own table: `refactor/06-entries/T-R006.md:67` reads "this
one waits for a separate decision about the three-way dependency. It is listed
so it is not forgotten, not so it can be deleted alongside the other eleven."

**The reduction was considered and does not work, and the reason is measured.**
`experiments/130-probe-parity.sh` names no engine of its own, zero
`docker|podman` lines, and still cannot run where `20` cannot. That is
T-1213's own Problem field at `TODO/gate.md:1124-1129`, and its Decision at
`:1139-1140` reads "The three convert as one unit. Converting `130` without
`20` tests nothing, and converting `20` without `10` builds nothing." T-1213 is
`Status: done`, so the unit is measured, not asserted: its Done record at
`TODO/gate.md:1146-1148` names the host podman build of `10` at image id
`9ed4f5c81452` and a `130` run of 16 matched and 0 differed.

**Milestones carries the dependency; [T-1619](packaging.md) carries the
decision, and neither closes without the other.** T-1619 owns the question and
names this entry's half at `TODO/packaging.md:1082-1087`. The plan's own task
row for that decision sits at `refactor/recon-c.md:137`. `refactor/` is
untracked and its ids have no heading in `TODO/`, so the gate cannot
resolve a plan id printed here, and T-1619 says the same at
`TODO/packaging.md:1154-1163`. The plan row and the tracked entry point at
one decision. A reader who finds either row finds the other.

### T-1637 Record the eleven converged-on-Rust counts and the 29 that stay in shell

Source:      `refactor/recon-c.md:184`;
             `refactor/06-entries/PLAN.md:49-50`
Category:    milestones
Priority:    P2
Effort:      S
Status:      done

Problem:     The corpus has two outcomes and the tree carries one
             number for the whole of it. A reader cannot tell how much
             is finished from how much is deliberately staying.
Premise:     The plan's own arithmetic is 81 retiring plus 11 deletions
             against 29 retained, and it sums to 121. The 81 is a sum
             over three verdicts, so quoting it without the three is a
             number nobody can check.
Approach:     Record the three converging verdicts separately from the
             sum, and the retained count beside them, so each is
             checkable on its own.
Decision:     The breakdown is the deliverable, not the total. A total
             whose parts cannot be re-derived is the defect the audit
             found twice already.
Prove:       `grep -n "KEEP-SHELL" TODO/milestones.md` and the same for
             each converging verdict name the counts that sum to the
             corpus size.

**Done.** The entry records the corpus as two outcomes rather than one number,
with each converging verdict counted on its own: RUST-TOOL 27 and RUST-TEST 18
give the 45 that converge wholly on Rust, SPLIT 36 retires because each has a
Rust half, and the KEEP-SHELL 29 and DELETE 11 rows do not. Those sum to the 121
total, and the ledger holds 122 lines with one header, so the size is
re-derived rather than quoted. The figures are the plan's post-correction table
at `refactor/06-entries/PLAN.md:40-47`; the shipped ledger is PRE-VC and counts
SPLIT 35, DELETE 12, KEEP-SHELL 27, RUST-TEST 20, so T-1622 owns the gap. The
Prove matches: `grep -c` in `TODO/milestones.md` returns KEEP-SHELL 43, SPLIT 9,
RUST-TOOL 7, RUST-TEST 11, DELETE 13, each verdict named in the table below. The
121 counts `experiments/*` and `scripts/*` at depth 1 only; `scripts/common/`
holds 28 more that the gate runs.

**The corpus has two outcomes and the 81 is a sum over three verdicts, so the
three are recorded here and the sum is not the deliverable.** The figures are
copied from the plan's post-correction table at
`refactor/06-entries/PLAN.md:40-47`, counted from the group report bodies after
VC-1 through VC-7, as that table's own lead-in at `:38` states.

| verdict | count | converges on Rust |
| --- | --- | --- |
| KEEP-SHELL | 29 | no. A host tool, device or OS facility. |
| SPLIT | 36 | partly. Every one has a Rust half. |
| RUST-TOOL | 27 | yes. A Rust binary replaces the script. |
| RUST-TEST | 18 | yes. A Rust test replaces the script. |
| DELETE | 11 | no. The measurement is obsolete. |
| **total** | **121** | 45 converge wholly on a Rust test or tool |

**The arithmetic is checkable, and it is the plan's line
`refactor/06-entries/PLAN.md:49-50`.** RUST-TOOL 27 plus RUST-TEST 18 is the 45
converging wholly, which is the total column at `:47`. Adding the 36 SPLIT
rows, each of which has a Rust half, gives the 81 that retire. 81 plus the 29
KEEP-SHELL rows plus the 11 DELETE rows is 121, and `grep -c ''` over
`refactor/06-entries/verdict-ledger.tsv` returns 122 lines, one of them the
header. So the corpus size is re-derivable from the ledger, not only quoted.

**The 29 that stay in shell are recorded per row in [T-1623](#t-1623-record-the-29-keep-shell-rows-as-the-retained-set-re-derived-from-the-ledger),
with the host dependency each one rests on.** The plan says those verdicts are
the most reliable part of the corpus, at
`refactor/06-entries/PLAN.md:52-59`, and that none of them rests on "a Rust
binary would have to shell out", which is not a reason.

⚠ **The ledger is PRE-VC and the table is POST-VC, so a reader counting the
ledger gets a different split for two rows and is not wrong about the file it
read.** `refactor/06-entries/PLAN.md:22-27` says so explicitly and names the
three rows: `95` RUST-TEST to KEEP-SHELL, `151` RUST-TEST to SPLIT, and `162`
DELETE to RETAIN. Counted from the shipped
`refactor/06-entries/verdict-ledger.tsv`, the figures are SPLIT 35, DELETE 12,
KEEP-SHELL 27, RUST-TOOL 27, RUST-TEST 20, which is 121 rows either way. The
difference is exactly the three corrections. T-1622 owns reconciling the ledger
with the seven entries, so this entry names the gap rather than repeating the
ledger. The same warning is on [T-1623](#t-1623-record-the-29-keep-shell-rows-as-the-retained-set-re-derived-from-the-ledger),
where the retained list is re-derived.

**The 121 figure is scoped to top-level files, and the whole tree carries more.**
`refactor/06-entries/PLAN.md:29-34` states the boundary: the 121 counts
`experiments/*` and `scripts/*` at depth 1 only, `scripts/common/` holds 28 more
scripts the gate runs at `gate.yml:59`, and a whole-tree count returns 167 only
because it also sweeps the local Python bytecode cache that
`.gitignore:123-125` ignores. Retiring the 121 must not delete
`scripts/common/check-*.sh`, which are the gate's checks rather than
measurements. The three rows counted here carry no such file, so the 81 does not
reach the gate's own checks.

