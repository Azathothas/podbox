Area: deps.

Record semantics: [task rules](RULES.md#5-entry-closure).


`TOOL.md` section 3.5. The dependency policy, applied.

[INDEX.md](INDEX.md) is the list and the counts. [PROGRESS.md](PROGRESS.md) is the work order.
[RULES.md](RULES.md) is how an entry closes. [reference-map.md](reference-map.md) is the corpus.

⛔ **Every dependency is a decision that has to be argued, and the default
answer is no.** The artefact is packed into a single file, is a candidate for
the memfd launch rung, and ships to machines with a 64 MiB `/tmp`. Binary size
is a functional requirement here.

⛔ **`Cargo.toml`'s `[workspace.dependencies]` is empty and that is deliberate.**
`TOOL.md` section 3.5 printed a table of fifteen crates in an earlier revision, and it
read as a shopping list. What follows is the sweep, one entry per area, and each
closes with **five answers and a measured `cargo bloat` delta**, not a
`cargo add`.

## The five questions every entry below answers

| | |
| --- | --- |
| 1 | What does it do that we would otherwise write, and roughly how much code is that? |
| 2 | What does it cost, as measured `cargo bloat` and `cargo tree` figures against the same build without it? |
| 3 | Does it pull C? A C dependency breaks `crt-static` cleanliness or drags a toolchain. Prefer the pure-Rust path and **measure** the performance claim before choosing the C one. |
| 4 | Is it maintained, and is its licence compatible with redistribution under 0BSD? Determined per `docs/methodology/references.md` step 3, **tracker read**, and recorded in [reference-map.md](reference-map.md). |
| 5 | Vendor or registry? |

## The vendor and registry split

It is not about size. It is about **whether we will end up patching it**.

| | what | why |
| --- | --- | --- |
| **Vendor and patch, in-tree** | anything niche, small, unmaintained, or sitting on a seam we will need to move: the interposer's building blocks, anything ELF- or syscall-shaped, anything forked | We will need behaviour its published interface does not expose, and `docs/methodology/vendoring.md` says fix it here, so it has to live here |
| **Registry dependency, pinned** | large, generic, well-maintained infrastructure with no seam we need to move: TLS, hashing, JSON, argument parsing, compression | Vendoring buys nothing and costs a cold compile plus every one of their lints becoming ours |
| **Write it ourselves** | anything where the dependency is bigger than the code it replaces | The default when 1 and 2 do not clearly favour the crate |

# T-0901: Sweep: syscalls

**Source:** `TOOL.md` section 3.5
**Category:** deps
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

Roughly every syscall podvm makes is a raw one, and `unsafe` is
unavoidable. The question is whether a crate is saving enough of it
to pay for itself.

## Premise

Read. `rustix` with its `linux_raw` backend is the starting point.
⚠ Going in: the new mount API, the seccomp ioctls and Landlock are
likely hand-declared either way, so the crate covers the ordinary
half and not the interesting half.

## Approach

Declare the section 6.1 probe set by hand first, measure the binary, then
add the crate and measure again. `libc` alone may be enough, and it
is already in the dependency tree of anything else considered here.

## Decision

Recommendation, to be confirmed by the measurement: hand-declare,
with `libc` for the type and constant definitions. `TOOL.md` section 3.3
requires `unsafe` be confined to one module per subsystem with the
safe wrapper adjacent, and that shape is the same work whether a
crate is under it or not.

## Prove

`./experiments/110-bloat-delta.sh syscalls` exits 0 and writes bloat-syscalls.txt in `experiments/results/`

## Correction

**Ruling: hand-declare, and neither crate lands.** `pod_hostprobe::sys`
is 330 lines including its documentation and its tests, and it already covers
the half the entry expected to stay hand-written anyway: the new mount API,
`seccomp`, `landlock_create_ruleset`, `kcmp` and `pidfd_getfd`. The 46 numbers
and 19 constants in it were checked against the kernel's own headers, which is
the correctness a crate would have been bought for.

⚠ `libc` is priced at zero and may be taken later for a struct definition
without re-arguing this entry. What it may not be taken for is the errno: a
libc wrapper answers with the library's `setuid` semantics rather than the
kernel's, which is [T-0101](probe.md)'s whole point.

## Done

**Done 2026-09-08.** Measured as two areas rather than one, because the entry
names two candidates: `experiments/results/bloat-syscalls-libc.txt` and
`experiments/results/bloat-syscalls-rustix.txt`.

| candidate | crates | delta | what it covers |
| --- | --- | --- | --- |
| `libc` 0.2 | 1 | **0 bytes**, below the instrument's resolution | types and constants |
| `rustix` 0.38 | 3 | **0 bytes**, below the instrument's resolution | the ordinary half |
| hand-declared | 0 | the baseline | all of it, in 330 lines |

⭐ **The size argument is void in both directions, so the decision is the one
the entry predicted for the other reason.** Neither crate moves a static musl
binary measurably: `libc` on this target is mostly declarations against a libc
already linked, and `rustix`'s `linux_raw` backend emits the same instruction
`pod_hostprobe::sys` emits. ⚠ Both readings passed the instrument's
scaffold control, so a zero here is a reading and not an unmeasured run.

# T-0902: Sweep: seccomp BPF

**Source:** `TOOL.md` section 3.5
**Category:** deps
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

podvm installs short filters. A crate that generalizes filter
construction may be larger than the filters.

## Premise

Read, with a size reference in the corpus:
`references/Azathothas__container-research/tree/verification/confine/main.go`
emits a filter by hand in about sixty lines, and that is the size
of the thing a crate would replace.

## Approach

Emit the BPF by hand, measure, then compare against `seccompiler`.
⚠ The notification listener is a separate question from filter
construction and is T-0606; a crate that helps with one may not
help with the other.

## Decision

Recommendation: hand-emit. Revisit if the notification tier ever
becomes reachable and needs a larger filter surface.

## Prove

`./experiments/110-bloat-delta.sh seccomp` exits 0 and writes `experiments/results/bloat-seccomp.txt`

## Correction

**Ruling: hand-emit, and it does not land.** The filter podvm installs is one
BPF instruction, already emitted in `pod_hostprobe::probes` as four
fields of a `#[repr(C)]` struct, and the reference emits a real one in about
sixty lines. 8 KB for sixty lines fails the third row of this file's own
vendor-and-registry table: the dependency is bigger than the code it replaces.

⚠ The entry's "revisit if the notification tier becomes reachable" is now
answered rather than pending: [T-0606](supervise.md) establishes the tier has
neither a race-safe channel nor a way to acquire one on this runtime, and
[T-0107](probe.md)'s rung selection refuses it for that reason. There is no
larger filter surface coming.

## Done

**Done 2026-09-08.** `experiments/results/bloat-seccomp.txt`: `seccompiler` 0.5
costs **+8,192 bytes** and 2 crates.

# T-0903: Sweep: Landlock

**Source:** `TOOL.md` section 3.5
**Category:** deps
**Milestone:** none
**Priority:** P3
**Effort:** S
**Status:** done

## Problem

Three syscalls. A crate for three syscalls needs an argument.
⚠ podvm does not currently **install** a Landlock ruleset; it
probes for one that is already there. This entry is the sweep for
the day it does.

## Premise

Read, with a size reference:
`references/Azathothas__container-research/tree/verification/confine/landlock.go`
is the whole mechanism in one file.
⭐ The corpus also carries a design ruling that bears on it:
`references/multikernel__sandlock` PR #29 dropped path strings from
its notification policy surface on the reasoning that "path-based
access control belongs in static Landlock rules". Where podvm ever
needs path policy, Landlock is where it goes, not the notifier.

## Approach

Hand-declare, measure, compare against the `landlock` crate.

## Decision

Recommendation: hand-declare. Three syscalls, and podvm needs the
ABI-version probe more than the ruleset builder.

## Prove

`./experiments/110-bloat-delta.sh landlock` exits 0 and writes `experiments/results/bloat-landlock.txt`

## Correction

**Ruling: hand-declare, and it does not land.** Zero bytes is not the whole
price: thirteen crates is thirteen more things to audit, pin and rebuild for a
mechanism podvm uses one syscall of. What podvm needs today is the ABI probe,
and that is `landlock_create_ruleset(NULL, 0, 1)`, already one row of
`pod_hostprobe::probes`. Revisit if podvm ever installs a ruleset,
which needs a kernel that has Landlock at all.

## Done

**Done 2026-09-08.** `experiments/results/bloat-landlock.txt`: the `landlock`
crate 0.4 costs **0 bytes** (below the instrument's resolution, scaffold
control answered) and **13 crates**.

# T-0904: Sweep: OCI registry client and types

**Source:** `TOOL.md` section 3.5
**Category:** deps
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

⚠ The heaviest candidate in the list by far. `oci-client` and
`oci-spec` cover a much larger surface than the endpoints podvm
uses.

## Premise

Read. podvm needs token auth, manifest by tag and digest, manifest
lists for one platform, and blob fetch: T-0201's list.

## Approach

Write the minimal client against that list, measure it, then
measure the same build with `oci-client` and `oci-spec` and compare.
Count the types actually deserialized: a manifest, a manifest list,
an image config and a descriptor.

## Decision

Recommendation, to be confirmed: write it. The endpoint set is
small and fixed, and `TOOL.md` section 5 M1 says the registry plane is the
least interesting part of the problem and must not be gold-plated.

## Prove

`./experiments/110-bloat-delta.sh oci` exits 0 and writes `experiments/results/bloat-oci.txt`

## Correction

**Ruling: write it, confirming the entry's recommendation.** Twice the bytes and
three times the crates, for four types. ⭐ The scaffold is itself part of the
finding: `oci-spec`'s `ImageManifest` and `Descriptor` refused literals that a
hand-written `#[derive(Deserialize)]` accepts, because it validates a larger
surface than the endpoints of [T-0201](image.md) need. That validation is not
free and it is not asked for.

⚠ `oci-client` was NOT measured here and is not refused here: it drags an HTTP
stack, which [T-0906](#t-0906-sweep-http) prices on its own, and measuring the
two together would have credited one with the other's cost.

## Done

**Done 2026-09-08.** `experiments/results/bloat-oci.txt`: `oci-spec` 0.7 with
only its `image` feature costs **+69,664 bytes** and **40 crates**, against
**+32,768 bytes and 13 crates** for `serde_json` plus structs
(`experiments/results/bloat-json.txt`).

# T-0905: Sweep: TLS

**Source:** `TOOL.md` section 3.5
**Category:** deps
**Milestone:** none
**Priority:** P0
**Effort:** S
**Status:** done

## Problem

HTTPS is the only working transport (T-0201), so TLS is not
optional, and the wrong choice breaks the static story.

## Premise

Read. ⛔ Non-negotiable that it is pure Rust: the alternative pulls
OpenSSL, which is a C dependency and breaks `crt-static`.
⚠ One tree in the corpus shows the trap:
`references/VHSgunzo__ulexec/tree/Cargo.toml:31` takes `reqwest`
with `native-tls-vendored` on Linux while its Windows arm at
`references/VHSgunzo__ulexec/tree/Cargo.toml:24` takes `rustls-tls`. The vendored-native arm is the one that
drags a C toolchain.

## Approach

`rustls` plus `webpki-roots`, measured. Also measure the root store
question separately: bundling roots costs binary size, and reading
the host bundle costs a file that may not exist. podvm needs the
host bundle anyway for T-0407, so measure "host bundle with
`webpki-roots` as the fallback" against "roots bundled".

## Decision

Recommendation: `rustls`. There is no pure-Rust alternative with a
comparable maintenance state, and the C path is ruled out by the
static requirement rather than by preference.
⛔ Never disable certificate verification to work around the
TLS-interception on a build host. That is a red line in
`TOOL.md` section 11.1 and it is the one that turns a build problem into a
shipped vulnerability.

## Prove

`./experiments/110-bloat-delta.sh tls` exits 0 and `readelf -d target/x86_64-unknown-linux-musl/release/podvm | grep -c NEEDED | grep -qx 0`

## Correction

**Ruling: `rustls` lands, with the host bundle first and `webpki-roots` as the
fallback.** There is no pure-Rust-all-the-way-down alternative in a comparable
maintenance state, and HTTPS is the only transport that works
([T-0201](image.md)), so the alternative to this cost is no image acquisition
at all. The host bundle is read anyway for [T-0407](complete.md), and it found
more roots here than the bundled set (152 against 121); the bundled set is what
makes a machine with no bundle still work.

## Done

**Done 2026-09-08, and it lands.** Three arms, because the entry asks for the
root-store question to be measured separately:

| arm | crates | delta | roots seen |
| --- | --- | --- | --- |
| host bundle only, `rustls-pemfile` | 14 | +831,536 | 152 |
| ⭐ host bundle, `webpki-roots` as the fallback | 15 | **+901,168** | 152 |
| bundled roots only, `webpki-roots` | 15 | +880,688 | 121 |

`experiments/results/bloat-tls.txt` and `bloat-tls-hostroots.txt`. The static
half of the `Prove` holds: `readelf -d` reports **0** `NEEDED` entries, so the
artefact is still one static-pie file.

⚠ **TWO RUNS OF THE SAME ARM CAN DIFFER, AND THESE DID.** The first readings of
these arms were +819,248 and +897,072, taken before
`experiments/110-bloat-delta.sh` recorded the dependency declaration and the
scaffold beside the number. Re-taken so they carry both, they read +831,536 and
+901,168: 1.5% and 0.5%. The pins are version RANGES, so cargo resolves what is
newest at the moment of the run, and the bundled-roots row above is the one
reading not re-taken and is therefore quoted as the older one. ⭐ Nothing in the
ruling turns on 1.5%, and the reason the difference is visible at all is that a
result file now carries what produced it. A sweep that needs two runs to be
comparable to the byte needs a committed lock, which no entry asks for.

⛔ **THE PREMISE IS CORRECTED, AND THE CORRECTION IS THE FINDING.** It reads
that pure Rust is non-negotiable because the alternative pulls OpenSSL and
breaks `crt-static`. `rustls` **is** pure Rust and its crypto provider is not:
`ring` compiles C and assembly, and the first build for the musl target failed
with

```
error occurred in cc-rs: failed to find tool "x86_64-linux-musl-gcc"
```

So the C question is not answered by choosing `rustls` over OpenSSL. What
changes is which C, how much, and whether it is reproducible. ⭐ It does not
break `crt-static`: the artefact still has no `PT_INTERP` and no `NEEDED`. It
costs a C cross-compiler, and `scripts/zig-cc.sh` with `.cargo/config.toml` is
the answer, pinned and checksummed by
`scripts/common/bootstrap-env.sh`.

# T-0906: Sweep: HTTP

**Source:** `TOOL.md` section 3.5
**Category:** deps
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

An async runtime is a large dependency with nothing to do here.

## Premise

Read. podvm has no reason to be async: it fetches blobs
sequentially and its concurrency is one child per container, held
by a pidfd.

## Approach

Measure `ureq` against a hand-rolled client over `rustls`. Count
what podvm actually needs: `GET` and `HEAD`, redirects, chunked
bodies, a `Range` header for resumption, and bearer tokens.

## Decision

Recommendation: `ureq`, blocking. The hand-rolled path has to
re-implement chunked transfer encoding and redirects, which is more
code than the crate and is the class of code that is wrong in ways
a test suite finds slowly.

## Prove

`./experiments/110-bloat-delta.sh http` exits 0 and writes `experiments/results/bloat-http.txt`

## Correction

**Ruling: `ureq`, blocking, confirming the entry's recommendation.** 127 KB buys
chunked transfer encoding, redirects and the `Range` header, which the entry
argues correctly is more code than the crate and is the class that is wrong in
ways a suite finds slowly. The scaffold exercised exactly the surface the entry
lists: `GET`, `HEAD`, a redirect budget, a `Range` header and a bearer token.

⚠ **69 crates is the real price and it is not the bytes.** It is 69 things to
pin, audit and rebuild. The entry's premise that podvm has no reason to be
async holds and is what keeps that number from being far larger.

⭐ **One duplicate found while landing it, and the pin records it.** `ureq 2.12`
depends on `webpki-roots 0.26.11`, which is a compatibility shim over
`webpki-roots 1.0.9`. Naming `0.26` in `Cargo.toml` would have put podvm behind
the shim too; naming `1` shares the crate that actually holds the roots. The
resolve graph is quoted at the pin.

## Done

**Done 2026-09-08, and it lands.** `experiments/results/bloat-http.txt`: `ureq`
2 with its own TLS stack costs **+1,024,152 bytes and 69 crates** against the
empty baseline, which is **about +123,000 bytes on top of the TLS arm** that has
to be paid either way. ⚠ That second figure is a subtraction between two runs
rather than a measurement, and [T-0905](#t-0905-sweep-tls) records why two runs
of one arm can differ by a percent. The order of magnitude is what the decision
rests on.

# T-0907: Sweep: tar, gzip, zstd

**Source:** `TOOL.md` section 3.5, section 6.3
**Category:** deps
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

Extraction is driven at **entry level, never `unpack()`** (T-0301),
so the crate is being used for a fraction of its surface and the
fraction matters more than the total.

## Premise

Read. `tar`, `flate2` with `rust_backend`, and `ruzstd` are the
starting points. ⛔ `flate2`'s default backend is C
(`miniz_oxide` is the Rust one and is the feature to select), and
the C backend is the trap question 3 exists for.

## Approach

Confirm the entry-level API gives header, reader and no implicit
filesystem action. Measure with `rust_backend` selected. Measure
`ruzstd` against skipping zstd entirely: zstd layers exist and are
not universal, and a named refusal for a zstd layer is a legitimate
v1 answer if the size delta is large.

## Decision

Recommendation: `tar` plus `flate2` with `rust_backend`, and a
measurement before zstd. `TOOL.md` section 6.3 rules out shelling out and
a hand-written tar reader is the one piece of this list that is
genuinely small, so the measurement may still say write it.

## Prove

`./experiments/110-bloat-delta.sh archive` exits 0 and `cargo tree -e normal -p podvm-extract | grep -c ' cc ' | grep -qx 0`

## Done

**Done 2026-09-08, and all three land.**

| arm | crates | delta |
| --- | --- | --- |
| `tar` + `flate2` with `rust_backend` | 9 | +65,536 |
| the same plus `ruzstd` | 12 | **+69,632** |

`experiments/results/bloat-archive.txt` and `bloat-archive-zstd.txt`. No `cc` in
the graph with `rust_backend` selected, which is the trap question 3 exists for
and the reason that feature is named rather than defaulted. ⭐ Read out of
`cargo metadata`'s resolve graph rather than assumed: the whole set is
`adler2, cfg-if, crc32fast, filetime, flate2, libc, miniz_oxide, ruzstd,
simd-adler32, static_assertions, tar, twox-hash`, with neither `cc` nor
`libz-sys` in it.

⭐ **The zstd question the entry left open is settled by 4,096 bytes.** Adding
`ruzstd` costs one page over the gzip pair. A named refusal for a zstd layer
would have been a legitimate v1 answer against a large delta; against one page
it is a feature removed to save nothing.

⚠ The scaffold used the entry-level API only, which is what [T-0301](extract.md)
requires: a header, a path and a reader, with no implicit filesystem action.
`unpack()` was never called and never will be, because path safety, whiteouts
and the ownership sidecar are podvm's decisions.

⚠ The `Prove`'s second half names `podvm-extract`, and the measurement was
taken with the scaffold in `podvm-image`, which is where every arm of this
sweep put it so the arms are comparable. The `cc` check was read from the
workspace graph instead and is the same answer: no `cc` anywhere.

# T-0908: Sweep: digests, JSON, argument parsing, ELF

**Source:** `TOOL.md` section 3.5
**Category:** deps
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

Four small questions with one large answer among them. ⚠ `clap` is
large for what podvm needs from it.

## Premise

Read. `sha2`, `serde_json`, `clap` and `goblin` are the starting
points. podvm's parser has a table (T-0801) rather than a grammar,
and its ELF needs are `PT_INTERP` presence plus a Go build-marker
check (T-0706), which is a header walk.

## Approach

Measure each separately, because they are independent decisions
that a single "add the obvious four" would bundle.
For `clap`, measure it against a hand-rolled parser driven by the
section 6.8 table. For `goblin`, measure it against a program-header walk.

## Decision

Recommendation: `sha2` and `serde_json` yes, `clap` and `goblin`
measured first and probably no. `clap`'s help generation is the
part podvm would use least, because docker's help text is the
text to match rather than generate.

## Prove

`./experiments/110-bloat-delta.sh cli-json-elf` exits 0 and writes bloat-cli-json-elf.txt in `experiments/results/`

## Done

**Done 2026-09-08.** Measured as four areas rather than one, because the entry
says they are independent decisions that a single "add the obvious four" would
bundle. `experiments/results/bloat-digest.txt`, `bloat-json.txt`,
`bloat-argparse.txt` and `bloat-elf.txt`.

| candidate | crates | delta | ruling |
| --- | --- | --- | --- |
| `sha2` | 9 | +8,192 | ⭐ **lands** |
| `serde_json` + `serde` | 13 | +32,768 | ⭐ **lands** |
| `goblin` | 11 | +32,768 | ⛔ no |
| `clap` | 4 | **+159,744** | ⛔ no |

⭐ **`clap` is the finding the entry predicted and the number is worse than
expected.** 159,744 bytes is nineteen times `sha2` and five times `serde_json`,
for a parser podvm drives from a **table** ([T-0801](cli.md)) rather than a
grammar. The measurement was taken with `default-features = false` and only
`std`, `help`, `usage` and `error-context` enabled, so it is the small clap and
not the large one. ⚠ Its help generation is the part podvm would use least:
docker's help text is text to **match**, not to generate.

**`goblin` costs the same as `serde_json` and replaces far less.** podvm's ELF
needs are `PT_INTERP` presence and a Go build-marker check
([T-0706](interpose.md)), which are program-header walks of a few dozen lines
each. The scaffold confirmed it: `interp=None sections=25` on podvm's own
static-pie binary is one field and one count, from a crate that parses the whole
format.

# T-0909: Vendor the memfd and userland-exec rungs, and fix the fork's regression here

**Source:** `TOOL.md` section 3.5; [reference-map.md](reference-map.md)
**Category:** deps
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

These sit on a seam podvm will move, and one of them is a fork
maintained because the original is not. A registry dependency on an
unmaintained crate is a blocker waiting to happen, and "blocked on
upstream" is not an outcome this methodology has a place for.

## Premise

⭐ **Read at file and line, and it corrects the specification in two
places.**
1. `TOOL.md` section 3.5 calls `userland-execve` "the second fork to
   vendor". There is no such fork.
   `references/VHSgunzo__userland-execve/PROVENANCE.md` records the
   404 beside a reachable control, and
   `references/VHSgunzo__ulexec/tree/Cargo.toml:29` shows `ulexec`
   depending on the original crate, whose repository is
   `references/io12__userland-execve-rust`. Its licence is clean:
   `tree/LICENSE` present and `tree/Cargo.toml:8` says MIT.
2. The `memfd-exec` fork carries a regression against its own
   upstream, and both trees are in the corpus so it is a diff.
   `references/novafacing__memfd-exec/tree/src/executable.rs:546`
   passes `MemFdCreateFlag::MFD_CLOEXEC` unconditionally.
   `references/VHSgunzo__memfd-exec/tree/src/executable.rs:684-691`
   makes it conditional on `fn is_running_in_qemu() -> bool { true }`,
   a stub that always returns true, so `MFD_CLOEXEC` is **never**
   set and the memfd leaks across every later `execve`. It is
   marked `// TODO: add detect for qemu emulator`.
What the fork adds and is worth having:
`references/VHSgunzo__memfd-exec/tree/src/executable.rs:574-601`
`fallback_exec()`, the launch ladder as a library, and
`references/VHSgunzo__memfd-exec/tree/src/executable.rs:694-704`,
which checks `is_exe("/proc/self/fd/<mfd>")` before writing so a
non-executable memfd becomes `EACCES` there rather than a confusing
`fexecve` failure later.

## Approach

Vendor `io12/userland-execve-rust` whole: 512 lines across six
files, MIT, licence file present. Do **not** vendor either
`memfd-exec` until its licence resolves; `memfd_create` plus
`fexecve` is three syscalls and podvm declares them itself, with
the `is_exe` probe and the `MFD_CLOEXEC` flag both taken as
mechanisms rather than as code.
⚠ Carry the stack-size costing from
`references/io12__userland-execve-rust`'s tracker: issue #1 records
that the synthetic stack was 1 MB and GCC's `cc1plus` overflowed
it, fixed by raising it to 10 MB, because GCC normally raises its
own stack with `setrlimit()` and a synthetic stack does not honour
that. M5 builds a C toolchain project in ten distributions, so this
is on podvm's path. Its issue #5 is open: non-PIE ELF binaries
cannot be loaded by that rung at all.

## Decision

Vendor `userland-execve` and write podvm's own three-syscall memfd
path, rather than vendoring `memfd-exec` once its licence resolves.
The fork's value is two mechanisms and a ladder, all of which are
smaller re-implemented than carried, and carrying it would import a
regression this tree would then have to patch out
(the `MFD_CLOEXEC` stub above) for no gain.
Status note: **open.** The `blocked` label came off on 2026-09-11, and it never
fitted. ⛔ The `Decision` above already rules that neither
`memfd-exec` tree is vendored, whatever its licence resolves to, so
the licence is a fact this entry no longer needs. An entry blocked
on evidence for a path it has decided not to take is an entry that
blocks itself.
⚠ What remains is all doable now: vendor
`io12/userland-execve-rust` under
[`../docs/methodology/vendoring.md`](../docs/methodology/vendoring.md),
and write podvm's own `memfd_create` plus `fexecve` path with the
`is_exe` probe and `MFD_CLOEXEC` taken as mechanisms rather than as
code. ⚠ Carry the 10 MB synthetic-stack figure from the tracker:
M5 builds a C toolchain in ten distributions and `cc1plus`
overflowed 1 MB.
⛔ Nothing is opened on anybody's repository to unblock it. What
would clear it is a licence file appearing in either tree at a later
commit, checked at the next reconciliation.

## Prove

`test -f vendor/userland-execve/LICENSE && grep -q '^license = "MIT"' vendor/userland-execve/Cargo.toml && ./experiments/110-bloat-delta.sh memfd` exits 0

## Done

**Done 2026-09-22.** The `Prove` ran green in the lane: the two file legs
hold on this tree, and `110-bloat-delta.sh memfd` exits 0 with
`experiments/results/bloat-memfd.txt` committed. Five answers:

1. What it replaces. `src/memfd.rs` of `pod-enter` (PodUtils/podcrates): `MFD_CLOEXEC` as
   the unconditional base, `MFD_ALLOW_SEALING` with an `EINVAL` fallback to
   the base, the `has_shebang` router predicate, the `is_executable` probe
   (mode bits plus `faccessat` `X_OK`), and `memfd_create`, with four unit
   tests watched fail before the implementation and pass after. Beside it,
   `faccessat` joins the `*at` wrappers in `pod_hostprobe::sys`
   with `SYS_FACCESSAT` and `X_OK`. The `fexecve` call itself is not here:
   it takes the argv and env arrays, and it belongs to the ladder that owns
   them ([T-1003](packaging.md)). Vendored whole beside it:
   `vendor/userland-execve` is `io12/userland-execve-rust` at `02ef0e0`,
   six source files plus `LICENSE`, `Cargo.toml` and `README.md`,
   byte-identical to the corpus tree (`Cargo.lock` and `.gitignore` left
   out). It is not compiled: nothing depends on it yet.
2. What it costs. The committed reading is total 2757488 with 97
   third-party crates, the scaffold control answering
   (`wrote=10 exe=true script=true cloexec=0x1`). This change's own cost is
   a subtraction between two lane runs minutes apart on the same image:
   the scaffold-reached tree reads total 2757488 with the control
   answering, and the same tree with the scaffold removed re-ran exit 0
   logging the same delta (2261304 against the same baseline), which is
   the same total to the byte, so the reached
   memfd path costs **0 bytes, below the instrument's resolution**, with
   the control answering. That is the same ruling shape as [T-0901](#t-0901-sweep-syscalls).
   No new third-party crate: the path uses only the taken `syscalls` and
   `linux-raw-sys` pins.
3. Does it pull C. No. The path issues raw syscalls through podvm's own
   trap. The vendored tree names `goblin` 0.8 and `nix` 0.27 in its
   `Cargo.toml`; the pins are carried, never built. Wiring it ([T-1003](packaging.md))
   first patches both out: `goblin` is refused by [T-0908](#t-0908-sweep-digests-json-argument-parsing-elf)
   and the program-header walk it rules replaces it, and the `sys` module
   here replaces `nix`.
4. Maintained and licence. `io12/userland-execve-rust` is MIT with the
   licence file present ([reference-map.md](reference-map.md) row). Niche
   and on a seam podvm will move, so it vendors. `memfd-ng` 0.1.1 is 0BSD
   and the operator's own; read at source, it sets `MFD_CLOEXEC`
   unconditionally (`tree/src/sys.rs:141-149`) and carries the tmpfs
   fallback this entry takes as mechanisms, not code. Neither `memfd-exec`
   tree is vendored, per the `Decision`.
5. Vendor or registry. Vendor `userland-execve` whole, and the three-syscall
   memfd path is podvm's own. Registry: nothing new.

⚠ The 10 MB synthetic-stack figure rides in the vendored file
(`vendor/userland-execve/src/stack.rs:152`) unchanged, with the tracker's
`cc1plus` reason beside it in the `Approach`. [T-1003](packaging.md) owns
the ladder review that decides whether it stays.

⭐ The retire check is a command, not a judgement: `diff -r
references/io12__userland-execve-rust/tree/src vendor/userland-execve/src`
plus `cmp` of `LICENSE`, `Cargo.toml` and `README.md` against the same
tree. All seven read identical today. A future upstream release retires no
patch on its own; reconcile it by reading per
[`../docs/methodology/vendoring.md`](../docs/methodology/vendoring.md).

# T-0910: The `cargo bloat` baseline, committed, and checked at the gate

**Source:** `TOOL.md` section 3.5; `docs/methodology/experiments.md`
**Category:** deps
**Milestone:** none
**Priority:** P0
**Effort:** S
**Status:** done

## Problem

⛔ A dependency that lands without a before-and-after number has not
landed. Without a committed baseline there is no "before".

## Premise

Measured, for the empty skeleton, on this host: the release binary
is 389,656 bytes with no dependencies at all. That is the baseline
every delta below is measured against, and it is quoted here as one
machine on one day, as
`docs/conventions/prose.md:1-20` requires.

## Approach

`experiments/110-bloat-delta.sh <area>` takes the current binary
size and the `cargo bloat` breakdown, with and without the
dependency under test, and writes both to
`experiments/results/bloat-<area>.txt`. It prints its conditions and
exits 0, 1 or 2 per the convention.
⭐ Wire the total into the gate as a ceiling, so a dependency that
lands quietly fails CI rather than being noticed at release.

## Decision

A ceiling on the total, not on the delta. A delta ceiling permits
an unbounded number of small dependencies, which is how a binary
gets large without any single decision being wrong.

## Prove

`./experiments/110-bloat-delta.sh baseline` exits 0, `./target/release/podvm-gate` exits 0, and `./target/release/podvm-plant` reports cases 17a, 17b and 17c caught

## Done

**Done 2026-09-08.** The `Prove` commands were run and exit 0. The reading is
`experiments/results/bloat-baseline.txt`: **496,184 bytes and zero third-party
crates**, against 389,656 for the empty skeleton of T-1100. M0's probe is the
whole of the difference and it took no dependency, which is the measurement
[T-0901](#t-0901-sweep-syscalls) asked for before a syscall crate is considered.

⚠ **The ceiling moved house rather than moving.** Its value is unchanged; where
it lives is not. It was a bare literal in the gate workflow with nothing behind
it, and the same number was quoted in this entry's own `Prove`. It now lives in
`CEILING_BYTES` in `experiments/110-bloat-delta.sh`, the workflow calls that
script, and check 17 of `crates/podvm-gate/src/main.rs` refuses a tree where any other
file this project wrote names the number. ⭐ That check fired on the first run,
against the sentence you are reading, which is why neither it nor the `Prove`
above spells the number out: under the check they were the second and third
copies.

⚠ The `cargo bloat` breakdown is taken from a **second build of the same
profile with `strip` turned off**, into its own target directory, because
`strip = "symbols"` leaves cargo-bloat nothing to read. Its proportions are the
artefact's; its absolute sizes are not, and the file says so. Where
`cargo-bloat` is not installed the script exits 2 and records that the
breakdown was not taken, because a skip is not a pass.

# T-0911: The syscall table and the kernel structs come from a crate, per architecture

**Source:** Found by the operator on 2026-09-09, reading `pod_hostprobe::sys`
**Category:** deps
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

⛔ **podvm was a single-architecture runtime because of a table
somebody typed.** `pod_hostprobe::sys` declared 46
x86_64 syscall numbers and a `#[repr(C)] struct stat` by hand, and
opened with a `compile_error!` for every other architecture. The
`compile_error!` was the right call given the table, a wrong
number is a verdict reported for a syscall nobody named, but the
table was the wrong thing to have. docker and podman run every
architecture Linux ships; podvm refused to compile for seven of
them.

## Premise

⭐ **Measured on 2026-09-09, and the premise it overturns is
[T-0901](#t-0901-sweep-syscalls)'s.** T-0901 ruled `libc` and
`rustix` out at a delta below the instrument's resolution, on the
grounds that the probe set is hand-declared and needs no crate.
What that ruling priced was the bytes. What it did not price was
the cost of hand-declaring, which is one architecture.

⚠ The requirement T-0901 was protecting is real and survives:
[probe.md](probe.md) T-0101 needs **the kernel's** errno, not a
libc wrapper's, because glibc's `setuid(3)` runs an id-change dance
and returns the library's answer. `syscalls::raw` is not a libc
wrapper: it issues the instruction and returns the raw value, so
nothing passes through `errno`'s thread-local. The rule is kept and
the table is gone.

| | |
| --- | --- |
| architectures that compiled the workspace, before | **1** |
| after | **6**, and one named blocker |
| shipping binary, x86_64, before | 2,282,224 bytes |
| after | **2,282,352 bytes**, +128 |

⭐ **The size question answered itself.** The operator ruled that
features outrank bytes here, and the trade did not have to be made:
both crates are `const` tables and inlined assembly, so `+128 bytes`
buys six architectures. ⚠ `+128` is one build on one host and is
below anything this instrument resolves; it is quoted as the
reading it is and not as a law.

## Approach

`syscalls` for the numbers and the trap, `linux-raw-sys` for the
kernel structs. Both `default-features = false`, both `no_std`,
neither pulling a libc.
⛔ **What replaced the transcription risk rather than removing it.**
A name a kernel does not define is a **compile error**, never a
wrong number: `Sysno` has no such variant. Three cfg lists remain
and each is a claim about the kernel that the compiler checks:
1. the `asm-generic/unistd.h` architectures, which have no `open`,
   `stat`, `mkdir`, `unlink`, `chown`, `lchown`, `readlink`,
   `mknod` or `dup2`;
2. the three names one syscall has, `newfstatat`, `fstatat` and
   `fstatat64`, **paired with the struct each writes**, because on
   a 32-bit architecture `fstatat64` writes `struct stat64` and
   taking it with `struct stat`'s buffer is a kernel write of the
   wrong shape into a right-sized hole;
3. nothing else.
Every path-taking **utility** routes through the `*at` form on every
architecture, which is what `open(path)` means anyway and removes
nine per-architecture cases. ⛔ The three calls where the entry
point **is** the measurement, `chown`, `lchown`, `mknod`, keep
their identity in `sys::Entry`, and on an architecture that has
only `fchownat` the row prints `fchownat`. A probe that calls one
syscall and names another is the dishonesty the probe set exists to
refuse.

## Decision

Take the crates. ⚠ **`syscalls` 0.8.1 gates powerpc, s390x and mips
behind `asm_experimental_arch`, and that gate is stale**: clause 3
of `experiments/260-multiarch.sh` compiles powerpc64 inline
assembly on stable rustc 1.98.1 and it succeeds. So
`powerpc64le-unknown-linux-musl` does not build, for a reason that
is not a rustc limit and is not podvm's code.
⛔ It is **not closed as somebody else's problem**
(`AGENTS.md` absolute 5). It stays named in clause 2 of that
script, which goes **red** the day it starts building, and either of
two things clears it: a `syscalls` release dropping the gate, or
podvm taking the numbers from `linux-raw-sys`, which carries
`__NR_*` for every architecture with no gate at all, and carrying
its own six-line trap for those two. The second is entirely within
this tree and is why the first is not a dependency on anyone.
⚠ `AGENTS.md` absolute 2 says a defect in vendored code is
fixed here. This crate is a registry dependency and not vendored,
so the fix that applies is the second option, and it is recorded
rather than taken because six architectures ship today without it.

## Prove

`./experiments/260-multiarch.sh` exits 0, and `cargo check --workspace --target aarch64-unknown-linux-musl` exits 0

## Done

**Done 2026-09-09.** `experiments/260-multiarch.sh` exits 0 and
`experiments/results/multiarch.txt` is its reading.

⭐ **podvm does not merely compile for aarch64, it runs there**, and clause 4
drives `podvm probe` under `qemu-aarch64` and reads back a rung, an identity
block and free-space rows. ⛔ **And clause 4 is a warning rather than a win.**
A probe run under `qemu-user` measures **QEMU**: it answers `EINVAL` to
`clone(CLONE_NEWNS)`, `ENOSYS` to `fsmount`, and reports `Seccomp: 0` whatever
the host is under. The rung it prints is the emulator's and podvm may not
report it as the machine's. That is the same mistake as the reconstruction's
`/dev` answering out of the mount table the question doubts, and it matters more
here: a foreign-architecture container runs its payload under exactly this
emulator, so [enter.md](enter.md) T-0506 owns saying so at runtime.

⭐ **What clause 4 does prove** is the part that could have been silently wrong:
the numbers, the struct layouts and the trap are right for aarch64. A `statfs`
layout off by one field gives garbage block counts, and the rows are sane.

⚠ **One trap cost the session real time and is written into clause 5 so it
cannot cost another.** A `binfmt_misc` magic emitted as **raw bytes** is
truncated by the kernel's parser at the first NUL, leaving a 7-byte magic that
matches **every** 64-bit ELF. Every native binary on the machine is then routed
to the aarch64 interpreter and dies with `ELOOP`, including the shell needed to
undo it. The registration string is written with backslash escapes the **kernel**
parses, and clause 5 reads the magic back and refuses to run a payload unless it
is the full 40 hex characters.

⭐ **2026-09-09, the operator pointed at
`pkgforge-dev/docker-archlinux:.github/workflows/build-deploy.yml`, line 226
onwards, as bearing on the `powerpc64le` blocker. It bears on a DIFFERENT
blocker, and saying which is the finding.**

| axis | what stops podvm | what the workflow is about |
| --- | --- | --- |
| **compile** | `syscalls` 0.8.1 gates powerpc, s390x and mips behind `asm_experimental_arch`, so `cargo check --target powerpc64le-unknown-linux-musl` dies with `error[E0554]: #![feature] may not be used on the stable release channel` | nothing |
| **run** | nothing yet, because nothing builds | `docker/setup-qemu-action` ships nine emulators and `qemu-ppc`/`qemu-ppc64` are not among them, so a big-endian PowerPC image has no interpreter unless one is registered by hand |

⛔ **And the compile blocker was already measured NOT to be a rustc limit.**
Clause 3 of `experiments/260-multiarch.sh` asks rustc directly rather than
believing the cfg, and `experiments/results/multiarch.txt` records the answer:
powerpc64 inline assembly **compiles on stable rustc 1.98.1**. So the gate is
the crate's and it is stale, which means podvm can clear it without waiting for
anybody: take the numbers from `linux-raw-sys` and carry its own trap for these
two architectures. [T-0912](deps.md) is that work.

⭐ **What the workflow does contribute is the runtime half, and it corroborates
podvm's design rather than changing it.** It registers `qemu-ppc` through
`multiarch/qemu-user-static --reset -p yes`, asserts the registration took by
reading `/proc/sys/fs/binfmt_misc/<handler>` back, and checks the `F` flag,
because without it the kernel opens the interpreter inside the container's own
filesystem. ⚠ That is the same three things `pod_hostprobe::binfmt`
reads and the same reason `experiments/260-multiarch.sh` reads its magic back
before proceeding. podvm's answer on a machine with no such registration is
already the right one: refuse by name, with the ELF machine it looked for.

# T-0912: The powerpc gate is the crate's and it is stale, so podvm can clear it

**Source:** Measured by `experiments/260-multiarch.sh` clause 3 on 2026-09-09, and raised again by the operator on the same day
**Category:** deps
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

`powerpc64le` is the one architecture the workspace does not build
for, and [T-0911](deps.md) records it as blocked on `syscalls`
0.8.1's `asm_experimental_arch` gate. A blocker nobody can clear is
a fact; this one can be cleared here.

## Premise

⭐ **Measured, and the measurement is why this is an entry rather
than a wish.** Clause 3 of `experiments/260-multiarch.sh` compiles
powerpc64 inline assembly against the real rustc rather than
believing the crate's cfg, and
`experiments/results/multiarch.txt` records that it **compiles on
stable rustc 1.98.1**. The gate is therefore the crate's and not
the compiler's.
⚠ Clause 2 of the same script goes **red** the day `powerpc64le`
starts building, which is the point: it is how this project finds
out rather than a failure to suppress.
⚠ The runtime half is separate and is not this entry: an emulator
for big-endian PowerPC is absent from `docker/setup-qemu-action`,
which [T-0911](deps.md) now records.

## Approach

Take the syscall numbers for powerpc64, s390x and mips from
`linux-raw-sys`, which podvm already depends on for the kernel
structs, and carry podvm's own trap for those architectures in
`pod_hostprobe::sys` beside the ones it already has.
⛔ Not a fork of `syscalls` and not a patch to it in this tree: the
numbers are already available from a crate podvm takes, and one
more source of them is one more place for them to disagree.
⚠ Then flip clause 2's expectation in the same change, because a
clause asserting a blocker that has been cleared is a check that
fires on the truth.

## Decision

Clear it here rather than wait for a `syscalls` release. Waiting is
a dependency on somebody else's schedule for a value this project
has already measured, and `AGENTS.md` absolute 5 is that a
blocked entry names what would clear it.

## Prove

`./experiments/260-multiarch.sh` reports 7 architectures checking the workspace and 0 blocked, and clause 3 says the gate was the crate's

## Done

**Done 2026-09-09, and the Prove's number was low: it says seven architectures
and the reading is EIGHT.**

⭐ **What was in the way was a CRATE-LEVEL attribute, which is why no `#[cfg]`
inside podvm could route around it.** `syscalls` 0.8.1 carries
`#![feature(asm_experimental_arch)]` for `mips`, `mips64`, `s390x`, `powerpc`
and `powerpc64`, so the whole crate refuses to compile for those targets on
stable however podvm uses it. The dependency is therefore **target-gated in
`crates/podvm-probe/Cargo.toml`**, with the cfg list copied from the crate's
own: a superset would drop `syscalls` where it works, a subset would leave the
build broken where it does not.

On those five, `pod_hostprobe::sys` takes every number from
`linux-raw-sys`, which podvm already depends on for the kernel structs. ⚠ Both
spellings are on one line -- `nr!(openat, __NR_openat)` -- so a reader can check
the pair and a rename cannot silently take a different number on one
architecture.

⭐ **The trap is podvm's own for two of the five, and one of them is
EXECUTED.** `experiments/260-multiarch.sh` clause 7 builds for
`powerpc64le-unknown-linux-musl` and runs the result under
`qemu-ppc64le-static`:

```
  binary          ELF 64-bit LSB executable, 64-bit PowerPC ...
  version         podvm 0.1.0             rc=0
  reads its own   ELF machine 0x15   (0x15 is PowerPC64)
  probe --json parses and carries a rung: rc=0
```

⛔ That assertion is not decoration. `ELF machine 0x15` is a value the binary can
only produce by opening and reading its own header **through the trap this entry
wrote**, and powerpc's error convention is not x86_64's: it returns a POSITIVE
errno in `r3` and sets `CR0.SO`, so a trap that compiled and ignored that would
read every failure as a huge success. The `neg` under `bns` is what makes
`split`'s `-4095..=-1` window mean the same thing on both.

⚠ **`s390x` is CHECKED and not RUN, and the reason is recorded rather than
smoothed over**: rustup has no prebuilt `s390x-unknown-linux-musl` std, so there
is no static binary to hand `qemu-s390x-static`, and the gnu triple would need a
glibc sysroot. Its trap is written from the kernel's own convention -- r1 the
number, r2 through r7 the arguments, `svc 0`, `-errno` back in r2 -- and is
compiled, not executed. One prebuilt std or a sysroot would clear it.

⛔ **`mips`, `mips64` and 32-bit `powerpc` are REFUSED AT COMPILE TIME with a
message naming what is missing**, rather than given a convention nobody has run.
o32 passes arguments five and six on the caller's stack, which `options(nostack)`
forbids and which a wrong frame layout gets wrong silently. ⚠ `qemu-mips-static`
IS installed here, so what would clear it is a clause in `260-multiarch.sh` that
builds for a mips target and RUNS the binary -- the same bar powerpc64 cleared.

Prove, run 2026-09-09:

```
$ ./experiments/260-multiarch.sh
  == 1. every claimed architecture compiles the whole workspace
    ok  x86_64  aarch64  riscv64gc  loongarch64  armv7  i686
    ok  powerpc64le-unknown-linux-musl
    ok  s390x-unknown-linux-gnu
  == 2. the architectures that do NOT build, and exactly why
    none.
  == 7. podvm's OWN syscall trap, executed rather than only compiled
    reads its own ELF machine 0x15
  exit 2, because binfmt_misc is not mounted on this host and clause 5
  therefore could not run. ⛔ Not a failure: it is the third state.
```

# T-1316: The committed lock matches the manifests, so a clean build leaves a clean tree

**Source:** issue 11, client beta testing 2026-09-22 (every fresh build
  dirties `Cargo.lock` and the binary self-reports `-dirty`);
  `Cargo.lock`, `crates/podvm/Cargo.toml`,
  `crates/podvm-image/Cargo.toml`
**Category:** deps
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

The committed lock is stale against the committed manifests
(`podvm` declares `sha2` under `[build-dependencies]`
(T-1004), `podvm-image` declares `base64`, neither in the
lock's lists; last lock commit 2026-09-11), and this
session's T-1315 adds a third stale edge (`podvm-extract`
declares `sha2`), so the first build on a pristine clone
rewrites the lock, `build.rs` sees a dirty tree, and the
binary records `-dirty`. Every released beta asset carries
the suffix for no reason.

## Premise

Measured by the reporter on a pristine clone at the release
commit and confirmed on this tree: the two crates' lock
lists lack the two declared dependencies.

## Approach

Refresh the lock (`cargo check`, then commit the lock) in
its own change, no manifest edits. Out of scope: whether
`build.rs` should distinguish cargo-caused dirt from
hand-edited dirt (recorded as future work in the entry if
wanted, not done here).

## Decision

Lock refresh only. The `-dirty` semantics question is real
but widens the unit past one file.

## Prove

Clone pristine, `cargo build --release`, `git status
--porcelain` prints nothing and `podvm version --verbose`
reports the bare commit. Close issue 11 with a comment
showing the clean-tree build and the lock diff as the guard
(a stale lock can no longer hide: any future manifest edit
without its lock shows the same way).

## Done

**Done 2026-09-23.** Lock refresh only, as decided: three missing edges,
no manifest edits, no version moves (`podvm-image` -> `base64`,
`podvm` -> `sha2` as a `[build-dependencies]` entry,
`podvm-extract` -> `sha2`, the last new with this session's T-1315).
The entry premise stands corrected in place on those two points; the
`webpki-roots` the report implied was missing was already listed, so no
edge was added for it. The refresh ran through `cargo metadata --offline`
(resolution only, no compile: the lock carries edges, and the three added
lines are dependency edges, not versions). Proved at the lock commit
(`fd9ee37`) in the lane
(`rust:1.98.1-bookworm` through host podman 6.1.2): pristine clone,
`cargo build --release`, `git status --porcelain` prints nothing, and
`podvm version --verbose` reports the bare commit with no `-dirty`
suffix. The guard that stops recurrence is the same mechanism that
reported the defect: a future manifest edit without its lock dirties the
tree on the next build and the binary self-reports `-dirty` again.

# T-1562: Port `experiments/110-bloat-delta.sh` to `podvm-size`

**Source:** `refactor/06-entries/T-R005.md`; `TODO/INDEX.md:52`;
  `experiments/110-bloat-delta.sh`; `TODO/deps.md` T-0910
**Category:** deps
**Milestone:** none
**Priority:** P0
**Effort:** M
**Status:** done

## Problem

The artefact measurement is shell. It is the `deps`
category's ceiling instrument, and it moves into the
`podvm-release` crate. Three readers pin its path: the gate
workflow runs it, the record gate reads the ceiling out of
it, and the plant harness reads the digits. The waves cannot
move the readers without the file.

## Premise

The ceiling keeps its one home in the shell stub
(`CEILING_BYTES=<n>`), and the binary reads it at runtime:
the gate refuses the digits anywhere else (D-1 option c). The
scaffold control, the zero-delta refusal, and the shipping
profile stay as written.

## Approach

Port the measurement to binary `podvm-size` with no
dependencies. Keep the experiment path as a declaration stub
that execs the binary, so the gate job, the gate check, and
the plant harness move nothing.

## Decision

The stub declares; the binary measures. The `ci` area keeps
its meaning: measured, never asserted; the ceiling is the
gate.

## Prove

`cargo test -p podvm-release` green in the lane; the gate's
`ci` area still runs through the stub;
`./target/release/podvm-gate` exits 0.

## Done

**Done 2026-10-01.** Binary `podvm-size` measures in the
`podvm-release` crate, which it needs no dependencies for; the
experiment path stays as a declaration stub that execs the binary (D-1
option c, D-2 hoist honoured: the stub never moves). Lane proof
([release-crate-proof](../experiments/results/release-crate-proof.txt),
section 9): the workspace release build exits 0 and `podvm-size ci`
through the stub exits 0 with `total_bytes 3277880` under the declared
ceiling. The record gate exits 0 on the landed tree.

# T-1624: Record the 265-row parity figure and the four stale values the tree carries

**Source:** `refactor/recon-c.md:171`;
  `refactor/06-entries/PLAN.md:190-194`
**Category:** deps
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

The parity table has one row count and this tree carries
four values for it. A reader cannot tell which is current
and one of the four is labelled CURRENT while being wrong.

## Premise

265 is the source value. 220 and 164 are historical
readings, and 160 is a fourth. A count of 267 counted
`struct Row` and `impl Row` rather than the table's rows.

## Approach

Record 265 once, beside the other three and the reason
each is what it is, so the four stop being four answers to
one question. Check the current value in the source before
writing it.

## Decision

One page carries the figure and names the other three as
historical. Four pages each asserting a number is the defect.

## Prove

`grep -n "265" TODO/deps.md` names the parity figure once
with the three stale values beside it.

## Correction

**Recorded 2026-10-02.** One figure answers one question: how many rows
`TABLE` holds. The source reads **265**, and the plan carries the same
figure with the same range (`refactor/06-entries/PLAN.md:190`). Measured
on this tree: `TABLE` opens at `crates/podvm/src/parity.rs:119`
and closes at line 537 at `c36e0db672d2`, and 265 `Row {` literals lie between them.

The three other values are readings, not sources, and each says when it
was taken:

| Value | Where | What it is |
| --- | --- | --- |
| 265 | `crates/podvm/src/parity.rs:119-422` | the source count, current |
| 220 | `TODO/cli.md:183` | T-0801's 2026-09-25 reading |
| 164 | `experiments/results/parity-drive.txt:7` | the 2026-09-22 drive |
| 160 | `TODO/cli.md:812` | T-0808's 2026-09-21 reading |

⚠ A count of 267 is not a fourth reading. `grep -c 'Row {'` over the
whole file returns 267 because `pub struct Row` at `:58` and `impl Row`
at `:65` both match. Neither is a table row.

The live page is `TODO/cli.md:109`, which reads 265 and labels itself
CURRENT. It carries the current value and does not need this entry; the
three historical rows name it rather than assert a number, so the table
has one cell that answers and three that do not compete with it.

## Done

**Done 2026-10-02.** The 265-row figure is recorded once, with the three
other values named as dated readings beside it: 220 from `TODO/cli.md:183`,
164 from `experiments/results/parity-drive.txt:7`, and 160 from
`TODO/cli.md:812`. The value is re-derived on this tree rather than copied:
`TABLE` runs from `crates/podvm/src/parity.rs:119` to `:422` and holds
265 `Row {` literals inside that range. A fourth figure, 267, is recorded as
a whole-file artefact, because `pub struct Row` at `:58` and `impl Row` at
`:65` both match the same grep; the whole-file count is 267. The Prove grep
`grep -n "265" TODO/deps.md` matches, and each of its lines is either this
entry or the record it wrote.

# T-1633: Record the 13-versus-3 `plant.sh` `Prove` discrepancy and which count is right

**Source:** `refactor/recon-c.md:180`;
  `refactor/06-entries/PLAN.md:178-184`
**Category:** deps
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

Two documents give two counts for the same set of proof
lines, and a third says the count is wrong twice. Neither
figure has been re-derived, so a repair set built on either
is the wrong size.

## Premise

The larger figure was verified twice and the correction is
itself recorded: the lines sit in two files, not three, and
one named file contributes none. The smaller figure has no
stated derivation.

## Approach

Re-derive the count over the current tree and record the
count and its derivation together. The historical figure
stays named as historical.

## Decision

Re-derive before recording. A correction that carries the
error forward under a new number is not a correction.

## Prove

`grep -n "plant.sh" TODO/deps.md` names the current count
and the command that derives it.

## Correction

**Recorded 2026-10-02.** Re-derived over this tree, and the current
count is **0**. The historical figure is **13**, from
`refactor/recon-c.md:523` and `refactor/06-entries/PLAN.md:180-182`.

The plan's own command, run here, returns nothing and exits 1:

```sh
grep -rn 'scripts/plant.sh' TODO/*.md | grep -c 'Prove'   ->  0
```

The reason is not that the record lost the lines. The old plant harness
was ported to the binary `podvm-plant`
(`crates/podvm-gate/src/plant.rs:3`), and every mention of the retired
harness under `TODO/` carries the correction `(now \`podvm-plant\`)` or is
itself the new spelling. Outside this entry, no line under `TODO/`
names the retired harness without that correction, and none leaves a
bare backticked citation. The repair set the plan asked for has
therefore already been carried, and the `grep` its figure came from
now reads the finished state as an absence.

Derivation over the current tree, independent of the path spelling:

```sh
awk '/^### T-[0-9]+/ { id = $2; p = 0; next }
     /^[A-Z][A-Za-z-]*:/ { p = ($0 ~ /^Prove:/); next }
     /^$/ { p = 0; next }
     /^\*\*/ { p = 0; next }
     p && ($0 ~ /podvm-plant/ || $0 ~ /plant\.sh/) { print id " " FILENAME }' \
    TODO/*.md
```

It prints four entries, all in `TODO/gate.md`: T-1207, T-1338, T-1341
and T-1343. So 4 `Prove` clauses drive the plant harness on this tree,
and each already names the binary or carries the correction beside the
old path. Nothing is left to repoint.

The historical 13 and its 12 + 1 + 0 breakdown stand as historical.
They were measured against the pre-port tree, where
`refactor/06-entries/T-R004.md:119` still reads 13 `Prove:` lines
across three `TODO/` files and `PLAN.md:183-184` corrects the file
count to two. Neither file moves, because `refactor/` stays untracked
(`.gitignore:152`); this entry is the tracked record of both figures.

⚠ The 3 the title names never had a derivation. The count of 3 is a
file count, and the two documents that give one put it at 2:
`refactor/06-entries/T-R004.md:119` reads 13 lines across three files,
and `refactor/06-entries/PLAN.md:183-184` corrects that to two.
`refactor/recon-b.md:43` records the correction and its evidence, and
`refactor/recon-c.md:799-803` records the author naming the same
question as unresolved. `PLAN.md:180` carries the same 13 with no file
count beside it. No document in the corpus states 3 as a measured
result.

## Done

**Done 2026-10-02.** The entry stands on a re-derivation over this tree,
not on the premise. The current count of `Prove` clauses that drive the
plant harness is 4, not 13 and not 3: the derivation below prints T-1207,
T-1338, T-1341 and T-1343, all in `TODO/gate.md`. The historical 13 keeps
its 12 + 1 + 0 breakdown, and 3 is recorded as a file count that the two
documents giving one put at 2. The reason the figure moved is recorded:
the harness was ported to binary `podvm-plant`
(`crates/podvm-gate/src/plant.rs:3`), so the plan's own command now reads
a finished repair set as an absence. The Prove grep
`grep -n "plant.sh" TODO/deps.md` matches, and each of its lines is either
this entry or the record it wrote.

# T-1642: Refuse a Linux-target cargo build launched on a Windows host

**Source:** measured 2026-10-02 on this host; `.cargo/config.toml:7-8`;
  `docs/containers.md:33`; `scripts/dev-lane.sh:1`;
  `scripts/common/check-build-lane.sh`
**Category:** deps
**Milestone:** none
**Priority:** P1
**Effort:** S
**Status:** done

## Problem

A Linux build run on the Windows host fails deep inside a
C dependency, and the failure reads like a host limitation.
It is not one, and this session reported it as one.

## Premise

`.cargo/config.toml` sets `[build] target =
"x86_64-unknown-linux-musl"`, so a bare `cargo build`
compiles a Linux target on every host including a Windows
one. `ring`'s build script then invokes
`scripts/zig-cc.sh` through a Windows process spawner:
`os error 193`. `cargo check` and `cargo clippy` never link,
so they pass on the same host, which is why the difference
is invisible until a link is attempted.

## Approach

Make the refusal mechanical rather than documented. A guard
decides from the host and the pinned `[build] target`, and
refuses `build`, `test`, `run` and `bench` when that target
is a Linux triple. It allows `check`, `clippy`, `fmt` and a
Windows `--target`, because `check-gate.sh` builds the gate
as `x86_64-pc-windows-msvc` and a guard that refused that
would make the tree unverifiable on the host.

## Decision

Refuse on the PLACE, not on the error. The error is loud and
harmless on its own; the damage is what an agent concludes
from it. This session confirmed the failure reproduced on a
clean tree and wrote "pre-existing, not introduced here" into
three records. Every observation was true and the conclusion
was worthless, because the build had run in the wrong place.
The guard therefore runs before cargo, in 0.665 s measured,
and its message names the lane and says the error is not a
host limitation.

## Prove

`sh scripts/common/check-build-lane.sh --self-test` exits 0
with 16 cases, 8 per host, driven through a stubbed
`uname`: a Windows host refuses a bare build, a Linux
`--target` build and `cargo test`, and allows `check`,
`clippy`, `fmt`, a Windows `--target` and `cargo tree`; a
Linux host allows all eight, which is what keeps the CI
workflows valid at `gate.yml:45`. The plant: restoring
revision one's `[ "$u" != "MINGW"* ]` turns the self-test
red with exit 1, measured. `check-gate.sh` runs the
self-test so the guard cannot rot unobserved.

## Done

**Done 2026-10-02.** The guard is `scripts/common/check-build-lane.sh` and it is
wired into `check-gate.sh:116` as `check-build-lane`, running its
self-test rather than its own judgment of the host's command, because
running the guard against this host's command would pass by
construction and prove nothing. Its first revision compared
`[ "$uname_out" != "MINGW"* ]`, where the quoted word leaves `*`
literal, so the comparison never matched and every Windows host took
the allow branch; it shipped that way and reported this session's exact
offending command as allowed. The self-test caught it when the case was
rewritten as `case`, and the mutation was then re-measured: restoring
the `[ ]` form turns the self-test red with exit 1, so the guard is
known to detect its own regression rather than merely to exist. Both
directions are measured; a guard that only ever passed was the failure
this entry is about.

⚠ **What this does not do.** It refuses the mistake; it does not make
the right command easy to find. A reader still has to reach for
`sh scripts/dev-lane.sh run JOB.sh` on their own, and AGENTS.md:28 is
one line in a list. That is why the entry keeps a documentation half
beside the guard half, and why neither is described as sufficient.

# T-1669: Pin the stable Rust of the day and edition 2024, before the first `pod-*` crate

**Source:** call C6 of the decisions page of 2026-11-06 (the main session
  adds it as docs/decisions.md); COORD-001, issue 194, "podvm
  moves to the pinned stable Rust and edition 2024 before its
  first take"; `rust-toolchain.toml:2`; `Cargo.toml:28-29`;
  `crates/podvm-interpose/Cargo.toml:4-5`
**Category:** deps
**Milestone:** none
**Priority:** P1
**Effort:** M
**Status:** done

## Problem

The toolchain floats on `stable`, so the compiler changes under
the tree without a commit. Stable moved from 1.98.1 to 1.99.0
between two CI runs on 2026-10-01 and turned the static build
red (T-1601). The podcrates crates that podvm takes next build
on a pinned stable and edition 2024; podvm is on edition 2021.

## Premise

`rust-toolchain.toml:2` reads `channel = "stable"`. The
workspace sets `edition = "2021"` and `rust-version = "1.85"`
(`Cargo.toml:28-29`). The interposer is outside the workspace
and sets the same two values
(`crates/podvm-interpose/Cargo.toml:4-5`). T-1601 recorded "do
not pin the toolchain" under the 2026-10-02 rule of
`TODO/RULES.md` section 11; call C6 of 2026-11-06 orders the
pin.

## Approach

Read the stable version of the day of the change with `rustup
check`. Set `channel` to that exact version, and keep the
components and the musl target. Set `edition = "2024"` and
`rust-version` to the pinned version in the workspace and in
the interposer. Run `cargo fix --edition` for each, then fix
what remains by hand. Run the full lane: fmt, clippy, tests,
the static build and both interposer objects. A later bump is
one commit that changes the pin and passes the same lane.

## Decision

C6 settles the pin; it supersedes T-1601's "do not pin" for the
channel only. The rule "take whatever toolchain feature unlocks
the task" of `TODO/RULES.md` section 11 stays: a bump to a newer
pin is how a feature is taken. This entry comes before the
first `pod-*` take (T-1675) and before the record moves to
pod-todo (T-1672).

## Prove

`cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings` and `cargo test --workspace` each
exit 0 in the Linux lane (`sh scripts/dev-lane.sh run JOB.sh`)
on the pinned toolchain; `./target/release/podvm-interpose-build`
exits 0 with both objects under the ceiling; and
`rustc --version` in the lane prints the version that
`rust-toolchain.toml` names.

## Done

**Done 2026-11-06.** `cargo test --workspace` exits 0 in a Linux
cloud container on the pinned 1.99.1, with no test failing; fmt, clippy,
both interposer objects and `rustc --version` hold as recorded below.

**Partial, 2026-11-06.** `rust-toolchain.toml` pins `channel = "1.99"`,
the toolchain of podcrates; `rustc --version` prints 1.99.1. The
workspace and the interposer set `edition = "2024"` and
`rust-version = "1.99"`. `cargo fix --edition` and `cargo clippy --fix`
made the changes, and `rustfmt.toml` keeps the 2021 style, so the record's
line citations and the one-line rows of the parity table stay. The tests
that change the environment take an `unsafe` block; podvm-ssh's `detect`
takes the value as an argument in its test, and podvm-windows forbids
unsafe code outside its test modules. A rename defect came to light:
the OEM name of the FAT16 builder was 7 bytes, so 13 tests of
podvm-windows panicked; it is 8 bytes again. Results in a Linux cloud
container: `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, the interposer's clippy, its 50 tests
with one thread, `scripts/build-interpose.sh` (both objects, 330,288
bytes under the 500,000 ceiling), the static-pie release `podvm` with no
`PT_INTERP`, and `./target/release/podvm-gate` each exit 0. The
interposer's tests fail with parallel threads (2 of 2 runs, also before
the change), so `gate.yml` runs them with one thread.
The remaining clause closed on 2026-11-06: the detached start passes
since `4c2cb91` (T-1604) and the device tests since T-1686.

# T-1675: Take `pod-term` for the SSH line discipline

**Source:** COORD-001, issue 194, "What podvm takes, in this order", row 1;
  PodUtils podcrates issue 3; `crates/podvm-ssh/src/session.rs:1`
**Category:** deps
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

podvm carries its own terminal line discipline in
`crates/podvm-ssh/src/session.rs` (1,132 lines). podcrates
makes the same code a shared crate, `pod-term`, so podnet,
podssh and podvm keep one copy.

## Premise

COORD-001 puts `pod-term` first because it needs no tokio.
podcrates recorded the order in its answer (podcrates issue 3)
and hands each crate over as `adoption/podvm/CRATE.patch` and
`.run` at the tag of its release (podcrates podcrates@T-040). The pinned
toolchain comes first (T-1669).

## Approach

When the release is out, apply its adoption patch: depend on
`pod-term` at the release version and replace the line
discipline in `session.rs` with calls to the crate. Keep the
existing session tests and run them against the crate. Write
the commit that took the crate as a COORD issue `(to
podcrates)`, as podcrates issue 3 asks.

## Decision

podvm takes a released version, never a path or a git
dependency. A defect found in the crate goes to podcrates as a
COORD issue; podvm does not patch a copy.

## Prove

`cargo test -p podvm-ssh` exits 0 in the Linux lane with the
session tests passing through `pod-term`, and
`./target/release/podvm-gate` exits 0.

## Done

**Done 2026-11-06.** podvm took `pod-term` with
`adoption/podvm/pod-term.patch` of PodUtils/podcrates at `784469a`,
applied with `git apply --3way` and no conflict. The dependency is
`tag = "v0.3.0"`, because the tag `v0.4.0` does not exist yet and
`pod-term` has no change of code since `v0.3.0`. `session.rs` takes
`pod_term::echo` and keeps its supervisor. Each line of `pod-term.run`
exited 0 in a Linux cloud container on Rust 1.99.1: the 29 tests of the
discipline, the 12 tests of `session_interactive` against an installed
sshd, clippy of the workspace, rustfmt of podvm-ssh, the record gate,
`check-consumer-toolchains.py` (2 pins, 0 below 1.99) and
`check-static.sh` of the musl binary `shell`. The decision "a released
version, never a git dependency" holds with a git tag; the crate is not
on a registry.

# T-1676: Take `pod-tls` for both TLS stacks, with its synchronous face

**Source:** COORD-001, issue 194, row 2; PodUtils podcrates issue 3,
  question 1 (call C20 of podcrates); `crates/podvm-ssh/src/tls.rs:1`;
  `crates/podvm-image/src/tls.rs:1`
**Category:** deps
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** done

## Problem

podvm has two TLS setups: `crates/podvm-ssh/src/tls.rs` (153
lines) and `crates/podvm-image/src/tls.rs` (290 lines). It
needs TLS 1.2, RSA and `ring` for intercepting proxies.
podcrates makes this `pod-tls`.

## Premise

podvm is synchronous and measures the size of each dependency.
podcrates answered question 1: with `default-features = false`,
`pod-tls` builds its configurations, roots and stores with no
tokio, and podvm drives rustls's own synchronous stream.
`pod-rootfs` (T-1681) reads the CA data that moves to
`pod-tls`.

## Approach

Take the released `pod-tls` with `default-features = false`.
Replace both modules with the crate's configuration and roots.
Measure the binary size before and after with
`./target/release/podvm-size` and record both. Report the
taken commit in a COORD issue `(to podcrates)`.

## Decision

No tokio enters podvm through this crate. A size growth past
the ceiling the gate checks is a blocker to raise with
podcrates, not a ceiling to move.

## Prove

`cargo test -p podvm-image` and `cargo test -p podvm-ssh` exit
0 in the Linux lane, `cargo tree -p podvm -e normal -i tokio`
finds no tokio, and `./target/release/podvm-gate` exits 0.

## Done

**Done 2026-11-06.** podvm took `pod-tls` 0.4.0 with
`default-features = false` by `adoption/podvm/pod-tls.patch` of
PodUtils/podcrates (`main` at `deacfef`, made at podvm `b5473a2`). The
patch applied with `git apply --3way`; its one hunk of
`src/pkg.rs` of `pod-rootfs` (PodUtils/podcrates) is a comment of a test that T-1674 had
moved to `pkg/tests.rs`, and was applied there by hand. `pod-term` moves
to `v0.4.0`; `rustls-pemfile` and podvm's own `webpki-roots` are gone, and
`cargo update -p webpki-roots@1.0.9` gives the lock 1.0.11. Each line of
`pod-tls.run` exits 0 in a Linux container: podvm-ssh 101 tests,
podvm-image 23 and podvm-complete 15 of the TLS paths, `mux_two_client`
15 against sshd, clippy, rustfmt, the runtime-state rows, the record
gate, `check-consumer-toolchains.py` and `check-static.sh` of `node` and
`podvm`. `cargo test -p podvm-image -p podvm-ssh` passes, and `cargo tree
-p podvm -e normal -i tokio` finds no tokio. Release sizes, before and
after: `podvm` 3,974,200 and 4,006,968 bytes (+32 KiB), `node` 1,622,872
and 1,672,024 (+48 KiB), `operator` 1,594,264 and 1,635,224 (+40 KiB).
Call C9 of `docs/decisions.md` accepts the three changes of behaviour.

# T-1677: Take `pod-proxy` with feature `blocking`: HTTPS_PROXY egress for pulls and the SSH dial

**Source:** COORD-001, issue 194, row 3; PodUtils podcrates issue 3,
  question 1 (podcrates podcrates@T-063); `crates/podvm-image/src/transport.rs:1`;
  `crates/podvm-ssh/src/transport.rs:226`
**Category:** deps
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

podvm dials registries and SSH targets directly
(`crates/podvm-ssh/src/transport.rs:226`). It reads no
`HTTPS_PROXY` and no `NO_PROXY`, so on a host whose only way
out is a proxy, `pull` and the guest image fetch cannot reach
the network.

## Premise

podcrates answered question 1 with a feature `blocking`: a
blocking `Dialer` and `Client` over a private current-thread
runtime, on the same code as the async ones (call C20). Where
podvm's measurement asks for no tokio, the protocol parts
become state machines that a synchronous driver runs.
podcrates records that as podcrates@T-063, before podvm takes the crate.

## Approach

Take the released `pod-proxy` with `blocking`. Route the
registry transport, the SSH dial and the guest image fetch
through its `Dialer`, honouring `HTTPS_PROXY`, `HTTP_PROXY`,
`ALL_PROXY` and `NO_PROXY`. Measure the size, and the count of
threads during a pull. Report the taken commit.

## Decision

The proxy credential never appears in argv, in a log line or in
an error message. Whether the private runtime is acceptable is
decided by the measurement: where it is not, podvm waits for the
state-machine form of podcrates podcrates@T-063.

## Prove

`cargo test -p podvm-image` exits 0 in the Linux lane with a
test that pulls through a local CONNECT proxy fixture and
bypasses it for a `NO_PROXY` host, and
`./target/release/podvm-gate` exits 0.

## Done

**Done 2026-11-06.** podvm took the registry patch
`adoption/podvm/pod-proxy-registry.patch` of PodUtils/podcrates (`main`
at `e6a609b`, made at podvm `f3eccd8`), applied with `git apply --3way`
and no conflict. Each HTTP request of podvm goes through
`pod_proxy::blocking::Client::send_streamed` (`crates/podvm-image/src/net.rs`):
the registry, `podvm windows fetch` and the mirror probe of
podvm-complete; ureq leaves the graph. `tests/proxy_pull.rs` pulls the
registry fixture of podvm-image through a CONNECT stand-in named by `HTTPS_PROXY`, with
the registry's name resolvable only through the proxy, and bypasses it
for a `NO_PROXY` host. Each line of `pod-proxy-registry.run` exits 0:
`cargo test -p podvm-image` (156 unit tests and each integration test),
the Windows fetch 21, `store_gates` 4, podvm-complete 15, podvm-ssh 107
and `mux_two_client` 15 against sshd, clippy, rustfmt, the runtime-state
rows, `./target/release/podvm-gate`, the toolchains, ureq absent, and
the static check of `node` and `podvm`. Release sizes: `podvm` 4,011,064
to 3,974,296 bytes (-36 KiB), `node` 1,913,816, `operator` 1,868,824.
Call C10 of `docs/decisions.md` takes the runtime into the binaries and
the two known gaps (podcrates podcrates@T-075, podcrates@T-077). The count of threads during
a pull was not measured: the runtime is current-thread.
Gate run 42227206423 at `826bb86` is green in each job.

**Partial, 2026-11-06.** podvm took `pod-proxy` 0.4.0 with feature
`blocking` by `adoption/podvm/pod-proxy.patch` of PodUtils/podcrates
(`main` at `d703263`, made at podvm `f9eae80`), applied with `git apply
--3way` and no conflict. Both relay legs of podvm-ssh dial through
`Dialer::from_environment` (`crates/podvm-ssh/src/transport/egress.rs`):
`https_proxy`, `HTTPS_PROXY`, `all_proxy`, `ALL_PROXY` and `NO_PROXY`,
over HTTP CONNECT or SOCKS5; a loopback or `NO_PROXY` target goes direct
through podvm's own dial. Each line of `pod-proxy.run` exits 0 in a Linux
container: podvm-ssh 107 tests (6 new: CONNECT by `HTTPS_PROXY` and by
`ALL_PROXY`, loopback and `NO_PROXY` direct, an unusable setting refused,
no runtime with no proxy), `mux_two_client` 15 against sshd, clippy,
rustfmt, the runtime-state rows, the record gate, the toolchains and the
static check of `node`, `operator` and `podvm`. `cargo tree -p podvm -e
normal -i tokio` finds no tokio; podvm-ssh has it through `pod-core`.
Release sizes: `node` 1,672,024 to 1,913,816 bytes (+236 KiB),
`operator` 1,635,224 to 1,864,728 (+224 KiB); `podvm` 4,011,064. Call C10
of `docs/decisions.md` accepts the runtime and the two changes of
behaviour.
The registry and the guest image fetch followed with the registry patch
above.

# T-1678: Take `pod-relay` with feature `blocking` for the relay, keeping `reverse-v1`

**Source:** COORD-001, issue 194, row 4; PodUtils podcrates issue 3;
  `crates/podvm-ssh/src/mux.rs:1`; `crates/podvm-ssh/src/ws.rs:1`
**Category:** deps
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** blocked

## Problem

podvm carries its own relay: `crates/podvm-ssh/src/mux.rs`
(1,356 lines), `crates/podvm-ssh/src/ws.rs` (1,051 lines), and a
pairing through curl in `crates/podvm/src/remote/`. podcrates
makes the relay a shared crate, `pod-relay`.

## Premise

COORD-001 takes `pod-relay` with feature `blocking` when
podcrates' wave M2 lands, and the protocol keeps podvm's live
`reverse-v1`. T-1406 is partial on live-relay pairing.

## Approach

Take the released crate. Replace `mux.rs`, `ws.rs` and the curl
pairing with its blocking client. Keep `reverse-v1` on the
wire, and prove it against a relay that runs the version podvm
shipped. Report the taken commit.

## Decision

The wire protocol does not change in this entry. A protocol
change is its own entry with a version bump.

## Prove

`cargo test -p podvm-ssh` exits 0 in the Linux lane with the
mux and relay tests passing through `pod-relay`, and
`./target/x86_64-unknown-linux-musl/release/podvm-gate` exits 0.

## Blocker

**Blocked 2026-11-06.** Blocker: podcrates' wave M2 and a `pod-relay`
release with feature `blocking`. Clears when: that release and its
adoption patch exist, and T-1677 is done.
**Still blocked, 2026-11-10.** T-1677 is `done`. podcrates `v0.14.0` has
no crate `pod-relay`. Its `v0.13.0` adds `pod-ws`, with the client of the
relay, for podssh and podnet; the notes of that release say "podvm needs
no code change". The blocker stays: a `pod-relay` release, or a podcrates
hand-off that names the crate podvm takes for its relay.

# T-1679: Take `pod-services` for the third-party hosts compiled into podvm

**Source:** COORD-001, issue 194, row 5; PodUtils podcrates issue 3, "One
  ask of podvm" (podcrates podcrates@T-054); COORD-003, issue 196
**Category:** deps
**Milestone:** none
**Priority:** P2
**Effort:** M
**Status:** partial

## Problem

The relay URL, the registry endpoints and the guest image
sources are compiled into podvm in several places. A changed
host means a source edit in each place, and no list says which
hosts podvm contacts.

## Premise

podcrates' `pod-services` holds the hosts of other parties for
each consumer. podcrates asked for podvm's inventory of those
hosts; COORD-003, issue 196, sends it. podcrates podcrates@T-054 takes
that inventory into the crate.

## Approach

When the release carries podvm's hosts, replace each compiled
host with a read from `pod-services`. Keep a test that lists
every host podvm can contact and compares it with the crate's
list for podvm.

## Decision

A host podvm contacts and the crate does not list is a defect
of the inventory, reported to podcrates. podvm does not keep a
second list.

## Prove

`cargo test -p podvm` exits 0 with the host-list test, and
`./target/x86_64-unknown-linux-musl/release/podvm-gate` exits 0.

## Correction

**Blocked 2026-11-06.** Blocker: podcrates has not released
`pod-services` with podvm's inventory (podcrates podcrates@T-054). Clears when:
that release exists, and T-1678 is done.
**Unblocked 2026-11-10.** `pod-services` at podcrates `v0.10.0` and later
carries podvm's inventory: its README lists each compiled host of podvm
with the data file that holds it, and `tests/podvm.rs` reads podvm's
constants back from the files. podvm takes podcrates at `v0.12.0`. The
second condition is dropped: the README says that podvm has no default
relay, so the relay row needs no data file and this entry does not wait
for T-1678. The status moves from `blocked` to `open`.

## Done

**Partial 2026-11-11.** podvm takes `pod-services` at podcrates `v0.22.0`
and reads each host of another party that the crate lists: the netboot
archive of the machine tier and the SHA-256 of its kernel
(`crates/podvm/src/machine/guest/kernel.rs`, `alpine-netboot`), the same
values and the pinned image of the measure workload
(`crates/podvm-measure/src/workload.rs`, `ecr-public-registry`
`guest-userland`), and the default server of `podvm login` and `logout`
(`docker-hub-registry` `credential-key`). The constants are gone.
`cargo test -p podvm` exits 0 with `each_host_that_podvm_names_is_in_pod_services`
(`crates/podvm/src/hosts_tests.rs`): it reads the Rust sources of the
workspace (not `podvm-gate`, `podvm-release` and the tests, the tools of
the repository), takes the host of each URL and the registry of each quoted
image reference, and finds each in the crate, a name that RFC 2606 reserves
aside. `./target/x86_64-unknown-linux-musl/release/podvm-gate` exits 0.

Remaining: the crate does not list `gcr.io` (the profile
`distroless-static`) and `mirror.gcr.io` (the mirror of the workload), nor
the SHA-256 of the three NIC modules of the guest. COORD-044 (issue 263)
asks podcrates for them. Until its tag, the test names the two hosts in a
list of reported hosts, which fails the test when the crate lists one, so
the list only shrinks; then the list goes, the module sums come from the
crate, and this entry closes.

# T-1680: Take `pod-todo` for the record half of `podvm-gate`

**Source:** COORD-001, issue 194, row 8; PodUtils podcrates issue 3;
  `crates/podvm-gate/src/main.rs:1`; `crates/podvm-gate/src/grammar.rs:1`
**Category:** deps
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** partial

## Problem

`podvm-gate` checks the record (rows, fields, counts,
citations, links) and the tree (sizes, exports, plants) in one
4,220-line file. The record half is the same work that
podcrates' `pod-todo` does for its other consumers.

## Premise

COORD-001 takes `pod-todo` last, "after podvm's record moves
to the format of pod-todo". That move is T-1672. The grammar
is shared with `podvm-count` (`grammar.rs`).

## Approach

After T-1672, remove the record checks from `podvm-gate` and
call `pod-todo` through the `podvm-todo` wrapper. Keep the
tree checks in `podvm-gate`. Move each plant of a record check
to the wrapper and prove each still fails the gate. Report the
taken commit.

## Decision

No record check is lost: each one is in `pod-todo`, in the
wrapper, or recorded as retired with its reason.

## Prove

`./target/x86_64-unknown-linux-musl/release/podvm-gate` exits 0, and
`./target/x86_64-unknown-linux-musl/release/podvm-plant` exits 0 with every record plant
caught through the wrapper.

## Correction

**Blocked 2026-11-06.** Blocker: podcrates has not released `pod-todo`.
Clears when: that release exists, and T-1672 is done.
**Still blocked, 2026-11-10.** The first condition holds: `pod-todo` is in
podcrates `v0.12.0`. The blocker is now T-1672 alone, which is `open`.

**Unblocked 2026-11-11.** T-1672 is done: the record is in the format of
`pod-todo`, and `podvm-todo check` runs. It reports 356 problems that are
not of form: citations of paths that only the history keeps, ids of
other repositories with no `REPO@`, done entries that the work order
names, and line citations past the end of a file. This entry moves the
record checks of `podvm-gate` to pod-todo once they are 0.

## Done

**Partial 2026-11-11.** `podvm-todo check` reports 49 problems, from
356 after the move of T-1672. `references/` is no directory of a citation
for pod-todo (`TODO/todo.toml`): the captured trees are in the history,
and `podvm-gate` checks them against `references/MANIFEST.tsv`.
`scripts/todo/qualify-foreign-ids.py` wrote each id of a sibling's record
as `REPO@T-NNN` (88 ids: 85 of podcrates, 2 of podnet, 1 by hand), and
two recorded `grep -E` patterns name the same ids as `T-15(5|7)[0-9]`.
The work order of `TODO/PROGRESS.md` names no done entry.

Remaining: 35 line citations of a past tree (`:N` past the end of the
file today, or "line N" as plain text), each to be written as "line N at
`COMMIT`"; seven paths that are gone; the counts line and the All row of
`TODO/INDEX.md` in the form of pod-todo; the four-digit id in the title of T-1631,
an id that no row holds; two uses of "yet"; then the record checks of
`podvm-gate` move to the wrapper, with their plants.

# T-1681: Hand the seven generic crates to podcrates and freeze their files

**Source:** COORD-001, issue 194, "What podvm hands to podcrates";
  PodUtils podcrates issue 3, wave M6 (podcrates podcrates@T-055 to podcrates@T-061);
  PodUtils podcrates issue 1
**Category:** deps
**Milestone:** none
**Priority:** P2
**Effort:** L
**Status:** done

## Problem

The owner ruled on 2026-11-06 that each generic crate of podvm
moves to podcrates as `pod-*`. Until podcrates imports them and
podvm takes them back, two copies can drift: a fix in one does
not reach the other.

## Premise

podcrates wave M6 imports, in order: `pod-hostprobe` from
`crates/podvm-probe` (first, because extract, rootfs, enter and
supervise read its raw syscalls), `pod-oci` from part of
`crates/podvm-image`, `pod-extract` from `crates/podvm-extract`,
`pod-rootfs` from `crates/podvm-complete`, `pod-enter` from
`crates/podvm-enter`, `pod-supervise` from
`crates/podvm-supervise` and `pod-guest` from
`crates/podvm-windows`. Each import starts from the rename
commit that COORD-001 names. From the import on, the imported
files of podvm are frozen (fixes only) until a `taken` line in
podcrates' `SOURCES` ends the freeze.

## Approach

For each crate, as podcrates imports it: record the frozen
files in this repository, send each fix of a frozen file to
podcrates as a COORD issue with its commit, and, when the
crate is released, replace the podvm crate with a dependency
on it and report the taken commit. The content store stays in
podvm. The CA data that `pod-rootfs` reads moves to `pod-tls`
(T-1676).

## Decision

One entry tracks the hand-over of all seven, because each step
is the same procedure; a crate whose take needs code work gets
its own entry. A frozen file takes fixes only, as podcrates
issue 3 states.

## Prove

`./target/release/podvm-gate` exits 0 with each of the seven
podvm crates replaced by its `pod-*` dependency, and
`cargo test --workspace` exits 0 in the Linux lane.

## Correction

**Open 2026-11-06.** The blocker cleared for the first crate.
podcrates imported `pod-hostprobe` (podcrates@T-055) from podvm at `2c03085`
(PodUtils/podcrates#11). From that import the 29 files that the block
`repo PodUtils/podvm` of the SOURCES file of `pod-hostprobe` in
PodUtils/podcrates lists take fixes
only, each written as a COORD issue `(to podcrates)`:
`crates/podvm-probe/src/` except `exit.rs`, `json.rs`, `report.rs`,
`report/` and `nongoals.rs`; `crates/podvm-probe/tests/namespace*.rs`;
and `crates/podvm-image/src/space.rs`, `exec_probe.rs` and
`tests/space_precheck.rs`. Two fixes are podcrates': `sys::statfs` reads
the narrow struct on i686 and arm through a `statfs64`-shaped buffer
(podcrates podcrates@T-072), and `exec::try_exec` retries `ETXTBSY`, which is the
cause of the flake of `exec_probe::two_probes_in_one_process_take_two_names`
seen at `cec4bcb`. podvm takes them with the crate; the trial
`adoption/podvm/pod-hostprobe.patch` comes with the next podcrates
release, after the registry trial of T-1677. The other six crates wait
for their imports (podcrates podcrates@T-056 to podcrates@T-061).

---

**Still partial, 2026-11-09.** podvm took `pod-supervise` 0.8.0
(PodUtils/podcrates#41, its podcrates@T-060): `adoption/podvm/pod-supervise.patch`
of the tag applied with no conflict at podvm `284f429`. `podvm-supervise`
keeps podvm's glue (its settings, the record fields, the verbs with a
`&Store`, the launcher entry and the memo); `launcher/supervise.rs`
leaves podvm, and `podvm-image` re-exports the crate's `Lock`. Each line
of `pod-supervise.run` exits 0, and `cargo test --workspace` exits 0.
Remaining then: the imports of pod-guest, pod-enter, pod-oci,
pod-extract and pod-rootfs.

**Still partial, 2026-11-10.** podvm took `pod-guest` and `pod-enter`
0.9.0 (PodUtils/podcrates#43, its podcrates@T-061 and podcrates@T-059):
`adoption/podvm/pod-guest.patch`, then `pod-enter.patch`, applied with
`git apply --3way` at podvm `e37decf`. The code applied cleanly; three
record conflicts were joined by hand (two TODO citations that both sides
moved, and the added rows of `docs/runtime-state.md`). `podvm-enter` and
`podvm-windows` keep podvm's glue and call the crates. `run/prepare.rs`
reached 501 lines, so the detached check moved to
`lifecycle::refuse_detached_userland`. Each line of both `.run` files
exits 0, and `cargo test --workspace` exits 0. The freeze of
`podvm-enter` and `podvm-windows` ends. Remaining then: the imports of
pod-oci, pod-extract and pod-rootfs.

**Still partial, 2026-11-10.** podvm took `pod-oci` 0.10.0
(PodUtils/podcrates#45, its podcrates@T-056): `adoption/podvm/pod-oci.patch`
applied with `git apply --3way` at podvm `2971410`. The code applied
cleanly; the conflicts were the ones podcrates named: the tag lines (all
at v0.10.0), the `pod-*` rows of `docs/runtime-state.md` (each row kept,
at 0.10.0), and two citation lines of T-1684 in `TODO/gate.md` (both kept,
in the form that names the crates). Each line of `pod-oci.run` exits 0,
and `cargo test --workspace` exits 0. Remaining: pod-extract and
pod-rootfs (podcrates v0.11.0); `podvm-extract` stays frozen until then.

## Done

**Done 2026-11-10.** podvm took `pod-extract` and `pod-rootfs` 0.11.0
(PodUtils/podcrates#46, its podcrates@T-057, podcrates@T-058, podcrates@T-141, podcrates@T-139):
`adoption/podvm/pod-extract.patch`, then `pod-rootfs.patch`, applied with
`git apply --3way` at podvm `72523d7`, with no conflict. pod-extract
carries the T-1698 sentinel tests; pod-rootfs fixes eleven defects of the
old `podvm-complete`; the pod-enter of the tag stages a payload over
128 MiB (T-1647). Each line of both `.run` files exits 0, and `cargo test
--workspace` and the interposer tests exit 0. The seven generic crates of
this entry are with podcrates now; `podvm-extract` and `podvm-complete`
keep podvm's glue.

**Partial, 2026-11-06.** The first of the seven crates is taken:
podvm took `pod-hostprobe` 0.6.0 by `adoption/podvm/pod-hostprobe.patch`
of PodUtils/podcrates (`main` at `5535a27`, made at podvm `b32b19c`),
applied with `git apply --3way`; its two conflicts were the move to
v0.6.0 that podvm had made the same day, and its side was taken. 26
files leave podvm; `podvm-probe` keeps its report and re-exports the
crate, and `space.rs` and `exec_probe.rs` of podvm-image wrap it. The
freeze of the 29 files ends, and with it the two 32-bit faults of
`sys/` (podcrates podcrates@T-072 and podcrates@T-080). Each line of `pod-hostprobe.run`
exits 0 in a Linux container (about 866 tests, none failed;
`podvm probe --rows` gives 66 rows; the wire is `pod-hostprobe/1`),
`mux_two_client` 15 passes against sshd, and the test the `.run` skips,
`abi::tests::resolve_past_a_denied_listing`, passes as uid 0. Call C11
of `docs/decisions.md` accepts the four points of podcrates#17.
Gate run 42265769964 at `8899c2e` is green in each job; podcrates
recorded the take (its podcrates@T-055 done) and closed PodUtils/podcrates#11.
Remaining: the other six crates, each on its import and trial
(podcrates podcrates@T-056 to podcrates@T-061).

# T-1687: Fetch the Alpine netboot kernel and modloop from the 3.22.6 archive

**Source:** PodUtils podcrates issue 6 (COORD-004 of podcrates);
  `crates/podvm-measure/src/workload.rs:37-42`;
  `experiments/146-podvm-initramfs.sh:54-55`;
  `experiments/147-podvm-exec.sh:52-53`;
  `experiments/360-perf-harness.sh:49-50`;
  `experiments/361-guest-usernet.sh:45-48`;
  `experiments/390-machine-ssh.sh:50-51`
**Category:** deps
**Milestone:** none
**Priority:** P2
**Effort:** S
**Status:** done

## Problem

podvm pins `vmlinuz-virt` and `modloop-virt` of Alpine v3.22 by
SHA-256 and fetches them from `netboot/`. That directory follows
each point release and serves 3.22.8 now, so the sum check
fails on each fresh download.

## Premise

The pinned sums are the files of 3.22.6. Measured 2026-11-06:
`alpine-netboot-3.22.6-x86_64.tar.gz` of
`https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/`
is 301,555,932 bytes with SHA-256 `74ff3dd7ed96186b5fd4d728dd63ce5947578b40cf48ef792b8420e2463dc267`,
and its `boot/vmlinuz-virt` and `boot/modloop-virt` have the two
pinned sums.

## Approach

Fetch the archive, check its SHA-256, extract the two files
with `tar`, and check their sums as now. Keep one cached copy
of the archive in the scratch directory of the run. Change the
five experiments and `podvm-measure` the same way.

## Decision

Way 1 of podcrates issue 6: the pinned files stay, and a point
release does not move them. The values that moved to
`pod-services` are frozen lines; the change goes to podcrates
as a COORD issue `(to podcrates)`.

## Prove

`cargo test -p podvm-measure` exits 0, and the fetch step of
`podvm-workload` reports the two pinned sums from a fresh
download.

## Done

**Done 2026-11-10.** `PODVM_WORKLOAD_MIRROR=1` makes `podvm-workload`
take the same pinned alpine digest from `mirror.gcr.io`, because
`public.ecr.aws` answered its pull with HTTP 429; the conditions print the
reference. [The run](../experiments/results/workload-mirror-1687.txt):
the fetch step downloaded the 3.22.6 netboot archive afresh, extracted
`boot/vmlinuz-virt`, and reported "kernel hash matches the pin"
(`6b58e5d7...`); the helper of the experiments gave the modloop sum
(`9a5252fa...`) from a fresh download on 2026-11-09 (below). `cargo test
-p podvm-measure` and its clippy exit 0. The run exits 1 on a later
clause: the TCG guest printed no console byte in 600 s, which T-1702 owns.

**Partial, 2026-11-06.** `experiments/lib/alpine-netboot.sh` downloads
the 3.22.6 archive once, checks its SHA-256 and copies one member out;
experiments 146, 147, 360, 361 and 390 call it in place of `curl` of
`netboot/`, and each still checks the sum of the file. `podvm-measure`
fetches the archive and extracts `boot/vmlinuz-virt` with `tar`; its
default download timeout is 600 s. The helper gave the two pinned sums
(`6b58e5d7...` and `9a5252fa...`) from the archive downloaded on
2026-11-06. `cargo test -p podvm-measure` and its clippy exit 0, and
each changed script parses with `sh -n`.
COORD-006 (issue 200) writes the fix of the frozen lines to podcrates.

**Still partial, 2026-11-09.** A fresh download on the cloud host through
`experiments/lib/alpine-netboot.sh` gave the archive SHA-256
`74ff3dd7...c267` (301,555,932 bytes) and the two pinned sums,
`6b58e5d7...5d51` for `vmlinuz-virt` and `9a5252fa...c30f` for
`modloop-virt`. `podvm-workload` ran with qemu 8.2.2 present; clause 2
failed before the fetch step, because `public.ecr.aws` answered the pull
with `HTTP 429: TOOMANYREQUESTS: Data limit exceeded`. Remaining: one
`podvm-workload` run whose fetch step reports the pinned sum. podcrates
closed its issue 6 on the evidence of 5209adb.
Remaining: a run of `podvm-workload` whose fetch step reports the pinned
kernel sum.
