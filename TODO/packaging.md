# packaging

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

Source:      `TOOL.md` section 5 M7, section 6.7; `references/qaidvoid__onelf`
Category:    packaging
Priority:    P2
Effort:      L
Status:      done 2026-09-22

Problem:     A runtime that can only start from a filesystem cannot start on a
             machine whose writable paths are full, and the machines this is for
             have a 64 MiB `/tmp`.
Premise:     ⭐ Read at file and line, and the shape is worth copying whole.
             `references/qaidvoid__onelf/tree/crates/onelf-rt/src/main.rs:128`
             names the order: memfd, then FUSE, then an ephemeral tmpfs, then a
             private run directory, then a persistent cache. Each **forced** mode
             refuses with a named reason rather than falling through:
             `references/qaidvoid__onelf/tree/crates/onelf-rt/src/main.rs:216-218`,
             `:235-237` and `:255-257`. The on-disk mode runs only when asked
             for, and otherwise the tool refuses:
             `references/qaidvoid__onelf/tree/crates/onelf-rt/src/main.rs:261-273`.
             ⭐ **And one mechanism that is easy to miss and costs a session to
             rediscover.**
             `references/qaidvoid__onelf/tree/crates/onelf-rt/src/main.rs:125-131`:
             `ONELF_MODE` **requests** a mode and the mode actually chosen is
             reported under a **different** name, `ONELF_ACTIVE_MODE`, "so a
             packed app that launches another one does not hand it a directive".
             podbox has exactly that hazard: `podbox run` inside `podbox run`.
             The request variable and the result variable must not share a name.
             ⚠ On this runtime the ladder is shorter than onelf's: FUSE needs
             `/dev/fuse`, which `mknod` cannot create, and an ephemeral tmpfs
             needs a mount. Both rungs are refused here by probe, not omitted.
Approach:    Implement memfd, then the private run directory, then the
             persistent cache, each with a named refusal. Probe for the two
             rungs this runtime lacks rather than assuming their absence, so the
             same binary uses them where they exist.
             ⚠ The userland-exec rung arrives vendored with `goblin` and `nix`
             pins ([T-0909](deps.md)); patch both out before wiring it, with
             the [T-0908](deps.md) program-header walk and the `sys` module in
             their place.
             Where the payload's own relocation is the problem rather than the
             filesystem, `references/VHSgunzo__sharun` is the lineage that solves
             it and `references/VHSgunzo__ulexec` drives both rungs from one
             tool. Neither substitutes for section 6.7: they solve getting a binary to
             run from nowhere, not filesystem virtualization.
Decision:    Two environment variables, request and result, from the start. The
             single-variable form works until podbox runs inside itself, and
             then it is a bug that reproduces only under nesting.
Prove:       `podbox run --rm public.ecr.aws/debian/debian:bookworm-slim sh -c 'apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq busybox-static' && PODBOX_MODE=memfd podbox run --rm public.ecr.aws/debian/debian:bookworm-slim /bin/busybox sh -c 'echo $PODBOX_ACTIVE_MODE' | grep -qx memfd && podbox run --rm public.ecr.aws/docker/library/alpine:3.20 sh -c 'test -z "$PODBOX_MODE" && test -n "$PODBOX_ACTIVE_MODE"'`

