# milestones

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
             because `scripts/check-todo.py` resolves every cited path and line
             and cannot do so into a branch it is not on.
             [reference-map.md](reference-map.md) records the choice and what a
             clone pays for it.
Prove:       `./scripts/check-todo.py && cargo build --release --target x86_64-unknown-linux-musl && readelf -l target/x86_64-unknown-linux-musl/release/podbox | grep -c INTERP | grep -qx 0`

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

Source:      `https://github.com/carlbomsdata/winquick`; [podvm.md](podvm.md)
Category:    milestones
Priority:    P3
Effort:      L
Status:      blocked 2026-09-26

Problem:     podbox turns an OCI reference into a process. Every rung it has
             assumes the payload is Linux, because every rung except the machine
             tier shares the host kernel. ⭐ **The machine tier does not**, and
             that is the one place where a guest of another operating system is a
             possibility rather than a category error.
Premise:     ⚠ **Recorded as a direction, and nothing here is measured.** An
             existing tool runs disposable Windows guests under an emulator on
             one host architecture, keeps a base image and its caches between
             runs, discards the guest's own writes, and returns the guest
             command's output streams and exit code unchanged. That contract is
             the one [podvm.md](podvm.md) T-1304 specifies for a Linux guest.
             ⚠ Its own page says the host support is one platform and that the
             rest is a plan. ⛔ So what is read here is the SHAPE and not any
             number.
Approach:    Do not start this before [podvm.md](podvm.md) T-1301 through T-1304
             are closed. The exec protocol, the image model and the probe are the
             same work, and doing them twice is how a second parity table
             appears.
             ⚠ What is genuinely new is the platform field. podbox already treats
             the platform as runtime data rather than a compile-time constant,
             which is the invariant in
             [`../docs/architecture.md`](../docs/architecture.md) that makes this
             expressible at all.
Decision:    Not taken, and deliberately so. ⛔ It is recorded here so that it is
             not rediscovered as a new idea, and it is P3 so that it cannot
             displace M6 or M7. [RULES.md](RULES.md) section 5 is why it is open
             rather than absent: nothing closes as out of scope.
             Ruled 2026-09-22: keep parked as P3. The prerequisites T-1301
             through T-1304 are closed, so the entry is schedulable, but it
             displaces nothing and stays open.
             Ruled 2026-09-23: unparked, work it next. It stays P3.
Prove:       `podbox run --platform windows/amd64 IMAGE cmd /c ver` returns the guest's own version string and its exit code, or podbox refuses by name with the leg that is missing

**Done 2026-09-23.** The second arm: a non-Linux guest is refused by name
with the missing leg, on every entry path, before anything is fetched or
entered. `lifecycle::ensure_linux_guest` (`crates/podbox-cli/src/lifecycle.rs`)
admits `linux` and refuses anything else at exit 125, naming the requested
`os/arch`, that every tier runs Linux guests, and the missing leg (no
Windows guest support). One helper, four call sites: `run`/`create`/`run -d`
(`run.rs` `prepare`, requested platform before the fetch and the stored
record after it), `exec` (requested platform before the store opens and the
stored record after it), and `start` (the image record). No Windows guest
runs anywhere: the machine tier boots Linux images and the chroot tier
shares the host kernel, so there is no leg to implement, only the refusal
to name. Pull is untouched per [T-0212](image.md): fetching bytes for
another machine stays allowed; entering them is what refuses.

The gate was watched fail first: `lifecycle::tests::a_non_linux_guest_is_refused_by_name`
against an always-admit stub exits FAILED (`left: Ok(()), right: Err(125)`),
then green after the implementation
(`experiments` lane, `rust:1.98.1-bookworm` through host podman 6.1.2).

