# packaging

Record semantics: [task rules](RULES.md#5-entry-closure).


`TOOL.md` section 3.4, section 6.7 and milestone M7.

[INDEX.md](INDEX.md) is the list and the counts. [PROGRESS.md](PROGRESS.md) is the work order.
[RULES.md](RULES.md) is how an entry closes. [reference-map.md](reference-map.md) is the corpus.

A single static binary. The interposer is the one object in the tree that is
not static, and it is embedded in the launcher rather than shipped beside it.

---

### T-1001 A single static binary with no `PT_INTERP`

Source:      `TOOL.md` section 3.4, section 5 M7
Category:    packaging
Priority:    P0
Effort:      S
Status:      done 2026-09-08

Problem:     The artefact must run on the target with no libraries present, and
             the memfd launch rung needs a static, dependency-free entrypoint.
Premise:     ⭐ **Measured here, on the empty skeleton.** `cargo build --release
             --target x86_64-unknown-linux-musl` succeeds and produces a
             389,656-byte static-PIE. `readelf -l` shows ten program headers and
             no `PT_INTERP`. One machine, one day: kernel `6.18.44-fc-v24`,
             rustc 1.94.1.
             The settings are `.cargo/config.toml:9-10` for `+crt-static` and
             `Cargo.toml:34-39` for the release profile.
Approach:    Hold it. This is the property every dependency decision in
             [deps.md](deps.md) is measured against, and the one a C dependency
             takes away.
             ⚠ A shell entrypoint skips the memfd rung entirely. That is why
             T-0803 installs `docker` and `podman` as symlinks rather than as
             wrapper scripts, and it is recorded in the corpus at
             `references/qaidvoid__onelf/tree/crates/onelf-rt/src/main.rs:133-148`,
             which refuses memfd mode for an entrypoint with shared library
             dependencies and explains that after `exec` nothing can be said
             about it any more.
Decision:    `crt-static` per target rather than under `[build]`. Setting it
             globally also applies it to build scripts and proc macros compiled
             for the host, which is a link failure on some hosts.
Prove:       `cargo build --release --target x86_64-unknown-linux-musl && readelf -l target/x86_64-unknown-linux-musl/release/podbox | grep -c INTERP | grep -qx 0`

**Done on the skeleton, and it stays open to regression rather than to work.**
The `Prove` command above was run on 2026-09-08 and exits 0:

```
$ file target/x86_64-unknown-linux-musl/release/podbox
ELF 64-bit LSB pie executable, x86-64, static-pie linked, stripped
$ readelf -l target/x86_64-unknown-linux-musl/release/podbox | grep -c INTERP
0
$ stat -c%s target/x86_64-unknown-linux-musl/release/podbox
389656
```

T-0910 wires the size into the gate so the property cannot be lost quietly.

---

### T-1002 Embed the interposer as bytes and place it inside the rootfs

Source:      `TOOL.md` section 4.3, section 6.7
Category:    packaging
Priority:    P1
Effort:      M
Status:      done 2026-09-22

Problem:     One artefact, not two. And an `LD_PRELOAD` path from outside a
             chroot does not resolve inside it, so extracting the object to the
             store is not enough.
Premise:     Measured for the build half:
             `experiments/results/interposer-libc.txt` records that the crate
             builds for both `x86_64-unknown-linux-musl` and
             `x86_64-unknown-linux-gnu` with `-crt-static`, at 14,064 and
             267,680 bytes on this host.
             The placement half is T-0702's premise and rests on the upstream
             maintainer's ruling in
             `references/fritzw__ld-preload-open/api/comments.json`.
Approach:    Build both objects with `scripts/build-interpose.sh`, embed both as
             byte arrays, and on first use write the one T-0706 selected into
             the rootfs at a fixed path under a podbox-owned directory. Set
             `LD_PRELOAD` to that path as the payload will see it.
             ⛔ The interposer must not depend on the rest of the tree. It is
             excluded from the workspace at `Cargo.toml:20-25` for that reason,
             and a `path` dependency back into the workspace is a build failure
             rather than a review comment.
Decision:    Write it into the rootfs rather than keep it in the store and bind
             it in. There is no attach path on this runtime, so a bind is not
             available, and a copy per container is a few hundred kilobytes.
Prove:       `podbox run --rm public.ecr.aws/docker/library/alpine:3.20 sh -c 'test -r /.podbox/interpose.so' && podbox run --rm public.ecr.aws/docker/library/alpine:3.20 sh -c 'true' 2>&1 | grep -q 'is preloaded for this payload'`

**Done 2026-09-22.** The work this entry specifies shipped under
earlier entries and this close verifies it rather than re-implementing
it. Both objects build through `scripts/build-interpose.sh` and embed
as byte arrays (`crates/podbox-cli/src/interpose.rs` `GNU`/`MUSL`);
placement writes the selected one to `/.podbox/interpose.so` through a
temporary file and an atomic rename; `LD_PRELOAD` merges the object
ahead of the payload's own value. Driven green on host podman 6.1.2
with the shipped binary: the file reads inside the payload, and the
banner announces the preload. The interposer stays outside the
workspace (`Cargo.toml`), and no `path` dependency reaches back into
it.

⛔ **The `Prove` above is amended: the committed spelling read
`/proc/self/environ`, which does not exist where `/proc` is unmounted.**
T-0707 measured the payloads unmounted. File presence plus the preload
announcement on stderr proves the same two halves: the object is inside
the rootfs, and the payload runs under it.

---

### T-1003 The launch ladder, and a single file with an embedded rootfs

Source:      captured TOOL.md section 6.7; onelf at 158b4af; current ladder
Category:    packaging
Priority:    P2
Effort:      L
Status:      done

Problem:     The forced launch ladder and embedded-rootfs format were
             recorded as done while part of their implementation is absent.
Premise:     The intended order is memfd, FUSE, tmpfs, run directory, and
             cache. Current code stages four modes. FUSE is selected but
             refused even when its probe succeeds. There is no embedded
             rootfs byte-store format in the released binary.
Approach:    Retain the implemented modes. Add bounded FUSE staging where
             the device and mount operations work. Define and prove the
             embedded rootfs input format. Drive tmpfs on a permitted host.
Decision:    Keep request and active-mode variables separate. Keep refusal
             on a denied prerequisite. A refusal is not an implemented rung.
Prove:       `sh experiments/358-ladder-rungs.sh` exits 0 for existing modes.
             Add tracked forced-FUSE, permitted-tmpfs, and embedded-rootfs
             drives. Each must verify payload bytes, selected mode, child
             status, bounded failure, and owned cleanup.

**Done 2026-09-30.** FUSE is a rung-complete read-only server, not a
refusal: a forked server speaks the kernel ABI read in-lane, serves
the extracted tree without copying it, answers `EROFS` to the
mutating opcodes and `ENOSYS` to unknowns, with every wait bounded
and cleanup on every path; the CLI's final ladder arm enters it the
way it enters tmpfs. The embedded-rootfs format is the `save`
OCI-layout tarball: per-blob hash verification already in `load`
makes it the smallest shape consistent with content addressing, with
no new loader code. `sh experiments/358-ladder-rungs.sh` exits 0
twice ([the drive](../experiments/results/ladder-rungs.txt)): the
forced-FUSE and permitted-tmpfs clauses take their refusal arms
naming node and mount with zero leftovers where this lane denies
them, and their entry arms assert word, bytes, and cleanup
automatically where a host grants them; the pack clause loads, runs,
and refuses the corrupt tarball at 125 naming the digest mismatch.
Limit stated beside the proof: live FUSE and tmpfs entry are
unproved here (`/dev/fuse` absent, `mount(2)` denied even in a
userns, all measured) and the binary-appended footer was future
work; the four completed staging implementations are not repeated.

**Partial 2026-09-30 (history).** The audit reopened the entry from its source.
Memfd, run directory, and cache have recorded live proof. Tmpfs staging
is wired, but its current saved live result proves only the refusal.
FUSE remains a named refusal in the CLI's final ladder arm.
The embedded-rootfs work is absent and remains acceptance work here.

[Earlier evidence](../docs/history/audit-before-2026-09-30/TODO/packaging.txt)
retains the implementation stages and the earlier scoped Done record.
Do not repeat the four completed staging implementations.

**Settled 2026-10-02 under T-1612: the two clauses named as future work are
split by what can clear them, and neither reopens this entry.** The binary-
appended footer is dropped rather than moved, because no document in this tree
defines it. The words appear in three places and nowhere else: this entry's
`Done`, [limits](../docs/limits.md), and T-1612 itself. `TOOL.md` says the
artefact "gets packed into a single file" (sections 3.2 and 3.5) and names
onelf's launch ladder, but no section describes appending a footer to the
binary, and this entry settled the embedded-rootfs format as the `save`
OCI-layout tarball with no new loader code. A clause with no specified shape is
not acceptance that can be met; carrying it forward under any status would
repeat the defect T-1612 found. If a footer is wanted later, it needs its own
entry that defines the bytes, the reader, and the failure it reports.

The live-entry prover is a limit rather than a task. The `358-ladder-rungs.sh`
drive already carries the entry arm and asserts word, bytes, and cleanup
wherever a host grants `/dev/fuse` and the mount; what is missing is a host
that grants them, and this one does not, measured three ways at `/dev/fuse`,
`mount(2)`, and inside a userns. No code change reaches that. [Limits](../docs/limits.md)
records it; this entry's status stays `done` because everything it claimed to
deliver is delivered and its `Prove` exits 0.

----

### T-1004 A reproducible build, and the artefact's own inputs recorded

Source:      `TOOL.md` section 10.9 via `paper_final.md` section 10.9
Category:    packaging
Priority:    P3
Effort:      M
Status:      done 2026-09-22

Problem:     A single-file artefact whose inputs are not recorded cannot be
             traced back to what produced it, and the audience is automated and
             will not remember.
Premise:     Read.
Approach:    Record, in the binary and reported by `podbox version --verbose`:
             the git commit, the rustc version, the target triple, the digest of
             each embedded interposer object, and the achieved mode of the build
             (whether it is static). Verify that two builds of the same commit
             on the same toolchain produce the same bytes, and where they do not,
             name what differed rather than dropping the claim.
             ⛔ No fabricated number: if reproducibility is not achieved, the
             field says so.
Decision:    Report rather than promise. `TOOL.md` says a single-file artefact
             "should record its immutable inputs", which is a reporting
             requirement; bit-for-bit reproducibility is a stronger claim and it
             is measured before it is made.
Prove:       `./experiments/120-reproducible-build.sh`; a runnable verdict
             (0 for a match or 1 for a mismatch) and its committed result
             capture close the entry

**Done 2026-09-22.** The binary records its inputs and reports them,
and two builds of one commit match byte for byte.
`crates/podbox-cli/build.rs` emits the commit (with `-dirty` where the
tree is modified, `unknown` where no commit is readable), the `rustc`
version, the target triple, the hex sha256 of each embedded interposer
object (`absent` where the placeholder went in), and whether
`crt-static` holds; `version --verbose` prints the seven-line document
through a new parity row, and bare `version` is byte-identical to
before. Three unit tests pin the rendering (every input named, unknown
stated never blank, version matching the binary), watched fail before
the implementation and pass after, and the full lane check is green.
Driven by `experiments/120-reproducible-build.sh`, exit 0: two release
builds in one lane container hash to the same sha256 with identical
verbose documents, recorded in
`experiments/results/reproducible-build.txt`.

---

### T-1005 A session reaches the code in one command, and the build runs behind the reading

Source:      Asked for by the operator on 2026-09-09
Category:    packaging
Priority:    P1
Effort:      S
Status:      done 2026-09-09

Problem:     ⛔ **Every session runs in a new machine and pays the same cold cost
             twice over**: the tools the last one installed are gone, and
             `target/` is empty, so the whole dependency graph is compiled again.
             ⚠ A session that reads the orientation documents first and builds
             second pays both serially, and nothing about the reading needs a
             toolchain.
Premise:     ⭐ **Measured on 2026-09-09**, `experiments/results/session-startup.txt`:

             | | |
             | --- | --- |
             | a compile against an empty target directory | **29 s**, 87 crates, 150 MB |
             | the same build warm | 0 s |
             | `bootstrap-env.sh --check` with everything present | 0 s |
             | `TODO/PROGRESS.md` plus `AGENTS.md` | **5,944 words** |
             | dev.sh (now `podbox-dev`) before it returns | **1 s** |

             ⚠ **The install path is not measured and is not estimated.** This
             container already has every component, and measuring what apt and
             the pinned zig download cost a genuinely fresh machine would mean
             removing them. The result file says `not measured here` rather than
             carrying a number nobody took.
Approach:    dev.sh (now `podbox-dev`), and `AGENTS.md`'s opening line is now it.
             1. **`dev.sh`** (now `podbox-dev`) detaches with `setsid`, runs the bootstrap and then
                the build, and returns in about a second **printing what to read
                while it works**. ⛔ Bootstrap first: the build needs `zig` for
                `ring`'s C and would otherwise fail in a way that reads as a code
                error.
             2. **`dev.sh status`** (now `podbox-dev status`) answers `running`, `ready`, `failed` with the
                log's tail, or ⭐ **`stale`**, which is the answer that would
                otherwise mislead: a `ready` predating the last edit is worse
                than no answer.
             3. **`dev.sh build`** (now `podbox-dev build`) for a source change, and it deliberately does
                **not** bootstrap. By the time a session is editing, the
                environment is up, and an apt check per edit is the cost this
                removes.
             4. **`dev.sh check`** (now `podbox-dev check`) before a commit: fmt, clippy, build, tests,
                the gate and the marker check, cheapest first, each read from the
                process that produced it.
Decision:    A shell script over a `Makefile`. ⚠ `make` would have to model the
             dependency graph cargo already models, and the two would drift; the
             one thing `make` buys that cargo does not is running the bootstrap
             and the build behind a session's reading, and that is what this is.
             ⛔ **A second `dev.sh` (now `podbox-dev`) attaches to the first rather than starting a
             competing cargo.** Two builds on one target directory block on the
             same lock, and the second looks like a hang, which is exactly the
             failure `AGENTS.md` says costs a session.
             ⚠ The state lives in `.dev/` under the repository rather than in
             `/tmp`, so a session that loses `/tmp` between turns does not lose
             the log that says why the build failed.
Prove:       `./target/release/podbox-dev` returns in under 5 s; `./target/release/podbox-dev wait` then reports `ready`; changing a source file makes `./target/release/podbox-dev status` report `stale` and exit 1

**Done 2026-09-09.** Every clause of the `Prove` was driven.
experiments/310-session-startup.sh (now `podbox-dev startup`) takes the numbers and clause 5 is dev.sh (now `podbox-dev`)
returning in 1 s.

**Done 2026-10-01 (the dev-driver port).** The driver is the `podbox-dev` binary in
the gate crate (`crates/podbox-gate/src/dev.rs`): `start`, `status`, `wait`,
`build` and `check` mirror `dev.sh`, `session` mirrors `session-start.sh`,
and `startup` mirrors `310-session-startup.sh`. The `check` step that ran
`./scripts/check-todo.py` runs `./target/release/podbox-gate`. Lane proof
with the scripts still beside the tree, then the three scripts deleted in
the same change; the red-run plant is `experiments/results/dev-port-plant.txt`.

⚠ **The staleness check hashes an explicit input set** rather than walking the
tree: `crates/**/*.rs` with their sizes and mtimes, plus `Cargo.toml`,
`Cargo.lock`, `rust-toolchain.toml` and `.cargo/config.toml`. ⛔ Deliberately
not `find .`, because `target/` is 2.3 GB and `references/` is 154 MB and
neither is an input to the build.

---

### T-1314 Nightly releases: one tag builds and tests every supported arch

Source:      the operator, 2026-09-22; `.github/workflows/gate.yml:9-13`; `.cargo/config.toml:40-55`
Category:    packaging
Priority:    P1
Effort:      L
Status:      done 2026-09-23

Problem:     Releases are assembled by hand. `v0.1.0-beta.1` went up through
             hand-run commands, only `x86_64` ships, and no step anywhere
             builds or runs the other six architectures podbox claims.
             `.github/workflows/gate.yml:9-13` triggers on pushes to main and
             pull requests, never on tags, and carries no release job; its
             build job at `.github/workflows/gate.yml:101-109` is `x86_64`
             alone.
Premise:     Read at file and line, not measured. `.cargo/config.toml:40-55`
             points cc-rs at `scripts/zig-cc.sh` for seven musl targets:
             `x86_64`, `aarch64`, `riscv64gc`, `loongarch64`, `armv7`
             (`musleabihf`), `i686` and `powerpc64le`. [T-0911](deps.md) took
             the workspace from one compiling architecture to six, and
             [T-0912](deps.md) cleared the seventh gate, so seven is the
             claimed set. Whether all seven still build, and what the
             embedded interposer objects cost per arch, is what the
             implementing session measures first. Whether
             `scripts/build-interpose.sh` cross-builds the embedded objects
             is unread at the lines that decide it; read it before promising
             per-arch embeds.
             ⚠ The beta verification is the per-arch bar to reuse: the binary
             runs `podbox 0.1.0`, both interposer digests are present, and
             `crt-static` reads yes.
Approach:    One new workflow, `.github/workflows/nightly.yml`, and nothing
             in `gate.yml` moves. On every `v*` tag it builds all seven
             static-PIE binaries, runs each on its own arch through qemu, a
             container, or a native runner (the route is per arch, the
             invariant is not: no binary publishes without executing), and
             publishes a pre-release named nightly with the binaries and
             their hashes beside it. Stable releases stay manual and are
             explicitly not designed here.
             ⛔ A tag that builds red publishes nothing: the per-arch smoke is
             the release gate, not a report beside it.
             Checkpoints: all seven build green before any workflow runs;
             one arch end to end before the matrix widens.
             Pitfalls: qemu timing flakes (every wait is on a condition, per
             [authoring](../docs/methodology/authoring.md) section 6, never
             on a duration); tag pushes racing the gate's cancel-in-progress;
             matrix failures hiding behind a green summary row.
Decision:    Ruled 2026-09-22. The stream is named nightly; every `v*` tag
             triggers it and pushes never do; the matrix covers all seven
             claimed archs; each arch is smoke-tested (version, both
             interposer digests, `crt-static`), and the full acceptance stays
             host-arch; stable is manual and far in the future.
             The rejected alternative is `nightly-*` tags: version tags would
             then do nothing until a manual stable flow exists, which is a
             second naming scheme for one stream.
Prove:       `git push origin v0.1.0-beta.3` publishes a nightly pre-release
             with seven assets, each with a green per-arch smoke row in the
             workflow run. (`v0.1.0-beta.2` proved the matrix and found the
             publish defect below; the tag was kept and the publish moved to
             the tag carrying the fix.)

**Done 2026-09-23.** One `v*` tag builds and smoke-tests all seven claimed
archs through `.github/workflows/nightly.yml`, with the per-arch smoke in
nightly-smoke.sh (now `podbox-smoke`) and the cross link that makes the builds possible
in `.cargo/config.toml`.

The workflow answers version tags alone; nothing in `gate.yml` moves. A
seven-leg matrix (arch, triple, emulator) builds the release binary, runs
the smoke under qemu where the binary cannot run natively, and stages the
binary beside its sha256. The publish job needs every leg, downloads the
seven assets with the run's own token, and creates the pre-release named
nightly (re-uploading where the tag already has one, never deleting and
remaking it). The checkout pin repeats `gate.yml`'s; the one new pin is
`actions/upload-artifact` v5.0.0. The bootstrap line repeats the build
job's, which is what `crates/podbox-gate/src/main.rs` check 19 holds it to.

The smoke asserts four things, each read from the process that produced
it: `version` exits 0 with the version line, `version --verbose` exits 0
and names the triple, both interposer digests are 64 lowercase hex digits
(never `absent`), and `crt-static` reads `yes` beside zero PT_INTERP. It
exits 2 where it could not run. The exit-2 arms (no arguments, no binary)
and the exit-1 arms (a `true` binary printing no version line, a `false`
binary exiting 1) were driven on the host before any lane run.

Checkpoint 1 measured first: plain `cargo build --release --target` linked
2 of 7. The host `cc` refuses foreign objects (`file in wrong format` on
rust's own self-contained crt, and on podbox's objects). The recipe is
`experiments/260-multiarch.sh` clauses 4 and 7 (`rust-lld` over
self-contained objects), moved into `.cargo/config.toml` one target per
section so every build uses it instead of a third copy in the next script.
`x86_64` and `i686` keep the host `cc`. `i686` additionally takes
`+crt-static` alone: without it its verbose document reads `no` while
every other arch reads `yes`, and one static shape is the artefact's rule
(T-1004). `riscv64` failed to compile on `Sysno::renameat`: the variant is
absent from `syscalls` 0.8.1 because the kernel has no such call on
asm-generic, so the const takes the `renameat2` spelling on that arch and
the wrapper already passes zero flags. `syscalls` has no newer release
(max 0.8.1, read 2026-09-23), so there is nothing upstream to wait for.
After the three fixes all seven link with no environment overrides:

```
x86_64      3503032 B  interp 0  crt-static yes
aarch64     2978512 B  interp 0  crt-static yes
riscv64gc   2725832 B  interp 0  crt-static yes
loongarch64 3010432 B  interp 0  crt-static yes
armv7       2763552 B  interp 0  crt-static yes
i686        2992944 B  interp 0  crt-static yes
powerpc64le 3248912 B  interp 0  crt-static yes
```

Checkpoint 2 ran one arch end to end first (aarch64 under
`qemu-aarch64-static`: `podbox 0.1.0`, rc 0, the triple named, both
digests present, `crt-static: yes`), then widened: x86_64 native,
riscv64gc, armv7, i686 and powerpc64le all read `SMOKE-OK` with agreeing
digests. `loongarch64` does not run in this lane: `qemu-loongarch64-static`
7.2 answers SIGILL (rc 132) to a hello-world binary built with the same
toolchain, so the emulator is the gap and not the binary; the leg is
decided by the workflow's own runner. The `s390x` check stays green beside
the change, and `nightly.yml` parses under a real parser (7 legs, trigger
`push tags v*`, publish needing the matrix).

Every per-arch binary carries the x86_64 interposer pair: the crate is
x86_64-only by design (`crates/podbox-interpose/src/lib.rs:50-61`), and
off-arch payloads decline naming both machines
(`crates/podbox-cli/src/interpose.rs:238-246`). Per-arch objects belong to
the T-0704 family, not to this entry.

Prove run 2026-09-23: `git push origin v0.1.0-beta.2` ran the workflow as
run 35768821081. All seven legs green (x86_64, aarch64, riscv64gc,
loongarch64, armv7, i686, powerpc64le), each with its smoke row, and the
loongarch64 leg green under the runner's own qemu, which settles the lane's
open question that way. The publish job died before creating anything:
`gh release create` shells out to git and the job had no checkout (`failed
to run git: not a git repository`). Nothing published, so the tag stands
and the fix (the checkout above) rides the next tag.

Prove run 2026-09-23, continued: `git push origin v0.1.0-beta.3` ran the
workflow as run 35769985618, conclusion success. All seven legs green with
one `SMOKE-OK` row each (x86_64 native at 3503032 B; aarch64, riscv64gc,
loongarch64, armv7, i686 and powerpc64le each under its qemu), and the
publish job created the pre-release named nightly on the tag with fourteen
assets: the seven binaries beside their seven sha256 files, verified back
through the release API. The loongarch64 leg green on the runner's qemu
settles the lane's SIGILL question as emulator age.

Prove run 2026-09-23, continued: `git push origin v0.1.0-beta.4` ran the
workflow as run 35805033988, conclusion success. All seven legs green
with the publish job completing, and the pre-release named nightly on
the tag carries fourteen assets, verified back through the release API.
The tag sits at the T-1207 close-out commit, which moves no build input.

Prove run 2026-09-23, continued: `git push origin v0.1.0-beta.5` ran the
workflow as run 35809485405, conclusion success. All seven legs green
with the publish job completing, and the pre-release named nightly on
the tag carries fourteen assets, verified back through the release API:
the x86_64 binary is 3503032 bytes, the same bytes this session
measured. The tag sits at the T-1212 close-out commit, which moves no
build input.

---

### T-1328 The nightly signs its artefacts, with provenance a downloader can check

Source:      issue 26, client beta testing 2026-09-22 (hash-only
             sidecars from the same release prove truncation, not
             origin); `.github/workflows/nightly.yml`,
             nightly-smoke.sh (now `podbox-smoke`)
Category:    packaging
Priority:    P2
Effort:      M
Status:      done 2026-09-25

Problem:     Each `.sha256` sidecar is served from the same release as
             its binary, so it protects against a truncated download,
             not a compromised release: no signature, no attestation, no
             SBOM (`grep -ri 'cosign|sigstore|sbom|attest' TODO/` returns
             nothing). For a tool installed as `docker`/`podman` on PATH,
             a verifiable signature is the one supply-chain control that
             matters.
Premise:     Read from the release (14 assets, hashes only) and the
             tree (no signing surface anywhere). The gap is absence, and
             absence is established by the empty grep.
Approach:    Sign the artefacts at publish (Sigstore/cosign keyless is
             the shape to evaluate: no long-lived key to guard, OIDC
             from the publish job), publish the bundle beside the
             binaries, and document one verification command a
             downloader runs. Out of scope: SBOM generation (named as
             future work if dropped), changing what is built.
Decision:    Keyless signing at publish (Sigstore, OIDC from the
             publish job): no long-lived key to guard and rotate, and
             verification names the workflow identity. Ruled 2026-09-23
             by the operator; the entry is workable as written.
Prove:       `sh scripts/verify-release.sh <tag> <arch>` (the documented
             command, wrapping `cosign verify-blob`) run by a fresh
             downloader against one artefact and its published bundle
             names the workflow identity; a tampered byte fails the
             check. Close issue
             26 (signing third) with a comment showing the verification
             and the publish-time signature as the guard that stops
             recurrence.

**Done 2026-09-25.** Proved on `v0.1.0-beta.6` (nightly run
36110971684, conclusion success): seven `.sigstore` bundles beside
the seven binaries (21 assets), `sh scripts/verify-release.sh
v0.1.0-beta.6 x86_64` exits 0 with `Verified OK` naming the
workflow identity, and the same check against a binary with one
flipped byte exits 1 on the digest mismatch. The review before the
tag corrected the expected identity from `@refs/heads/main` to
`@refs/tags/<tag>`: the workflow answers version tags alone, so a
tag-built bundle carries the tag's ref and the earlier identity
would have failed closed on honest artefacts. The guard is the
sign step plus the verifier script. Issue 26 is closed, as checked through
the repository API on 2026-09-30. No close-out action remains.

---

### T-1329 The per-arch smoke pulls and extracts, not just versions

Source:      issue 26, client beta testing 2026-09-22 (a broken
             non-x86_64 binary publishes green);
             nightly-smoke.sh (now `podbox-smoke`)
Category:    packaging
Priority:    P2
Effort:      M
Status:      done 2026-09-23

Problem:     The per-arch smoke asserts version, target, both interposer
             digests, `crt-static` and no `PT_INTERP`, but never pulls,
             extracts or runs a payload: a non-x86_64 binary broken for
             its actual job still publishes green. The release note says
             so parenthetically ("the full acceptance stays host-arch"),
             so it is honest, but honesty about a gap is not the coverage.
Premise:     Read from the smoke script: no pull, no extract, no run on
             any arch leg.
Approach:    `pull` plus `extract` per arch against a loopback fixture
             (no registry access needed), asserting the payload bytes;
             keep the run itself host-arch (qemu-user execution stays out
             until T-1327's pairs exist to make it meaningful). Out of
             scope: full per-arch acceptance (the note keeps saying
             host-arch until that entry exists), registry-dependent
             fixtures.
Decision:    Loopback pull+extract per arch. The fixture shape already
             exists in the registry-fixture work (T-0206 family); reuse
             it rather than inventing a second fixture.
Prove:       `./target/release/podbox-smoke` (or its per-arch leg) fails on a
             fixture whose layer bytes are flipped and passes on the
             honest one, on the native leg; groups 1-4 run on all seven
             legs. Close issue 26 (smoke third) with a comment showing
             the failing-then-passing legs and the pull+extract
             assertions as the guard that stops recurrence.

**Done 2026-09-23.** Group 5 in nightly-smoke.sh (now `podbox-smoke`): a
synthetic one-file image travels by save/load through the binary
under test (import, save, load, extract, payload bytes read back),
then the tarball's layer blob is flipped and the same load must
refuse with a digest mismatch. Save/load stands in for the decided
loopback pull: both funnel through the loader's per-entry hash
check, and neither needs a registry, a quota, or the network; the
store is scratch. Group 5 runs on native legs only (the matrix's
x86_64 leg); groups 1-4 run on all seven. `run` stays out per the
Approach: qemu-user execution waits on T-1327's pairs.

**Done 2026-10-01 (the smoke port).** The smoke is the `podbox-smoke`
binary in the gate crate (`crates/podbox-gate/src/smoke.rs`): the six
assertion groups run against the binary it is given, and `--diagnostics`
drives the gate-diagnostics fixture. Lane proof runs the binary green
against the retired script on the same binary, then deletes
scripts/nightly-smoke.sh in the same change; the red-run plant is
`experiments/results/smoke-port-plant.txt`.

Lane-proved on the host arch with a lane-built binary (`.tmp`
job, since removed): honest leg passes reading `smoke-payload`,
flipped leg refuses, `SMOKE-OK x86_64-unknown-linux-musl`. The
corruption targets the layer blob inside the OCI layout, not the
outer tar framing: `load` hashes each entry's bytes against the
descriptor naming it, so a flipped outer byte only corrupts GNU
tar headers the loader never hashes (measured: first shape passed
the flipped leg). The guard is the group-5 leg itself: a binary
broken for pull or extract fails its own publish.

---

### T-1334 The release carries its build commit's gate state and its reproducibility boundary

Source:      issue 13, client beta testing 2026-09-22 (beta.1 build
             commit failed the repo gate; cross-host bytes differ);
             `TODO/packaging.md` (T-1004)
Category:    packaging
Priority:    P1
Effort:      S
Status:      done 2026-09-25

Problem:     Two release-integrity gaps. The beta.1 build commit shipped
             with the repo's own consistency gate red (the fix landed
             minutes later), and nothing in the release says the gate
             state of the commit it names. And the notes read as
             reproducible while cross-host builds differ in the embedded
             glibc interposer object (host glibc version symbols via
             `interpose.map`), so a downloader has no path ending in the
             released bytes; T-1004's wording ("same toolchain, same
             bytes") is met only intra-host.
Premise:     Measured by the reporter: same rustc and zig, pristine
             clone, different bytes (`interpose-gnu` digest differs,
             `interpose-musl` matches). The gate failure (run 35755156697,
             T-1004-shaped `check-todo` refusal) is record.
Approach:    State both in the release path: the notes (or `version
             --verbose`) carry the build commit's gate state, and one
             line states the boundary ("byte-identical within one
             host/toolchain; cross-host builds differ in the embedded
             glibc interposer object, digest in `version --verbose`").
             T-1004 keeps its intra-host claim with the boundary named.
             Out of scope: pinned-glibc-header interposer builds (a real
             fix for cross-host bytes, filed as future work, not done
             here), signatures (T-1330).
Decision:    Boundary statement, not a rebuild pipeline. The bytes are
             honest once conditioned; the missing piece is the condition.
Prove:       `grep -c` over the next beta's notes finds the build
             commit's gate run and conclusion plus the boundary line;
             `version --verbose` (or the notes) states it.
             Close issue 13 with a comment showing the notes and the
             boundary sentence as the guard that stops recurrence.

**Done 2026-09-25.** Proved on `v0.1.0-beta.6`'s notes, read back
through the release API: build commit `6d86129`, gate `success` with
its run URL, and the reproducibility boundary line, all present.
(The full forty-hex commit is in the notes themselves.)
The current notes step refuses publication without a successful main gate
on the exact build commit. Issue 13 is closed, as checked through the
repository API on 2026-09-30. No close-out action remains.

----

### T-1423 The release tag names the workspace version

Source:      beta.11 publication; `.github/workflows/nightly.yml`;
             `Cargo.toml:23`
Category:    packaging
Priority:    P1
Effort:      S
Status:      done

Problem:     Tag `v0.1.0-beta.11` built with the manifest still reading
             `0.1.0-beta.10`, so the published binary reports the wrong
             release. The nightly smoke only asserts the `podbox *`
             shape, never tag equality.
Premise:     One workspace version feeds every crate, and the nightly
             matrix builds from the tag. Nothing between the tag push
             and the publish refuses a mismatch.
Approach:    Bump-then-tag in that order, and hold the order with a
             guard: the nightly build's first step refuses where the
             tag and the manifest disagree.
Decision:    A mismatched tag publishes nothing. The guard names both
             values.
Prove:       The nightly workflow is green on tag `v0.1.0-beta.12`; the
             fresh binary reports `0.1.0-beta.12`; the bundles verify;
             the lane exercise runs version, pull, run, and the closure
             acceptances on it.

**Done 2026-09-30.** The manifest reads `0.1.0-beta.12` with the lock,
the nightly build refuses a mismatched tag before one arch builds
(proven both ways: beta.12 matches, beta.11 refuses), and the source
snapshot is regenerated. The nightly workflow is green on the tag
([the matrix proof](../experiments/results/publication-beta12.txt)):
7 architectures, 42 assets at build commit `067c0ce`. Both bundles
verify ([binary](../experiments/results/binary-release-beta12.txt),
[SSH](../experiments/results/ssh-release-beta12.txt)). The fresh
binary reports `0.1.0-beta.12`, and the lane exercise runs version,
pull, run, 127/125 refusals, the exact exe answer, inspect, verify,
strict-safety refusal, the pack round-trip, the detached cycle, and
the four helpers green
([the drive](../experiments/results/exercise-beta12.txt)). Beta.11
stays published with its defect recorded; no history was rewritten.

### T-1559 Port `scripts/build-state.py` to `podbox-buildstate`

Source:      `refactor/06-entries/T-R005.md`; `TODO/INDEX.md:53`;
             `scripts/build-state.py`; `experiments/393-build-freshness.py`
Category:    packaging
Priority:    P2
Effort:      M
Status:      done

Problem:     Build freshness is decided by a Python script. The shell
             retirement moves durable workflows into native Rust, and this
             script is `packaging` category work with no reason to stay
             Python. Two consumers pin its interface: `podbox-dev` calls it
             at four sites in `crates/podbox-gate/src/dev.rs`, and
             `experiments/393-build-freshness.py` imports it as a module.
Premise:     The record format is the contract: schema `podbox-build/1`,
             sorted keys, CPython separators, one trailing newline, atomic
             replace through a `.new` sibling. The four CLI verbs keep
             their names, flags, messages, and exit codes.
Approach:    Port the logic to `crates/podbox-buildstate`, binary
             `podbox-buildstate`, with no dependencies. SHA-256 and the
             JSON writer are hand-rolled to match CPython byte for byte.
             Keep `scripts/build-state.py` as a compat shim that execs the
             binary, so the `dev.rs` call sites and the `py_compile` gate
             step move nothing. Re-drive `393` through the shipped CLI with
             the same fixture and the same verdict lines, so the run proves
             the binary.
Decision:    The binary owns the logic; the script is a shim. The port
             must not reintroduce the unread `.dev/last-build-inputs`
             stamp: only `.dev/build-state.json` is read and written.
Prove:       `cargo test -p podbox-buildstate` green in the lane; the
             retired script and the binary agree on `inputs` and `commit`
             over the same tree; `python3
             experiments/393-build-freshness.py` exits 0 against the
             binary; the plant transcript shows each stale state going red;
             `./target/release/podbox-gate` exits 0.

**Done 2026-10-01.** The `podbox-buildstate` crate carries
`podbox-buildstate` and `podbox-release-licenses` with no
dependencies, and both scripts stay as exec shims. Lane proof
([buildstate-crate-proof](../experiments/results/buildstate-crate-proof.txt)):
unit tests 7 and 6 passed; retired script and binary agree on the
inputs digest and the commit over the same tree; each writer's record
reads `ready` under the other; both shims reach their binaries; the
licence inventories are identical; `393` exits 0 with all fourteen
fixture rows green; clippy and fmt clean. Plant
([transcript](../experiments/results/buildstate-crate-plant.txt)):
every stale shape goes red and every refusal exits 2. The record gate
exits 0 on the landed tree.

### T-1560 Port `scripts/release-licenses.py` to `podbox-release-licenses`

Source:      `refactor/06-entries/T-R005.md`; `TODO/INDEX.md:53`;
             `scripts/release-licenses.py`; `scripts/package-ssh.sh:43`
Category:    packaging
Priority:    P2
Effort:      S
Status:      done

Problem:     The licence inventory for the locked package set is produced
             by a Python script. It is `packaging` category work and moves
             into the `podbox-buildstate` crate beside `podbox-buildstate`.
             Its one live caller is `scripts/package-ssh.sh`, which runs it
             with `--output` under whichever Python it finds.
Premise:     `inventory.json` is the contract: `json.dumps` with `indent=2`
             and insertion-ordered keys. The package set, the licence stem
             match, the traversal guard, and the exit codes stay as written.
Approach:    Port the logic to binary `podbox-release-licenses` with no
             dependencies, including a minimal JSON reader for the `cargo
             metadata` output. Keep `scripts/release-licenses.py` as a
             compat shim that execs the binary, so `package-ssh.sh` and the
             `py_compile` gate step move nothing.
Decision:    The script keeps its path and its `--output` flag; the binary
             adds an optional `--root` that defaults to the checkout found
             by walking up from the working directory.
Prove:       `cargo test -p podbox-buildstate` green in the lane; the
             retired script and the binary produce identical inventories
             over the same lock; `./target/release/podbox-gate` exits 0.

**Done 2026-10-01.** `scripts/release-licenses.py` keeps its path and
its `--output` flag and now execs `podbox-release-licenses` from the
`podbox-buildstate` crate, which pulls in no dependencies. Lane proof
([buildstate-crate-proof](../experiments/results/buildstate-crate-proof.txt),
section 5): retired script and binary produce identical inventories
over the same lock, and the shim reaches its binary. Plant: a
missing `--output` and an unwritable output each exit 2. The record
gate exits 0 on the landed tree.

### T-1561 Port `scripts/verify-release.sh` to `podbox-verify`

Source:      `refactor/06-entries/T-R005.md`; `TODO/INDEX.md:53`;
             `scripts/verify-release.sh`; `TODO/packaging.md` T-1328
Category:    packaging
Priority:    P2
Effort:      M
Status:      done

Problem:     The downloader's half of the signed nightly is a shell
             script. It is `packaging` category work and moves into the
             `podbox-release` crate. Its callers name the path, the two
             asset kinds, and the three exit states.
Premise:     The workflow identity is the contract: the nightly workflow
             at the tag being verified, keyless through OIDC. `gh` and
             `cosign` are driven directly with the same 120-second bound
             the script's `timeout` gave them. A wait past the bound is
             "could not run", never a verification failure.
Approach:    Port the logic to binary `podbox-verify` with no
             dependencies. Keep `scripts/verify-release.sh` as a compat
             shim that execs the binary, so the prose callers and the
             publication proof keep working.
Decision:    The script keeps its path and its `TAG ARCH [binary|ssh]`
             shape; the binary owns the fetch, the identity, and the
             scratch cleanup on every exit path.
Prove:       `cargo test -p podbox-release` green in the lane; a missing
             tool and a bad kind each exit 2 through the shim;
             `./target/release/podbox-gate` exits 0.

**Done 2026-10-01.** The downloader half of the signed nightly runs as
binary `podbox-verify`, built from the `podbox-release` crate with no
dependencies, and `scripts/verify-release.sh` remains a shim that execs
it. Lane proof
([release-crate-proof](../experiments/results/release-crate-proof.txt),
section 4): no arguments exits 2 and a bad arch exits 2, both through
the shim, and the shim reaches its binary. Plant: a bad kind exits 2
with the refusal naming the missing tool. The record gate exits 0 on
the landed tree.

### T-1563 Port `experiments/120-reproducible-build.sh` and `157` to `podbox-prove-t0211`

Source:      `refactor/06-entries/T-R005.md`; `TODO/INDEX.md:53`;
             `experiments/120-reproducible-build.sh`;
             `experiments/157-lock-inheritance-prove.sh`; `TODO/image.md` T-0211
Category:    packaging
Priority:    P1
Effort:      M
Status:      done

Problem:     The reproducible-build measurement and the lock-inheritance
             proof are shell. Both are mutation-planted store-lock proofs,
             which is why they share one binary rather than one name. The
             `157` anchors are the current source text: the guard in
             `Lock::try_acquire` and the one flag in `Lock::open`. A stale
             anchor passes vacuously.
Premise:     The lane job and the mutation table are the contract: two
             release builds of one tree and one mutation at a time, each
             asserted to land before it is read. The dirty-subject refusal
             and the restore-from-copy rule move with the logic.
Approach:    Port both proofs to binary `podbox-prove-t0211` with
             `reproducible` and `lock-inheritance` subcommands and no
             dependencies. Keep both experiment paths as compat shims that
             exec the binary with the matching subcommand, so T-1004 and
             T-0211 keep their paths.
Decision:    One binary because both are store-lock proofs. The scripts
             keep their paths, their flags, and their exit codes; the
             binary owns the job text, the anchors, and the reports.
Prove:       `cargo test -p podbox-release` green in the lane; the
             lock-inheritance proof runs its two mutations against the
             current tree; `./target/release/podbox-gate` exits 0.

**Done 2026-10-01.** One dependency-free binary, `podbox-prove-t0211`,
now lives in the `podbox-release` crate and answers both store-lock
subcommands, so the two retired experiments reach it through shims that
exec it. Lane proof
([release-crate-proof](../experiments/results/release-crate-proof.txt),
sections 6 and 6b): the lock-inheritance proof runs both mutations
against the current tree with two attempts per test; retired and
binary both exit 1 with agreeing verdicts; clause 1 passes 2 of 2.
Qualification: neither mutation reddens its own test (both exit 0
where a nonzero is wanted on one arm), which is the tree's behaviour,
not the port's: retired and binary agree exactly, so the port is
faithful and T-0211 stays partial on the live lock. Plant: an unknown
subcommand exits 2, and a run without the lane refuses honestly. The
record gate exits 0 on the landed tree.

### T-1565 Port `scripts/release-notes.sh` to `release-notes`

Source:      `refactor/06-entries/T-R005.md`; `TODO/INDEX.md:53`;
             `scripts/release-notes.sh`; `.github/workflows/nightly.yml`;
             `TODO/packaging.md` T-1334
Category:    packaging
Priority:    P2
Effort:      S
Status:      done

Problem:     The nightly release notes are written by a shell script. The
             notes carry the gate state of the build commit and the
             reproducibility boundary, both computed, never copied. The
             publish job has no Rust toolchain, so the binary reaches it as
             a build-leg artefact, not a fresh build.
Premise:     The refusal contract is the point: no successful main gate
             for the exact build commit means no notes, exit 2. The SSH
             paragraph stays conditional on the tagged tree carrying the
             packaging script.
Approach:    Port the logic to binary `release-notes` with no
             dependencies. Keep `scripts/release-notes.sh` as a compat
             shim. Ship the musl-static binary from the x86_64 build leg
             as its own artefact and run it from the publish job, so the
             notes step needs no toolchain where it runs.
Decision:    The notes step runs the binary, not the shim. The shim stays
             for operators. The nightly diff is one new artefact and one
             repointed step, recorded here.
Prove:       `cargo test -p podbox-release` green in the lane; a tag with
             no gate refuses through the shim; `./target/release/podbox-gate`
             exits 0.

**Done 2026-10-01.** The nightly notes come from the dependency-free
binary `release-notes` in the `podbox-release` crate;
`scripts/release-notes.sh` stays as an exec shim for operators. Lane
proof
([release-crate-proof](../experiments/results/release-crate-proof.txt),
section 4): a tag with no gate refuses through the shim, and the
shim reaches its binary. Plant: notes for a tag that names no commit
exits 2 naming the missing tool. The nightly diff lands here too, as
the Decision requires: the x86_64 leg couriers the musl-static binary
through its own artefact under a name outside the `dist/podbox-*`
release glob, and the publish job runs it instead of the shim. The
workflow parses under the gate's own reader and the gate stays green;
a live tag run owns the end-to-end proof. The record
gate exits 0 on the landed tree.

### T-1566 Port `experiments/395-reconcile-repository.py` to `podbox-reconcile`

Source:      `refactor/06-entries/T-R005.md`; `TODO/INDEX.md:53`;
             `experiments/395-reconcile-repository.py`;
             `TODO/packaging.md` T-1314
Category:    packaging
Priority:    P2
Effort:      M
Status:      done

Problem:     The publish-branch reconciliation is a Python script. It is
             git history and patch comparison, and it moves into the
             `podbox-release` crate. Its verdicts distinguish a failed
             expectation from a machine that could not answer.
Premise:     The pins are the contract: the retained branch name and the
             two fixed revision ranges. The `--expect-deleted` arm keeps
             the script's test: absent is the expected state, and anything
             else fails.
Approach:    Port the logic to binary `podbox-reconcile` with no
             dependencies, adding an optional `--root` that defaults to the
             checkout found by walking up. Keep the experiment path as a
             compat shim that execs the binary.
Decision:    The script keeps its path and its flag; the binary owns the
             git calls, the pins, and the verdict lines.
Prove:       `cargo test -p podbox-release` green in the lane; the binary
             reads the current branch state; `./target/release/podbox-gate`
             exits 0.

**Done 2026-10-01.** `podbox-release` grew binary `podbox-reconcile`,
which carries no dependencies and reads the branch state itself; the
retired experiment path execs it as a shim. Lane proof
([release-crate-proof](../experiments/results/release-crate-proof.txt),
section 5): the binary reads the current branch state (rc 2 where the
retained publish ref is absent, 0 where present) and the shim reaches
its binary. Plant: the `--expect-deleted` arm runs over the live
refs and prints its report. The record gate exits 0 on the landed tree.

### T-1567 Port `experiments/399-publication.py` to `podbox-publish`

Source:      `refactor/06-entries/T-R005.md`; `TODO/INDEX.md:53`;
             `experiments/399-publication.py`; `TODO/packaging.md` T-1314
Category:    packaging
Priority:    P2
Effort:      M
Status:      done

Problem:     The publication acceptance is a Python script. It reads exact
             commit CI and the optional release and branch acceptance, and
             it moves into the `podbox-release` crate. Signature content
             verification stays with `podbox-verify`; this binary never
             writes to a remote.
Premise:     The error taxonomy is the contract: a failed expectation
             prints `FAIL:` and exits 1, and a missing tool, ref, or answer
             prints `cannot run:` and exits 2. The release tag shape and
             the unique architecture matrix keep the script's tests.
Approach:    Port the logic to binary `podbox-publish` with no
             dependencies, including a minimal JSON reader for the `gh`
             answers. Keep the experiment path as a compat shim that execs
             the binary.
Decision:    The script keeps its path and its flags; the binary owns the
             workflow reads, the matrix parse, the asset-set comparison,
             and the verdict line.
Prove:       `cargo test -p podbox-release` green in the lane; a bad tag
             shape fails and a missing origin refuses through the shim;
             `./target/release/podbox-gate` exits 0.

**Done 2026-10-01.** The publication acceptance is now binary
`podbox-publish`, a dependency-free member of the `podbox-release`
crate; the experiment path stays as an exec shim that reaches it. Lane
proof
([release-crate-proof](../experiments/results/release-crate-proof.txt),
sections 4 and 4b): the shim reaches its binary, and a malformed tag
through the shim exits 2 with `cannot run: required command failed:
gh` (the remote read precedes the tag-shape check where `gh` is
absent; the exit-1 shape on a connected host is unit-proven).
`podbox-reconcile` beside it reads the same live branch state. The
record gate exits 0 on the landed tree.

### T-1605 Push a version tag and prove the nightly release-notes wiring live

Source:      T-1565; `.github/workflows/nightly.yml`;
             `TODO/RULES.md` section 2
Category:    packaging
Priority:    P1
Effort:      S
Status:      partial

Problem:     T-1565 ported `scripts/release-notes.sh` to the
             `release-notes` binary and the nightly job runs the
             couriered musl-static copy. That wiring is proven on the
             lane and by the plant transcript, but never end to end:
             no version tag has been pushed since the port, so the
             publish path has not executed once against the binary.
Premise:     The lane proof drives the binary the same way the nightly
             job does, so the untested part is the trigger and the
             courier, not the release-notes logic. A tag push is the only
             thing that starts it.
Approach:     Push a version tag and read the resulting run. This no
             longer needs operator authorisation: `TODO/RULES.md`
             section 2 records the standing grant of read and write on
             this repository, including tags and the releases they
             publish. Confirm the workspace version and the existing tag
             list before choosing the number, so the tag does not
             duplicate one. Read the nightly run's exit code and the
             release it produced.
Decision:    Take the tag in this task. Do not stop and ask: the operator
             settled on 2026-10-02 that agents are free to do read and
             write operations on this repository and only this
             repository, and named tags and releases explicitly.
Prove:       `git tag --list` shows the new tag on the landed commit; the
             nightly run it triggers exits 0; the published release
             carries release notes generated by the `release-notes`
             binary rather than by the retired shell; the run log names
             the binary path it executed;
             `./target/release/podbox-gate` exits 0.

**Partial 2026-10-02. The tag is pushed and the nightly ran; it is red,
and the cause is a shim, not the wiring.** `git tag --list` shows
`v0.1.0-beta.13` on commit `1a87b10`, which is the landed commit and the
version the workspace manifest names; the tag push triggered two nightly
runs, 36974650357 and 36974650815. `TODO/RULES.md` section 2's grant
covers this, so no authorisation was waited on.

The wiring under test is not reached. All six per-arch legs fail at
`smoke and package the SSH helpers` with exit 2 and this line:

```
release-licenses: podbox-release-licenses is not built;
cargo build -p podbox-buildstate
```

That is `scripts/release-licenses.py`, the exec shim T-1560 left behind
so `scripts/package-ssh.sh` and the `py_compile` gate step keep working.
Its candidate list searched two fixed layouts, `target/release/` and
`target/x86_64-unknown-linux-musl/release/`, plus two Windows ones. The
nightly builds each leg for its own triple into `target/<triple>/release/`,
so on i686, aarch64, riscv64gc, loongarch64, armv7 and powerpc64le the
binary was built and the shim did not look where it was. The shim now
searches all seven triples from the nightly's own matrix, and the list is
checked against that matrix rather than kept in step by hand.

So two clauses are unmet and neither is claimed. The nightly run does not
exit 0, so the publish path has still never executed end to end. And
because every leg stops before `publish`, the release notes were never
generated on this tag and there is no release to read them from. What is
established is the trigger and the courier path: the tag fires the
nightly, and the failure is one line of shell between them.

The two fixes this reading needs are in this change: the shim's search
list, and the regenerated `docs/runtime-state.md`, which the `check-todo`
job reported as stale because the version bump moved the declared
version.

### T-1619 The `10`/`20`/`130` three-way decision belongs to the wave that deletes `10`

Source:      `refactor/recon-c.md:137`, the plan's row, read off disk
             2026-10-02; `refactor/06-entries/PLAN.md:174-177`;
             `refactor/DEFERRALS.md:51-53`
Category:    packaging
Priority:    P0
Effort:      M
Status:      done

Problem:     One script's deletion is conditional on two others, and the
             three-way dependency is called irreducible. The decision
             has to be recorded before the deletion, and nothing in the
             tracked record says who owns it.
Premise:     The plan's correction is conditional: the delete stands only
             if the two dependants are dealt with first, and both name
             the script in live paths. The plan also rules that a
             reduction of the dependency was considered and does not
             work.
Approach:     This entry records the ownership and the clearing condition
             only. The decision itself is taken by the wave that
             deletes `experiments/10-build-target-image.sh`, because that
             wave is the one whose deletion the decision unblocks. Write
             the decision into that wave's entry, here, or in both with
             one pointing at the other.
Decision:     Named here, owned there. A second decision record would be
             two places to keep in step, and this entry's value is that
             the question has a name and an owner rather than a
             duplicate of the answer.
Prove:       `grep -n "10-build-target-image" TODO/packaging.md` names
             this entry as the owner of the decision and the wave entry
             that takes it.

**Done 2026-10-02.** The entry names the owner and the clearing
condition, and it does not take the decision: the wave that deletes
`experiments/10-build-target-image.sh` takes it. The body records the
two live pointers at `experiments/20-enter-target.sh:70` and
`experiments/300-run.sh:308-311`, the one-unit conversion rule at
`TODO/gate.md:1139-1140`, and `refactor/DEFERRALS.md:49-53` as the
assignment. It corrects the `Premise` on two points: three citers read
`TODO/gate.md:1125` as the `Decision:` when it is a `Problem:`
continuation line, and no document records a reduced dependency being
tried and rejected. `grep -c "10-build-target-image"
TODO/packaging.md` returns 8.

**The owner, named, and the wave that takes the decision.** The three
scripts are `experiments/10-build-target-image.sh`, which builds the image,
`experiments/20-enter-target.sh`, which enters it, and
`experiments/130-probe-parity.sh`, whose clauses 1 and 3 drive `20`
(`experiments/130-probe-parity.sh:131`, `:149`).

⛔ **The deletion is conditional on the other two, not free.** Two live
paths name `10` by name. `experiments/20-enter-target.sh:70` prints
`SKIP: $IMAGE not built. Run ./experiments/10-build-target-image.sh` and
exits 2 inside the check that guards its own run, and
`experiments/300-run.sh:308-311` prints the same path after
`docker image inspect container-research/target:1` fails. A deletion that
leaves either pointer naming a file that is gone is not a deletion.
`refactor/03-peer-review-2/round-2.md:129-137` records this as Correction
7, and `refactor/02-peer-review-1/round-1.md:720-724` as its item 7.

**The three convert as one unit, and the word is `irreducible`.**
`TODO/gate.md:1139-1140` reads "The three convert as one unit. Converting
`130` without `20` tests nothing, and converting `20` without `10` builds
nothing." That sentence is the ruling the plan carries, so the conversion
order is already settled. `refactor/06-entries/PLAN.md:174-177` (VC-4)
and `refactor/06-entries/T-R006.md:67` both cite it.

⚠ **Three places cite that ruling at `TODO/gate.md:1125`, and on this
tree `:1125` is a `Problem:` continuation line, not the `Decision:`.**
The `Decision:` moved to `:1139` at commit `996cb98` (2026-10-01), which
is the shell-retirement port. `refactor/07-verify/verify-waves.md:144`
marks the claim **TRUE** while reading `:1125` as the `Decision:`, so its
check passed on a line number that had already moved. The three citers
are `refactor/06-entries/PLAN.md:177`, `refactor/06-entries/T-R006.md:67`
and `refactor/03-peer-review-2/round-2.md:407`, and `refactor/` is
untracked by `.gitignore:152`, so nothing repairs them.

⚠ **The Premise's second claim is not supported by the sources it names.**
Reading `refactor/06-entries/PLAN.md:174-177`, `T-R006.md:67`, both peer
reviews, and the two group reports, no document records a reduced
dependency being tried and rejected. The plan asserts
`irreducible` and cites the `Decision:` above, and that is all: a
statement about conversion order, not a record of a rejected reduction.
Searched for `reduc`, `two-way`, and each way of naming the SKIP line
being dropped or inlined; every hit is the word `irreducible` itself.
`refactor/01-audit/group-7.md:145-153` names the decision the owner has to
take, and it offers two shapes, keep the reconstruction or drop it, rather
than ruling one out.

**The owner is the wave that deletes `10`, and no tracked entry is that
wave yet.** The script exists in `refactor/06-entries/T-R006.md`, whose
row `:67` reads "this one waits for a separate decision about the
three-way dependency. It is listed so it is not forgotten, not so it can
be deleted alongside the other eleven." The other ten are already gone,
deleted by commit `996cb98`; `10` is the one that stayed, which is
consistent with the row's own condition.
`refactor/DEFERRALS.md:49-53` assigns the row to the wave that deletes
`10`, and `refactor/recon-c.md:34-39` records that `T-R006.md` is a
seventh wave the plan's section 8 omits, so a reader following that
section drops it. `refactor/recon-c.md:804-810` still carries the open
question of whether that seventh wave is authorised work at all.

⛔ **So the owner is named here and the clearing condition is unmet until
a wave entry exists.** `refactor/` is untracked, so its ids cannot be
written into a `TODO/` body: the gate's cross-reference check at
`crates/podbox-gate/src/main.rs:4007` reads every `T-` followed by four
digits on a body line and raises "names T-NNNN, which is not an entry"
for each, and it has no exemption for a plan id. The plan's own row
number is therefore not printed here. `refactor/06-entries/T-R006.md` is
the wave document, and `refactor/recon-c.md:137` is its task row. When a
tracked wave entry lands, this entry takes the decision and that entry
points here, per the `Approach`.

⚠ **The ledger is PRE-VC and this row is one of the three the plan does
not list as changed.** `refactor/06-entries/verdict-ledger.tsv:2` records
`experiments/10-build-target-image.sh` at DELETE, group report
`group-7.md:120`. `refactor/06-entries/PLAN.md:22-27` says the ledger is
PRE-VC and names the rows that change: `95` (VC-3), `151` (VC-7) and
`162` (VC-2). `10` is not among them, so a reader who trusts the ledger
over the prose deletes `10` and keeps the other two. The same ledger
carries `20` as KEEP-SHELL (`:33`) and `130` as RUST-TEST (`:9`), and
`refactor/06-entries/T-R006.md:13-14` sources its own table from the
ledger, so the error reaches the wave document.

**What clears it.** The three-way decision is recorded in a tracked wave
entry, that entry's `Decision` names what happens to
`experiments/10-build-target-image.sh`, and the pointers at
`experiments/20-enter-target.sh:70` and `experiments/300-run.sh:308-311`
are repaired in the same change as the deletion, per
`refactor/06-entries/T-R006.md:86-95`, which warns that a bare
`experiments/` citation left behind turns the gate red. [T-1631](milestones.md)
carries the dependency itself beside this entry's question, so the two
are found together and neither closes alone.