**Done 2026-09-22.** The CLI wiring in
`crates/podbox-cli/src/ladder.rs`, driven by foreground `run` and
refused everywhere else. `prepare` reads `PODBOX_MODE` (an unknown word
refuses with the rung list), refuses the force on `create`, `run -d`
and the machine tier before anything is fetched, and admits the rung
into `Prepared` beside the probe rows; the foreground path drives it
through `enter_forced` with a banner line naming the entered rung, and
`exec` refuses the force at entry. `Availability` feeds from the probe
rows: FUSE from the open row, tmpfs from the attach verdict, rundir and
cache down until wired, `PODBOX_CACHE` of `1` or `true` opting the
cache rung in. The memfd leg stages bytes through `memfd::stage`
(create, write, seal where accepted) and execs the fd through
`run_ladder`, which closes the parent's copy past the fork on every
path. Nine unit tests pin the request parse, the scope refusals, the
three feeds and the three pre-entry refusals (sketch, script, missing),
all fork-free; `staging_hands_back_a_live_cloexec_descriptor` pins the
stage path with close-on-exec set. Driven by
`experiments/163-ladder-drive.sh`, exit 0 twice on host podman 6.1.2
with a lane-built musl binary: the forced memfd over a static payload
enters on the rung (`ACTIVE_MODE=memfd`, rc 0), the default and
scrubbed entries report chroot, and the fuse, unknown-word,
exec-scope and dynamic-payload refusals each name their reason at 125,
with raw transcripts in `experiments/results/sweep163/`. Two findings
from the drive: alpine's busybox is dynamically linked (`PT_INTERP
/lib/ld-musl-x86_64.so.1`, measured from the pinned image), so the
rung correctly refuses it and the static payload is debian's
busybox-static at `/bin/busybox` (the package ships that name;
verified static with no `PT_INTERP` against the `.deb` ground truth,
which also confirms apt under podbox installs byte-correct); and the
setup run carries no `--rm`, which would delete the rootfs holding its
install. Full `dev.sh check` green in the lane. The embedded-rootfs
byte store stays the entry's named follow-up; rundir and cache stay
sketched.

**Build notes, 2026-09-22: the skeleton change.** What this change holds, and what it does
not:

| rung | state | where |
| --- | --- | --- |
| env-var pair | rung-complete, unit-tested | `crates/podbox-enter/src/plan.rs`, `lib.rs` |
| memfd driver | rung-complete, unit-tested | `crates/podbox-enter/src/memfd.rs`, `abi.rs` |
| fd-exec number | rung-complete, unrun | `crates/podbox-probe/src/sys.rs` |
| payload resolve | rung-complete, unit-tested | `crates/podbox-enter/src/ladder.rs` |
| ladder entry | rung-complete, unrun | `crates/podbox-enter/src/lib.rs` |
| FUSE probe | rung-complete, unit-tested | `crates/podbox-probe/src/probes.rs` |
| FUSE, tmpfs | sketched: ordered, probe-fed, named refusals | `crates/podbox-enter/src/ladder.rs` |
| run-dir, cache | sketched: refused as not implemented | `crates/podbox-enter/src/ladder.rs` |
| vendor patch | `goblin` and `nix` out, one at a time | `vendor/userland-execve/Cargo.toml` |
| CLI wiring | rung-complete, unit-tested, driven (163) | `crates/podbox-cli/src/ladder.rs`, `run.rs`, `exec.rs`, `lifecycle.rs` |

Decision: two environment variables, request and result, from the start. The
single-variable form works until podbox runs inside itself, and then it is a
bug that reproduces only under nesting. The scrub sits at both choke points:
`Plan::env_for` strips `PODBOX_MODE` and any stale `PODBOX_ACTIVE_MODE` from
what the image or the caller declared, and `spawn` filters them again on the
way to `execve` while pushing `PODBOX_ACTIVE_MODE=<entered rung>`, so a
hand-built plan cannot leak one either. Cost if wrong: a nested `podbox run`
inherits a directive it was never given, and the outer run's forced mode
decides the inner run's rung silently.

⚠ Two substitutions the tree forced, recorded so the lane does not re-derive
them. Neither number crate names `fexecve` (`syscalls` 0.8.1 and
`linux-raw-sys` 0.12.1 were read on 2026-09-22 and neither spells it), so the
fd-exec goes through `execveat` with the empty path and `AT_EMPTY_PATH`,
which is the same kernel operation and the one libc implements `fexecve`
with. The 10 MB synthetic-stack figure rides in the vendored file unchanged;
the ladder review that decides whether it stays is this entry's follow-up,
not this change.

⚠ The "single file with embedded rootfs" byte store is OUT of scope: the
Approach specifies only launch order. It is a follow-up line here, not a rung
built beside it.

Lane run 2026-09-22 over this change plus T-0207's: full `dev.sh
check` green (fmt, workspace clippy with `-D warnings`, musl release
build, workspace tests 469 passed 0 failed in 14 suites including the
new scrub, eligibility, write/seal and ladder tests, interpose tests,
`check-gate.sh --fast` 9 passed with the 2 familiar skips). The run
found two defects, both fixed here: an unused test-only import in
`ladder.rs` and an `Errno` without `Display` in the sealing test.
`check-todo.py`'s only complaint is this file citing the then-untracked
`ladder.rs`, which the commit clears. The Prove drive and the CLI
wiring (read `PODBOX_MODE`, feed `Availability`) landed in the close
above.

**Build notes, 2026-09-22: the memfd leg.** The memfd leg's three missing pieces:

- the payload, read from outside the rootfs: `ladder::resolve_payload`
  searches `path_dirs` in order for a bare name and resolves a `/`-carrying
  argument under the root through `abi::resolve_in`, so a `..` that escapes
  is refused rather than followed; `ladder::payload_bytes` bounds the read
  at 128 MiB, which is `abi::Elf::read`'s own ceiling. Four unit tests pin
  the order, the missing name, the escape and the bytes. The errors ride
  the crate's one-parameter `Result` as `Error::Runtime`, which is the
  file's own shape (`Mode::parse`); the first cut wrote
  `Result<_, String>` and the lane's clippy refused it in eight places.
- the entry the rung drives through: `spawn` is now `spawn_with` with the
  entered rung word and no fd, and `spawn_ladder`/`run_ladder` drive beside
  it with the ladder rung word and the written memfd. The child execs the
  fd through `memfd::exec_fd` without resolving a path where one was handed
  in, else falls to the path candidates (a `#!` script routes past
  fd-exec). A non-memfd mode through this entry refuses: the other rungs
  are ordered, not rung-complete. The parent names the new failure row
  (`execveat of the memfd`) and keeps the 126/127 split for it. One entry
  sequence, so the fork, the chroot order and the readiness pipe cannot
  drift between the two (`docs/conventions/code.md`).