```
$ podbox run --platform windows/amd64 any/image:tag cmd /c ver; echo $?
podbox run: windows/amd64 is not a Linux guest. podbox runs Linux guests only: the chroot tier shares the host kernel and the machine tier boots Linux images. No windows guest support exists (TODO/milestones.md T-1112), so there is nothing to pull or enter for this platform
125
```

No registry was contacted: the refusal sits before the fetch, so the
reference above never resolves and no store state changes. `podbox probe`
is unchanged: there is no Windows leg to probe, and the entry's first arm
(a guest version string) arrives with a guest podbox cannot enter.

**Partial, 2026-09-25.** Issue 29 reopens this entry: the refusal arm
holds, the guest arm does not exist. Study order is
`references/carlbomsdata__winquick/tree/docs/architecture.md`, then
`tree/src/platform.rs`, `tree/src/qemu.rs` `boot_command`,
`tree/src/mailbox.rs` with `guest/agent.cmd`, then a release build and
its `doctor` (tree version `0.5.0`, not `0.5.1`). First target is a
Linux KVM Windows guest in that shape (NVMe root overlay, FAT mailbox,
`cmd.exe` agent, `-nic none`, per-run overlay discarded), never TCG:
the reference refuses TCG as a different product, and its Linux guest
path is itself unverified. Prerequisites in order are QEMU 11 or newer
with `qemu-img` (T-1301), KVM opened not stated, UEFI code and vars,
a mount-free setup writer, a licensed image fetch with an accept-terms
gate (never redistributed, never committed), and the T-1301 space and
fsize checks before a guest starts. Keep pull allowed per T-0212.
Fix area is a new `podvm` windows driver beside T-1303 and T-1304,
with the `lifecycle` gate kept as the last resort. Risk if wrong is a
slow TCG guest claimed as equivalent, or a licensed image
redistributed. Prove is the entry's first arm on a KVM host with the
image installed, plus the refusal arm naming the exact missing leg
with nothing fetched or mutated, and a KVM-denying fixture beside the
T-1317 one so the refusal stays driven where KVM is absent.

**Studied 2026-09-26, in the entry's order.** `architecture.md`
(437 lines): qemu as a child process (the GPLv2 boundary), NVMe
root overlay discarded per run, FAT mailbox with the batch agent
through AutoRun, UEFI code with per-run vars, `-nic none`, no
network, no streaming. `platform.rs` (315 lines): Linux x86_64 is
`qemu-system-x86_64`, `-M q35`, `-accel kvm`, `-cpu host`, OVMF
code with distro alt names, vars template with alts, and never
TCG. `qemu.rs` `boot_command`: pflash pair, NVMe root and
mailbox (`cache=writethrough`), ramfb, `-display none`,
`-rtc base=localtime`, `-no-reboot`, serial to file, QMP on a
unix socket. `mailbox.rs` with `guest/agent.cmd`: protocol v1
(`WQMARK`, `WQCMD`, `WQGO` with the run token, `WQOUT`,
`WQERR`, `WQCODE` last), inbox components only.

**Refusal arm hardened 2026-09-26.** The machine tier dispatched
before the platform gate, so a Windows request over
`--podbox-tier=machine` reached the leg refusal and read as a
Linux guest the driver has not arrived for. `run.rs` `prepare`
and `exec` now parse and gate the platform before the tier
dispatch (ladder checks keep their precedence): a Windows
request names `windows/amd64` with the missing support on every
path. Driven by `experiments/362-windows-refusal.sh`, exit 0 on
the lane: KVM denied ENOENT in probe JSON, `run`, machine-tier
`run` and `create` each exit 125 naming `windows/amd64` with
the store byte-identical after, CLI lifecycle+tier units 26
passed. Report in `experiments/results/windows-refusal.txt`.

**Blocked on the guest arm.** No reachable machine holds
`/dev/kvm` (lane ENOENT, measured in the drive above), lane
qemu is 7.2.22 against the prerequisite 11 with `qemu-img`,
and no licensed Windows image is installed anywhere (one is
never fetched or committed here). What unblocks: a KVM host
with the image installed under the accept-terms gate the entry
names. The entry stays partial on that blocker.