- the FUSE rung's probe input: `open(/dev/fuse, O_RDWR)` as a Census leg in
  the outer environment beside the ptmx pair it copies the rule from, and
  `fuse_usable` beside `ptmx_usable`, true only on an `Ok` open. Two unit
  tests pin the rule and the row's place.

Lane run 2026-09-22 over this change: full `dev.sh check` green (fmt,
workspace clippy with `-D warnings`, musl release build, workspace tests
with the 4 ladder and 2 probe tests new, gate 9 passed with the 2 familiar
skips). The run found the `Result` arity above and three fmt spots, all
fixed here. The CLI wiring (read `PODBOX_MODE`, feed `Availability`,
drive the Prove) landed in the close above.

---

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
             | `scripts/dev.sh` before it returns | **1 s** |

             ⚠ **The install path is not measured and is not estimated.** This
             container already has every component, and measuring what apt and
             the pinned zig download cost a genuinely fresh machine would mean
             removing them. The result file says `not measured here` rather than
             carrying a number nobody took.
Approach:    `scripts/dev.sh`, and `AGENTS.md`'s opening line is now it.
             1. **`dev.sh`** detaches with `setsid`, runs the bootstrap and then
                the build, and returns in about a second **printing what to read
                while it works**. ⛔ Bootstrap first: the build needs `zig` for
                `ring`'s C and would otherwise fail in a way that reads as a code
                error.
             2. **`dev.sh status`** answers `running`, `ready`, `failed` with the
                log's tail, or ⭐ **`stale`**, which is the answer that would
                otherwise mislead: a `ready` predating the last edit is worse
                than no answer.
             3. **`dev.sh build`** for a source change, and it deliberately does
                **not** bootstrap. By the time a session is editing, the
                environment is up, and an apt check per edit is the cost this
                removes.
             4. **`dev.sh check`** before a commit: fmt, clippy, build, tests,
                the gate and the marker check, cheapest first, each read from the
                process that produced it.
Decision:    A shell script over a `Makefile`. ⚠ `make` would have to model the
             dependency graph cargo already models, and the two would drift; the
             one thing `make` buys that cargo does not is running the bootstrap
             and the build behind a session's reading, and that is what this is.
             ⛔ **A second `dev.sh` attaches to the first rather than starting a
             competing cargo.** Two builds on one target directory block on the
             same lock, and the second looks like a hang, which is exactly the
             failure `AGENTS.md` says costs a session.
             ⚠ The state lives in `.dev/` under the repository rather than in
             `/tmp`, so a session that loses `/tmp` between turns does not lose
             the log that says why the build failed.
Prove:       `./scripts/dev.sh` returns in under 5 s; `./scripts/dev.sh wait` then reports `ready`; touching a source file makes `./scripts/dev.sh status` report `stale` and exit 1

**Done 2026-09-09.** Every clause of the `Prove` was driven.
`experiments/310-session-startup.sh` takes the numbers and clause 5 is `dev.sh`
returning in 1 s.

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
`scripts/nightly-smoke.sh` and the cross link that makes the builds possible
in `.cargo/config.toml`.

The workflow answers version tags alone; nothing in `gate.yml` moves. A
seven-leg matrix (arch, triple, emulator) builds the release binary, runs
the smoke under qemu where the binary cannot run natively, and stages the
binary beside its sha256. The publish job needs every leg, downloads the
seven assets with the run's own token, and creates the pre-release named
nightly (re-uploading where the tag already has one, never deleting and
remaking it). The checkout pin repeats `gate.yml`'s; the one new pin is
`actions/upload-artifact` v5.0.0. The bootstrap line repeats the build
job's, which is what `scripts/check-todo.py` check 19 holds it to.

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
             `scripts/nightly-smoke.sh`
Category:    packaging
Priority:    P2
Effort:      M
Status:      open

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
Status note: **open, and it proves on the next tag, not on this
             tree.** The workflow change (install, sign, upload) and
             the verifier script are committed here, but no `.sigstore`
             bundle exists until the nightly publish job runs one: the
             Prove command names `<tag>`, and only a tag push runs it.
             T-1334's notes half is in the same position. Both close on
             `v0.1.0-beta.6` (RESUME's tag-gated instruction), each with
             its own artefact read back.

Prove:       `sh scripts/verify-release.sh <tag> <arch>` (the documented
             command, wrapping `cosign verify-blob`) run by a fresh
             downloader against one artefact and its published bundle
             names the workflow identity; a tampered byte fails the
             check. Close issue
             26 (signing third) with a comment showing the verification
             and the publish-time signature as the guard that stops
             recurrence.

---

### T-1329 The per-arch smoke pulls and extracts, not just versions

Source:      issue 26, client beta testing 2026-09-22 (a broken
             non-x86_64 binary publishes green);
             `scripts/nightly-smoke.sh`
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
Prove:       `scripts/nightly-smoke.sh` (or its per-arch leg) fails on a
             fixture whose layer bytes are flipped and passes on the
             honest one, on the native leg; groups 1-4 run on all seven
             legs. Close issue 26 (smoke third) with a comment showing
             the failing-then-passing legs and the pull+extract
             assertions as the guard that stops recurrence.

**Done 2026-09-23.** Group 5 in `scripts/nightly-smoke.sh`: a
synthetic one-file image travels by save/load through the binary
under test (import, save, load, extract, payload bytes read back),
then the tarball's layer blob is flipped and the same load must
refuse with a digest mismatch. Save/load stands in for the decided
loopback pull: both funnel through the loader's per-entry hash
check, and neither needs a registry, a quota, or the network; the
store is scratch. Group 5 runs on native legs only (the matrix's
x86_64 leg); groups 1-4 run on all seven. `run` stays out per the
Approach: qemu-user execution waits on T-1327's pairs.

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
             `TODO/packaging.md:282` (T-1004)
Category:    packaging
Priority:    P1
Effort:      S
Status:      open

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
Status note: **open, and it proves on the next tag, not on this
             tree.** `scripts/release-notes.sh` is committed and was
             driven locally against `v0.1.0-beta.5` (build commit plus
             gate run and conclusion plus boundary line, all present),
             but the notes it generates only publish when the nightly
             publish job runs one: the Prove reads the next beta's
             notes, and only a tag push writes them. Closes on
             `v0.1.0-beta.6` beside T-1328, each with its own artefact
             read back.
Prove:       `grep -c` over the next beta's notes finds the build
             commit's gate run and conclusion plus the boundary line;
             `version --verbose` (or the notes) states it.
             Close issue 13 with a comment showing the notes and the
             boundary sentence as the guard that stops recurrence.