**Guest arm landed 2026-09-26, under TCG, in the authoring
sandbox.** New crate `crates/podbox-windows`: `fat16.rs` (an
MBR-partitioned 16 MiB FAT16 volume built and read in process,
so the driver does not shell out to `mkfs.fat` or `mtools`),
`agent.rs` (the two `cmd.exe` scripts and the mailbox protocol),
`plan.rs` (the emulator argv, and the accelerator taken from the
machine tier's own `Profile` so `tcg` is run rather than
refused), and `lib.rs` (`mailbox`, `outcome`, `stage`, `run`,
`provision`). New CLI surface `podbox windows
doctor|setup|run` in `crates/podbox-cli/src/windows.rs`, wired
from `main.rs`, with parity rows and a `docs/code-map.md` row.
`lifecycle::ensure_linux_guest` still refuses a non-Linux guest
on the OCI path, but now routes it by name through
`lifecycle::guest_verb` to `podbox windows run` instead of
claiming no support exists. `prove` is met in spirit by that
verb rather than by `run --platform windows/amd64`, because a
Windows guest is a disk image and not an OCI rootfs.

What the reference's shape could not be ported as written, each
measured against the real image and each the reason for a
divergence: `cmd.exe` `AutoRun` does not fire for the shell
Validation OS starts; a `Run`/`RunOnce` value does not either,
because that logon never reaches `userinit.exe`'s `Run`
processing; `sc create` with a `cmd.exe` image starts the script
and is then terminated by the service control manager once the
process fails to report `SERVICE_RUNNING`, and raising
`ServicesPipeTimeout` to 900000 did not save it; an `onstart`
scheduled task as `SYSTEM` does fire, so that is the autostart.
`mountvol /P` strips the volume's drive letter from the mount
manager's persistent database, so the reference's dismount made
every later boot unable to find the mailbox; the agent no longer
dismounts, and it probes D through Z rather than a fixed letter.

Verified against the real guest under `tcg` on 2026-09-26, both
halves from the crate's own code path: `provision` booted a
fresh overlay, typed the installer through the emulator monitor
(`SETUP.TXT` = `INSTALLED D:`), and the guest powered itself off
so its FAT writes were flushed before the read; then `run` over
the provisioned image, in a fresh disposable overlay, returned
`Microsoft Windows [Version 10.0.26100.9278]`, the command's own
output, an empty stderr, exit code 0 and the matching token in a
28-second boot. `cargo test -p podbox-windows --lib` is 32
passed, run in the sandbox. Four defects were found by review
and fixed, each with a test: an argv-shaped command that quoted
a whole line into one token (`cmd.exe` refused with exit 123,
which the real guest reproduced); a base-image format taken from
the extension, so `.img` would have been passed as `-F img`; a
provisioning boot killed before the guest had flushed `SETUP.TXT`;
and a provisioning install written into a scratch overlay that
`run` then discarded.

⚠ **What this sandbox could not verify, named rather than
implied.** The workspace was not compiled: `cargo check -p
podbox-cli` does not fit in the only directories this sandbox may
execute from (the dependency graph exhausted a 245 MB tmpfs while
still building proc macros), and the shipping target needs `zig`
for `ring`. So the five edited `podbox-cli` files are reviewed
and not compiled here, and `podbox windows doctor|setup|run` has
not been run as a verb. Every claim above is about the
`podbox-windows` crate, which does compile and whose tests do
run. The `kvm` arm of `accel_for` is unit-tested and was not
exercised: no reachable machine has `/dev/kvm`. No licensed image
is fetched, committed or redistributed: the base image existed in
the sandbox already and is not in the tree. Remaining: compile the
CLI on a machine with the full toolchain, run the verb, and take
the `kvm` arm on a KVM host.
