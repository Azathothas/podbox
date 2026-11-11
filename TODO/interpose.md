# interpose

Record semantics: [task rules](RULES.md#5-entry-closure).


`crates/podvm-interpose`. `TOOL.md` section 6.7, milestone M6.

[INDEX.md](INDEX.md) is the list and the counts. [PROGRESS.md](PROGRESS.md) is the work order.
[RULES.md](RULES.md) is how an entry closes. [reference-map.md](reference-map.md) is the corpus.

⛔ **`interpose` is a compatibility tier and never a security property**, and
the banner says so.

⭐ **Interposition is two jobs, and calling them one is the mistake the research
measures.** Path virtualization is proven to work on this runtime with
`mount(2)` denied. Ownership virtualization is a separate job that a path
interposer does not do, and it is the half that clears the wall stopping the
most tools. `pathmap` is the corpus's evidence for both halves at once, because
it has the first and demonstrably not the second.

---

### T-0701 The cdylib build constraints

Source:      `TOOL.md` section 6.7; `experiments/results/interposer-libc.txt`
Category:    interpose
Priority:    P0
Effort:      M
Status:      done

Problem:     This object runs inside other people's processes. A default Rust
             cdylib exports `rust_eh_personality` and friends into every process
             it is loaded into, allocates on paths it has intercepted, and can
             deadlock its host.
Premise:     ⭐ **Measured here.** experiments/60-interposer-libc.sh check A
             records that the crate cannot be built as a cdylib at all under the
             workspace's own `+crt-static`:

             ```
             error: cannot produce cdylib for `podvm-interpose` as the target
             `x86_64-unknown-linux-musl` does not support these crate types
             ```

             `scripts/build-interpose.sh:9-15` records why the fix is `RUSTFLAGS`
             and not a crate-local config: cargo merges config files up the
             directory tree and appends the parent's rustflags **after** the
             child's, so a local `-crt-static` is followed by the root's
             `+crt-static` and loses.
Approach:    Four constraints, none optional:

             1. a version script exporting **only** the interposed symbols;
             2. no allocation and no locks on an interposed path;
             3. no `println!`; write to fd 2 directly;
             4. it must survive the payload forking, which every workload here
                does constantly.

             ⚠ **Constraint 2 is too strong as stated and the corpus says so.**
             `references/pkgforge-dev__cross-libc-dlopen/tree/src/cross-libc-dlopen.c:585-598`
             holds a lock on an interposed path and writes down what makes it
             safe: it is a `__sync_lock_test_and_set` spin with `sched_yield`
             rather than a pthread mutex, and **nothing under it can re-enter
             it**. Without any lock, two threads both pass a bounds check and
             both post-increment an index, writing one entry past the end of a
             table. The rule podvm holds is **no re-entry under the lock**.
             `references/fritzw__ld-preload-open` paid for the other half of
             this twice: tracker PR #4 "fix memory corruption" and PR #3 "Cache
             the results of dlsym".
             ⚠ **And forwarding re-enters the payload's allocator, which is
             fine.** `fopen` returns a `FILE *` this object cannot construct and
             `opendir` a `DIR *`, so forwarding through `dlsym(RTLD_NEXT, ...)`
             is the only correct implementation, and it makes linking against a
             libc unavoidable: there is no raw-syscall object that dodges the
             question. What constraint 2 forbids is **this object's own**
             allocation on an interposed path, not the payload's.
Decision:    A `dlsym(RTLD_NEXT, ...)` result cached in a `static` per entry
             point, resolved on first use, as
             `references/VHSgunzo__pathmap/tree/path-mapping.c:603-606` does.
             The alternative, resolving on every call, is a `dlsym` inside every
             intercepted `open`.
Prove:       `./experiments/105-interpose-ownership.sh` exits 0, and its check A asserts the exported set against `interpose.map` and that `rust_eh_personality` is not in it


**Done 2026-09-09.** `crates/podvm-interpose/`, and all four constraints are
asserted rather than intended.

⭐ **`interpose.map` is the version script and check A compares the OBJECT with
it**, in both directions and for both libcs: 17 declared, 17 exported, and 0
`rust_eh_personality`. ⛔ Comparing the object with the script rather than
counting exports is what makes it a check: a name in the script that `src/lib.rs`
does not define is a silent non-interposition, and one the object exports that
the script does not list is a symbol some other library in the payload's process
resolves to podvm.

⛔ **`scripts/build-interpose.sh` produced TWO GLIBC OBJECTS and exited 0.** The
`x86_64-unknown-linux-musl` object recorded `libc.so.6`, `libgcc_s.so.1` and
`ld-linux-x86-64.so.2` in `DT_NEEDED`, because `rustc` passes `-lgcc_s` on that
target even under `panic = "abort"` and the host `cc` then linked it against the
host's glibc. The script passes `scripts/zig-cc.sh` as the linker now and
**asserts `DT_NEEDED` after every build**, on an exact word: `libc.so.6` contains
`libc.so`, so a substring test would call a glibc object musl-linked, which is
the very mistake being caught.

The four constraints, and how each is met:

1. **the version script**, above;
2. ⭐ **no allocation and no lock on an interposed path.** Every buffer is on the
   stack, the `dlsym` cache is one `AtomicPtr` per entry point resolved on first
   use, and the memo needs no lock at all because `O_APPEND` makes the kernel do
   the serialising. ⚠ The constraint as written is too strong and this entry
   already said so: what it forbids is THIS object's allocation, and forwarding
   re-enters the payload's allocator unavoidably;
3. **no `println!`**: `src/say.rs` writes to fd 2 with one `write(2)` out of a
   512-byte stack buffer, because stdout belongs to the payload;
4. **it survives a fork**: nothing is registered with `atfork`, nothing is held
   across one but a function address in an `AtomicPtr`, and the memo is a file
   rather than memory precisely so a `fork` and an `execve` both keep it.

⚠ **`dlsym` is the one import that decides which payloads this object can
serve**, and check F asserts what happens: built on glibc 2.39 it imports
`dlsym@GLIBC_2.34`, and against the pinned 2.31 payload
[T-0709](#t-0709-select-the-interposer-by-dt_needed-and-refuse-on-the-version-predicate)'s
reader refuses the pair from ELF with nothing loaded, and the loader then says
`version 'GLIBC_2.34' not found` when the pair is forced.

---

### T-0702 One object per libc, and it must live inside the rootfs

Source:      `experiments/results/interposer-abi.txt`; `references/fritzw__ld-preload-open` tracker
Category:    interpose
Priority:    P0
Effort:      M
Status:      done

Problem:     Two separate failures share this entry because the fix is the same
             artefact. A preload object built against one libc cannot serve a
             payload built against another, and an absolute `LD_PRELOAD` path
             from outside a chroot does not resolve inside it.
Premise:     ⚠ **The premise as first written was wrong, and the correction is
             below rather than in place of it.** The entry was written expecting
             experiments/60-interposer-libc.sh to show a musl-linked object
             failing to load into a glibc payload. It shows no such thing: on
             this host the musl-target object records `libc.so.6` in
             `DT_NEEDED`, because `--target x86_64-unknown-linux-musl` hands the
             link to the host's own C toolchain and there is no musl here. The
             script now reads `DT_NEEDED` and **exits 2** rather than answering
             from an object that is not musl-linked.
             What is measured, and settles the requirement from the other side:
             `references/pkgforge-dev__cross-libc-dlopen/tree/docs/limits.md:20`
             records that `regoff_t` is 4 bytes on glibc and 8 on musl, and that
             the `FTW_*` constants are off by one, and that "the offset is
             compiled into the object, so no preload can reach it". ⭐ Symbol
             **versioning** across a libc boundary is bridgeable, by the memfd
             rewrite at
             `references/pkgforge-dev__cross-libc-dlopen/tree/src/cross-libc-dlopen.c:8-12`.
             Struct **layout** is not. An interposer that hands a `struct stat`
             back to the payload must use the payload's layout.
             Upstream agrees at the build level:
             `references/VHSgunzo__pathmap/tree/Makefile:13-19` carries separate
             `path-mapping-glibc.so` and `path-mapping-musl.so` targets.
             ⭐ **And the cross-libc load is now measured, in both directions,
             with both controls.** `experiments/results/interposer-abi.txt`
             check B, taken with `musl-gcc` installed and the C reference
             interposer as the subject:

             ```
             control glibc object -> glibc        rc=0
             glibc object -> musl payload         rc=127  Error relocating /i.so: __snprintf_chk: symbol not found
             control musl object -> musl          rc=0
             QUESTION B musl object -> glibc      rc=127  /lib/x86_64-linux-gnu/libc.so: invalid ELF header
             ```

             ⭐ **And on 2026-09-08 it was measured a second time, with podvm's
             OWN Rust cdylib rather than the C reference interposer, which is
             what the correction above said could not be done here.**
             experiments/60-interposer-libc.sh exits **0** for the first time,
             and `experiments/results/interposer-libc.txt` carries:

             ```
             linker for musl: scripts/zig-cc.sh (zig 0.16.0)
             x86_64-unknown-linux-musl      OK   286056 bytes  1 NEEDED entries
             musl-target DT_NEEDED   libc.so
             gnu-target  DT_NEEDED   libgcc_s.so.1 libc.so.6 ld-linux-x86-64.so.2
             musl object into a glibc payload: rc=127
               /usr/bin/env: error while loading shared libraries:
               /lib/x86_64-linux-gnu/libc.so: invalid ELF header
             ```

             What changed is the linker, not the conclusion. `rustc` passes
             `-lgcc_s` on the musl target even under `panic = "abort"` and
             `musl-tools` ships no musl-linked `libgcc_s.so.1`, so the default
             `cc` linked the "musl" object against the host's glibc and the arm
             could not run. `zig cc` carries its own `compiler-rt`.
             ⭐ **Two independent objects, one C and one Rust, now give the same
             answer through the same mechanism**, and the requirement no longer
             rests on a single reference implementation.

             ⭐ The mechanism is **not** symbol versioning and not struct
             layout: it is the **SONAME**. musl's libc declares none, so an
             object linked against it records `libc.so` in `DT_NEEDED`, and on a
             glibc host `/lib/x86_64-linux-gnu/libc.so` is a GNU ld script.
             The loader rejects it at the ELF header, before a symbol is read.
             That makes the discriminator one `readelf` away, with nothing run,
             which is what T-0709 turns into a selection rather than an attempt.
Approach:    Build one object per libc, embed both, and select on the payload's
             `PT_INTERP` (T-0706). Then place the selected object **inside the
             rootfs** before the chroot and set `LD_PRELOAD` to its path as the
             payload will see it.
             ⭐ The absolute-path half is the upstream maintainer's ruling,
             recorded in `references/fritzw__ld-preload-open/api/comments.json`
             on issue #5, 2024-12-04: a relative `LD_PRELOAD` "would break as
             soon as your preloaded process changes to a different directory and
             spawns a new process". `TOOL.md` section 6.7 says the object is "extracted
             to the store on first use", and the store is outside the chroot, so
             that placement does not survive step 3 of section 6.5.
             Also from that tracker, issue #6 and its 2025-02-17 comment: the C
             source needs `struct stat64` to `struct stat`, `DISABLE_FTW`, and a
             hand-defined `__OPEN_NEEDS_MODE` to build against musl at all.
             podvm declares the syscalls rather than the libc struct variants
             and avoids that whole class.
Decision:    Two objects, not one with runtime detection. Runtime detection
             cannot change a struct offset the compiler already emitted.
Status note: **no longer blocked, and the musl gap is closed.** The measurement
             this entry waited on is taken by `experiments/80-interposer-abi.sh`.
             ⭐ And podvm's own Rust cdylib IS musl-linked now:
             `scripts/build-interpose.sh` passes `scripts/zig-cc.sh` as the
             linker for that target and asserts the resulting `DT_NEEDED`, so
             the object records `libc.so` where it recorded `libc.so.6` before.
             ⛔ Without the assertion the script exited 0 having produced TWO
             GLIBC OBJECTS, which is the whole requirement inverted and reads as
             success.
             ⭐ **THE EMBEDDING IS RULED AND HALF BUILT, on 2026-09-12.** This
             `Approach` says "embed both", and the two objects are built by a
             script rather than by cargo, so `include_bytes!` in `podvm`
             would make an ordinary `cargo build` fail on a tree where the
             script has not run, which breaks `cargo test --workspace`, the
             acceptance and every contributor's first command.
             ⚠ **A FOURTH SHAPE WAS TAKEN, AND IT IS NOT ONE OF THE THREE THIS
             ENTRY LISTED.** `crates/podvm/build.rs` COPIES what
             `scripts/build-interpose.sh` left behind, and writes an EMPTY file
             where an object is absent. `include_bytes!` always compiles, a
             fresh clone with no zig still builds, and an empty object is read
             at run time as "carries none" so the payload gets
             [T-0706](#t-0706-classify-the-payload-and-decline-with-a-named-reason)'s
             named decline rather than a preload that cannot load.
             ⛔ **The rejected three, and why.** A `build.rs` that RUNS the
             script is a cargo inside a cargo, which waits on a lock its own
             parent holds. ⚠ That hazard was MEASURED rather than assumed:
             the embedding probe script (deleted 2026-10-01) ran a nested
             build in two shapes on 2026-09-12 and **both completed**, so it
             does not fire
             here. It is still refused, because it would fire on somebody
             else's machine and copying cannot. A `build.rs` that refuses is
             loud and still breaks the plain build. The objects committed as
             artefacts are build output in the tree, which
             `docs/conventions/git.md` section 4 forbids.
             ⛔ **AND THE ORDER HAD TO MOVE WITH IT.** dev.sh check (now `podvm-dev check`)
             and the gate workflow both built the binary BEFORE the objects, so
             with this shape they would have embedded two placeholders and
             passed. The interposer step now runs first in both.
             ⚠ **The PLACEMENT was still open until 2026-09-18**, which is this
             `Approach`'s second sentence: the selected object written INSIDE
             the rootfs before the chroot, and `LD_PRELOAD` set to the path the
             payload will see. It landed in `crates/podvm/src/interpose.rs`
             (`place`, called from `apply`); the Done note below carries the
             run.
Prove:       `./experiments/80-interposer-abi.sh` exits 0, and `podvm run --rm public.ecr.aws/docker/library/alpine:3.20 sh -c 'test -f /.podvm/interpose.so && env | grep -q "^LD_PRELOAD=/\.podvm/interpose\.so"'`. ⛔ **The reference was an unqualified tag until 2026-09-12**, which resolves through the engine's shortname aliases to a quota-bearing registry, so this acceptance could not be run under the rule `scripts/common/distro-matrix.sh` states. It names the M5 row for that distribution now. [T-1209](gate.md) owns the other 42 lines with the same defect. ⛔ **The second half read `/proc/self/environ` until 2026-09-18**, and the chroot has no `/proc`, so that line could not pass under podvm: `grep` answered `No such file or directory` while the object sat placed beside it. The environ is read through the payload's own `env` instead.


**Done 2026-09-18.** The placement half landed in `crates/podvm/src/interpose.rs` (`place`, and `apply` calling it): the selected object is written to `/.podvm/interpose.so` inside the rootfs before the chroot, sibling plus rename, and `LD_PRELOAD` names it, podvm's object first where the caller set one. `run`, `exec` and `create` share the one call. `experiments/159-interpose-placement.sh` clause A is this `Prove`, pinned to the M5 digest, and `experiments/results/interpose-placement.txt` carries the run: file present, `LD_PRELOAD` in the payload's `env`, exit 0.

⭐ **The resolver follows absolute links under the rootfs, because the first
shape declined every dynamic payload.** Alpine's `/bin/sh` points at the
absolute `/bin/busybox`, which names the host's file when read from outside
and the image's from inside. The naive read found nothing, declined `sh`,
and the payload still ran, so the tier read as absent on every image whose
shell is an absolute link. `resolve_in` walks the guest path component by
component and splices an absolute target back under the root, the way the
guest kernel would; `..` past the root and a link loop are refused rather
than followed.

**Done 2026-10-01 (the libc-interpose port).** The `60-interposer-libc.sh`
measurement is the `podvm-libc-interpose` binary in the gate crate
(`crates/podvm-gate/src/libc_interpose.rs`), staying out of the workspace
members: no new crate, no member row. No `Prove` line names the retired
script and no workflow step runs it, so nothing repoints; the red-run
plant is `experiments/results/libc-interpose-port-plant.txt`.

---

### T-0703 Path virtualization: the entry-point set and `*at` resolution

Source:      `TOOL.md` section 6.7; `references/VHSgunzo__pathmap`
Category:    interpose
Priority:    P0
Effort:      L
Status:      done

Problem:     A payload asks for a path that is not there. Without a rewrite in
             front of every path-taking entry point, `-v` is a copy and nothing
             else.
Premise:     ⭐ **Measured here, and it corrects a number in the
             specification.** `TOOL.md` section 7 records pathmap as "129 interposed
             entry points". Building
             `references/VHSgunzo__pathmap/tree/path-mapping.c` with its own
             Makefile and reading `nm -D --defined-only` gives **129 exported
             defined symbols**, of which **113** are libc entry points and 16
             are internals with a `_` or `pm_` prefix. 129 is the symbol count;
             113 is the entry-point count.
             ⭐ **And the family that is easiest to under-cover is measured.**
             `experiments/results/interpose-symbols.txt` checks A and B: an
             interposer defining `execve` alone catches **1 of 7** exec entry
             points, and the same seven with every entry point defined catch
             **7 of 7**. `execvp`, `execl`, `execlp`, `posix_spawn`,
             `posix_spawnp` and `execv` each resolve the path themselves and
             reach glibc's internal exec, which does not go through the PLT.
             Arm B is what makes arm A's zeros the entry points rather than a
             broken harness.
             ⚠ On this host `execvp` is imported by 14 shared objects,
             `posix_spawn` by 12 and `posix_spawnp` by 7, among them
             `libglib-2.0.so.0`, four `libpython3.*`, `libdbus-1`, `libarchive`,
             `libmagic` and `libsystemd`. A count on another host will differ;
             the mechanism will not.
             ⭐ **Keep both the `64` and the plain names, and the `__xstat`
             shape too.** Measured: all four `libpython3.*` and
             `libglib-2.0.so.0` import only `stat64`, `lstat64` and
             `fstatat64@GLIBC_2.33`, while `libarchive` and `libdbus-1` import
             the plain `stat`, `lstat` and `fstatat`. Both sets are live in one
             process, so neither can be dropped as a duplicate. `__xstat`,
             `__lxstat` and `__fxstatat` are the pre-2.33 shape and glibc 2.39
             still exports them at `GLIBC_2.2.5` and `GLIBC_2.4`, so they are
             not dead code either: the **payload's** libc decides which shape
             its libraries call, not the host the interposer was built on.
             ⚠ The adopted mechanism already defines all of them. Measured
             against the built object: every exec entry point above is present,
             and the path-taking names it does not define are `fstat`, `fstat64`
             and `fexecve`, which take a descriptor, plus `system` and `popen`,
             which take a shell command and reach the interposer through the
             child shell.
Approach:    ⭐ **Do not maintain a list. Produce the gap with a command.**
             `experiments/100-interpose-symbols.sh` check D reads a pinned
             rootfs from outside, enumerates the path-taking libc names its own
             binaries import, and subtracts what the interposer defines. On a
             pinned debian rootfs on 2026-09-08 that is **38 names reached, 3
             not defined**: `eaccess`, `mount` and `umount`. A list somebody
             maintains rots; a number a command produces does not.
             ⚠ Read the rootfs from **outside**. A reader run inside the image
             measures that image's package list rather than its binaries, and
             most images ship no `readelf` at all. It is also the wrong
             position: podvm extracts a rootfs and inspects it from outside.
             Cover the families, not a list of names: `open*`, `stat*`, `exec*`,
             `chdir`, `readdir`, `getcwd`, `readlink`, `realpath`, the xattr
             family, `link*`, `rename*`, `mkdir*`, `mk*temp`, `glob`, `scandir`,
             `ftw`, and the `*at` variants of each.
             ⭐ The `*at` mechanism to copy is
             `references/VHSgunzo__pathmap/tree/path-mapping.c:161-187`
             `absolute_from_dirfd()`: a relative path is made absolute by
             `readlink`ing `/proc/self/fd/<dirfd>`, and `/proc/self/cwd` for
             `AT_FDCWD`, before any matching happens.
             ⚠ **And one defect in it that podvm must not copy.** On a
             `readlink` failure that function does `return path`, handing the
             **unmapped relative path** to the real call with no diagnostic. A
             path that could not be resolved then reaches the real tree
             silently. podvm fails the call with `ENOENT` and writes one line
             to fd 2, because a silent escape from a virtual view is the
             `sandlock` failure mode in another costume.
Decision:    A macro over the families, as
             `references/VHSgunzo__pathmap/tree/path-mapping.c:250-262` does, so
             a new entry point is one line rather than a copied body. Hand-write
             only the ones whose argument shape the macro cannot express, which
             in pathmap is `fchownat` at
             `references/VHSgunzo__pathmap/tree/path-mapping.c:1242-1256`.
Prove:       `./experiments/161-path-rewrite.sh` exits 0 and `nm -D --defined-only crates/podvm-interpose/target/x86_64-unknown-linux-gnu/release/libpodvm_interpose.so | grep -c ' T ' | awk '$1 >= 60'`. ⛔ **The first half named `50-interpose-tier.sh` until 2026-09-18**, which measures pathmap, somebody else's object, and cannot verify this one: the wrong subject with a green exit, the same defect class [T-0709](#t-0709-select-the-interposer-by-dt_needed-and-refuse-on-the-version-predicate) amended its own `Prove` for. `161` drives podvm's object through podvm on both libcs, and `experiments/results/interpose-paths.txt` carries the run.


**Done 2026-09-18.** The object interposes 89 names (`src/map.rs` plus ten
macros and twelve hand-written shapes in `src/lib.rs`): the `open`, `stat`
(already there), `exec`, `chdir`, `opendir`/`scandir`, `readlink`/`realpath`,
`xattr`, `link`, `rename`, `mkdir`, `mkstemp`/`mkostemp`/`mkdtemp`, `access`,
`chmod`, `truncate`, `utime`, `statfs`/`statvfs`, `remove`, `fopen`, `glob`,
`ftw`, `tmpfile` families and every `*at` variant of each, `execveat`
included. Functions taking only descriptors stay out, which is
`experiments/100-interpose-symbols.sh`'s own rule: there is nothing in them
to rewrite. The table
is one variable, `PODVM_MAPS`, in `FROM:TO[,FROM:TO...]` pairs mirroring
pathmap's syntax; longest match wins at a component boundary, and
`/.podvm/` never rewrites. With no table every call forwards unchanged, so
an unmapped payload cannot fail here at all.

⭐ **Three deviations from the letter, each with its reason.** `AT_FDCWD`
resolves through the real `getcwd`, which answers identically without
`/proc`. A descriptor-relative path the table cannot be checked against
forwards unchanged: the kernel resolves it through a directory that is
already real, so forwarding is kernel-correct and failing it would break
`tar`, `find` and `ls -R` on any mapped run, which `161` clauses G/G2 hold
green. `execl`, `execlp`, `execle` and `execlpe` are absent: they are
C-variadic and stable Rust cannot define one (rust-lang/rust#44930), which
the gap clause owns beside `mount`, `umount`, `umount2`, `dlopen` and
`dlmopen` (T-0708's and the loader's). Pure forwarders with no path to
rewrite (`getcwd`, `readdir` and their kin) stay out: risk without function,
and T-0705 interposes them with the reverse mapping when it lands. `tmpfile`
is the exception: the completeness command counts it as reached, so it
forwards.

⭐ **The memo proof runs where the kernel cannot be asked nicely.** `161`
clause H drops privilege in a child, so `chown` fails `EPERM` on any host
however rootful, and the memo answers `0:42` with the file grown by one
record, on both libcs. The `chown`/`stat` families resolve through the same
funnel, so a mapped `chown` memos the real file (clause J2).

---

### T-0704 Ownership virtualization: the half a path interposer does not have

Source:      `TOOL.md` section 6.7, `paper_final.md` section 9.3; `references/salsa-debian__fakeroot`
Category:    interpose
Priority:    P0
Effort:      L
Status:      done 2026-09-22

Problem:     `chown 0:42` fails identically with and without a path interposer
             loaded. This is the wall that stops the most tools, and clearing it
             is what makes M6 worth doing.
Premise:     ⭐ **Measured, and the reason is visible at file and line.**
             `references/VHSgunzo__pathmap/tree/path-mapping.c:1240-1241` routes
             `chown` and `lchown` through the same `OVERRIDE_FUNCTION` macro as
             every other entry point: the **path** is rewritten and the uid and
             gid are passed through untouched. So the interposer sees the call
             and changes nothing about the ids, and the kernel answers `EINVAL`
             exactly as it would have without it.
             The model for the other half is fakeroot's
             `references/salsa-debian__fakeroot/tree/libfakeroot.c:875-907`:
             read the real metadata, overwrite `st_uid` and `st_gid` with the
             **intended** values, memo them, then attempt the real call, and
             swallow the failure.
Approach:    Intercept the `chown` family and return success after recording the
             intended owner to the sidecar T-0302 writes. Report the memo back
             from the `stat` family, so a workload that asks who owns a file
             gets the image's answer. Do the same for `setuid`, `setgid` and
             `setgroups`: success plus an identity memo.
             ⛔ **fakeroot's errno test is wrong for this runtime and copying it
             would reproduce the bug.**
             `references/salsa-debian__fakeroot/tree/libfakeroot.c:903` is
             `if(r&&(errno==EPERM)) r=0;`, and there are eight such sites at
             `:903`, `:930`, `:960`, `:995`, `:1033`, `:1066`, `:1100` and
             `:1134`. **Here the errno is `EINVAL`**, so every one of them would
             return the error to the caller. podvm swallows `EPERM` **and**
             `EINVAL`, and records which it swallowed.
             The switch that decides whether to attempt the real call at all is
             `references/salsa-debian__fakeroot/tree/libfakeroot.c:669-678`
             `dont_try_chown()`, a cached read of an environment variable.
             podvm probes once instead of asking the user, and caches the
             verdict the same way.
Decision:    Probe once and cache, rather than fakeroot's environment variable.
             A user who has to know to set a variable has to know the wall
             exists, and the audience is automated.
Prove:       `./experiments/105-interpose-ownership.sh` exits 0, and then `podvm run --rm public.ecr.aws/docker/library/alpine:3.20 sh -c 'chown 0:42 /tmp/f && stat -c %u:%g /tmp/f' | grep -qx '0:42'` once T-0702 places the object

**Done 2026-09-22.** Both items the entry left open landed under their own
entries with driven evidence. The wiring is [T-0702](#t-0702-one-object-per-libc-and-it-must-live-inside-the-rootfs),
done 2026-09-18: `place` writes the object to `/.podvm/interpose.so`
before the chroot and `apply` sets `LD_PRELOAD` to it, driven by
`experiments/159-interpose-placement.sh` clause A with the run in
`experiments/results/interpose-placement.txt`. The identity half is
[T-0711](#t-0711-the-identity-calls-and-podvms-honesty-rules-point-the-other-way-from-fakeroots),
done 2026-09-19: the operator's answer 3 rules the fork this entry left
open, so the `Approach` sentence asking for success-plus-memo on
`setuid`, `setgid` and `setgroups` is superseded by honest refusal by
default with `--user` turning on the fakeroot behaviour, driven by
`experiments/106-interpose-identity.sh` (14 checks, exit 0). The object
half is re-driven since: `experiments/105-interpose-ownership.sh`
check G runs it end to end through the host memo with
`experiments/results/interpose-ownership.txt` carrying 7 checks, exit 0,
and a guest `podvm run` smoke test answered `0:42`.

**Partial, 2026-09-09.** The object does it and it is measured under both libcs;
what is not in is the wiring that puts the object inside a rootfs `podvm run`
enters, which is
[T-0702](#t-0702-one-object-per-libc-and-it-must-live-inside-the-rootfs), and the
identity half of the `Approach`.

⛔ **THE SIDECAR CANNOT BE THE MEMO, and this `Approach` said it would be.**
`src/sidecar.rs` of `pod-extract` (PodUtils/podcrates) writes JSONL **beside the rootfs in the
store**, and this object runs after the `chroot`, where the store does not exist.
It is also JSON, and parsing it would allocate on an interposed path, which
[T-0701](#t-0701-the-cdylib-build-constraints) constraint 2 forbids. The memo is
a fixed-width binary file at `/.podvm/ownership.memo`, **inside** the rootfs,
keyed by `(dev, ino)` rather than by path: a path is renamed, hard-linked and
resolved through symlinks, and the question is about the inode.

⛔ **AND IT MUST CROSS PROCESSES, which rules out a table in memory.** This
entry's own `Prove` is `sh -c 'chown 0:42 /tmp/f && stat -c %u:%g /tmp/f'`, and
those are two `execve`s: a memo in this object's memory would be gone before the
question was asked. fakeroot solves that with a DAEMON, `faked`, which podvm has
no room for inside somebody else's chroot. `O_APPEND` and one 32-byte write per
record is what replaces the daemon and the lock both.

⛔ **`statx` IS NOT A DUPLICATE OF `stat`, and leaving it out made the glibc arm
report `0:0` while the memo was written correctly.** Measured on 2026-09-09:
coreutils' `stat` on a glibc 2.41 payload asks `statx(2)` and never reaches
`stat`, `stat64` or `__xstat`; busybox's `stat` on musl does call `stat`. ⚠ So
the musl arm passed and the glibc one did not, and **a one-libc test would have
shipped it**. That is the same shape as T-0703's finding about the `64` names
and one libc's importers.

⚠ The errno correction this entry predicted is real and is implemented: the
object swallows `EPERM` **and** `EINVAL`, where fakeroot tests `EPERM` alone at
eight sites. `--cap-drop=CHOWN` gives `EPERM` and is what checks D and E use;
the runtime podvm targets gives `EINVAL` for an unmapped id.

⛔ **The control comes first and it is the one that could have made every other
arm pass for the wrong reason.** On a machine that CAN chown, the object must
change nothing and write no memo: check C asserts `stat` reports `0:42` because
the KERNEL did it and `/.podvm/ownership.memo` does not exist. An interposer
that reported the caller's intent where the real call would have worked is weaker
than the bare chroot it replaces.

⚠ **What is not in**, and it is named rather than left to be discovered:

- **`setuid`, `setgid` and `setgroups`**, which this `Approach` asks for in one
  sentence. They need a per-PROCESS identity memo that survives an `execve`, and
  an inode-keyed file cannot carry one. fakeroot's answer is an environment
  variable, which this entry's `Decision` rejected for the file question and
  which may be right for this one; it is a fork nobody has ruled on;
- **the wiring**: nothing sets `LD_PRELOAD` yet, so `podvm run` does not load
  this object at all. T-0702 is that entry, and until it lands the `Prove` above
  runs through `docker run -v` rather than through podvm.

Prove, run 2026-09-09:

```
$ ./experiments/105-interpose-ownership.sh
  A: both objects export exactly what interpose.map declares, 17 each,
     and rust_eh_personality is in neither
  B: sizeof=144 dev=0 ino=8 mode=24 uid=28 gid=32, under both libcs
  C: the real chown worked and podvm wrote no memo
  D: the bare chown failed (rc=1) and podvm's answered 0:42     glibc
  E: the musl object answered 0:42 where the bare chown failed   musl
  F: the reader refused the too-old pair from ELF and the loader agreed
```

---

### T-0705 Reverse mapping, so the payload reads back what it wrote

Source:      `TOOL.md` section 6.7; `references/VHSgunzo__pathmap`
Category:    interpose
Priority:    P1
Effort:      M
Status:      done

Problem:     A payload that opens `/mapped/x`, then calls `getcwd` or
             `realpath`, gets the real path back and notices the virtualization.
             `configure` scripts and build systems compare the two and fail in
             ways that name neither.
Premise:     Read at file and line.
             `references/VHSgunzo__pathmap/tree/path-mapping.c:145-152`
             `reverse_fix_path()` maps a real path back to its virtual name, and
             only for a path lying under one of the destination prefixes.
             `references/VHSgunzo__pathmap/tree/path-mapping.c:730-758` is
             `realpath` with the reverse map applied to the result.
             ⚠ A limitation stated in that code, at
             `references/VHSgunzo__pathmap/tree/path-mapping.c:752`: when the
             caller supplied the output buffer, "we cannot safely apply reverse
             mapping", so `realpath(p, buf)` is **not** virtualized while
             `realpath(p, NULL)` is.
Approach:    Reverse-map `getcwd`, `get_current_dir_name`, `readlink`,
             `realpath` and `canonicalize_file_name`. `readdir`'s `d_name`
             takes no reversal: a bare name carries no prefix, so there is
             nothing to match it against. For the caller-supplied-buffer case
             podvm writes the virtual name when it fits and returns `ERANGE`
             when it does not,
             rather than silently returning the real one: `ERANGE` is a defined
             answer the caller handles and a real path is a leak.
Decision:    `ERANGE` over a silent real path. The buffer case is rare and a
             leaked host path in a `configure` script's output is a build that
             bakes in the wrong prefix.
Prove:       `podvm run --rm -e PODVM_MAPS=/mapped:$PWD public.ecr.aws/docker/library/alpine:3.20 sh -c 'cd /mapped && test "$(pwd)" = /mapped'`
             (The `-v` spelling the entry was authored with does not exist;
             `-e` carries the same table and the assertion is unchanged.)

**Done 2026-09-22.** `map::unrewrite` mirrors `rewrite` with the sides
swapped (longest TO wins, the `/.podvm/` guard holds, `-2` past the
buffer), with unit tests for the match, the boundary, the longest
win, the guard, the trailing-slash and root TO sides. Six entry
points read results back through it: `getcwd` (both the allocating
extension and the sized buffer), `get_current_dir_name`,
`realpath` (both shapes), `canonicalize_file_name`, `readlink`
and `readlinkat`, each keeping the forward path's contract beside
the reversal. With an empty table every one forwards exactly as
before, so unmapped runs keep byte-identical behaviour. `getcwd`
and `get_current_dir_name` join the export map (the musl object
hid them without it; the gnu object already exported them).
Driven green on host podman with the shipped binary
(maps `/mapped:/etc`): the entry's `pwd` test, the forward read,
`realpath`, the `readlink` error passthrough, and on the rocky
payload `getcwd`, `realpath`, `get_current_dir_name` and the ruled
`ERANGE` (a 6-byte buffer answers NULL with `ERANGE` where the
real 4-byte path would fit, a 64-byte buffer reads `/mapped`).
Close-out probe on the shipped binary: archlinux and voidlinux-musl
both answer RC=0 with the preloaded line and no decline line in full
stderr, musl `getcwd` holds end to end (`pwd -P` under `/mapped:/etc`
answers `/mapped`), and `nm -D` on both lane-built objects exports
`getcwd` and `get_current_dir_name`.
Out of scope: the `-v` volume flag (a separate unit; `-e` proves
the mechanism), and second-guessing the `ERANGE` Decision here.

---

### T-0706 Classify the payload and decline with a named reason

Source:      `TOOL.md` section 6.7, `paper_final.md` section 10.3
Category:    interpose
Priority:    P0
Effort:      S
Status:      done

Problem:     Starting a mode that cannot reach the payload fails later, deeper
             and less legibly than declining it.
Premise:     Read, and one row of it is measured in the research this project
             rests on: a libc interposer never sees a Go program's `lchown`,
             with or without cgo, because Go's `os` package issues it as a raw
             syscall.

             | ELF property | verdict |
             | --- | --- |
             | `PT_INTERP` present | dynamically linked; interposition may reach it |
             | `PT_INTERP` absent | static; interposition cannot reach it |
             | Go build markers present | unreachable regardless of linkage |
             | anything else | may still issue raw syscalls; no ELF property proves coverage |

             ⭐ The precise reason for row 2 is in the corpus:
             `references/pkgforge-dev__cross-libc-dlopen/tree/docs/limits.md:45`
             observes that a fully static binary "has no `LD_PRELOAD`
             mechanism, because there is no dynamic loader to honour it". It is
             not that the preload fails; nothing reads the variable.
             That page also splits the static case three ways and labels all
             three unverified, which is the honest state and worth copying.
             `references/apptainer__apptainer` is the fourth tool at the
             ownership wall whose unpacker is Go, so interposition cannot rescue
             it either.
Approach:    Read the ELF once, at `run` and at `exec`, and treat the answer as
             **advisory**. Where it says unreachable, decline the mode by name.
             ⭐ The reasoning to copy for why this happens before exec rather
             than after is
             `references/qaidvoid__onelf/tree/crates/onelf-rt/src/main.rs:133-148`:
             "exec of a valid ELF succeeds, and the loader's failure happens
             afterwards, in a process that is no longer ours." That is the last
             point at which anything can be said about it.
Decision:    Decline the tier, do not fall back to a copy silently. A `-v` that
             became a copy is a different program's semantics.
             The `ptrace` tier is not a fallback either: it is a separate rung
             that needs a usable `ptrace`, which this runtime denies, and
             `references/proot-me__proot/tree/src/cli/cli.c:135-138` shows what
             a tool that assumes otherwise tells the user.
Prove:       `./experiments/159-interpose-placement.sh` exits 0. Clause B is
             row 2's acceptance: it stages podvm's own release binary
             (musl static, no `PT_INTERP` per
             [T-0701](#t-0701-the-cdylib-build-constraints)) with `extract`
             plus a host copy plus `run`, and asserts the
             `interpose: declined` line and exit 0. The old `run -v`
             one-liner never ran and is gone: `run -v` is a refused `None`
             row, and check 26 of `crates/podvm-gate/src/main.rs` now refuses any
             done Prove that names one (TODO/gate.md T-1325). ⚠ **That drives row 2 of the table and not row 3**, and the substitution is stated rather than quiet: podvm's own release binary is `x86_64-unknown-linux-musl` under the workspace's `+crt-static`, which [T-0701](#t-0701-the-cdylib-build-constraints) records, so it carries no `PT_INTERP` and is a static payload this tree already builds. ⭐ The command IS the check on that: a payload with a `PT_INTERP` would not be declined and the line would fail. ⛔ **The line named an unqualified Go image until 2026-09-12**, which resolves through the engine's shortname aliases to a quota-bearing registry. ⚠ **Row 3, the Go payload, has no acceptance now**, because every row of `DISTRO_ROWS_M5` is a base distribution and none is a Go image, and inventing a reference is what [T-1209](gate.md) refuses. That entry's `Approach` step 2 is where the row is asked for. ⛔ **The `-v` in that line never ran: `run -v` is a refused `None` row.** `experiments/159-interpose-placement.sh` clause B stages the same static binary with `extract` plus a host copy plus `run` instead, and asserts the decline line and exit 0.


**Done 2026-09-18.** The classifier landed in `crates/podvm/src/interpose.rs` (`classify`, read once per payload, advisory, safe direction on every doubt) and runs at `run`, at `exec` on both paths, and at `create` through the shared `prepare`. Where it says unreachable the tier is declined by name on stderr and the payload still runs. `experiments/results/interpose-placement.txt` carries the run: row 2 declines naming the static payload with exit 0, the dynamic control is preloaded and not declined. ⭐ **Row 3 is proved without an image.** The Go markers outrank a present interpreter by unit test over all four section names, because [T-1209](gate.md) refuses an invented reference and no `DISTRO_ROWS_M5` row is a Go image. That answers [PROGRESS.md](PROGRESS.md)'s open question with the second option: the classification's Go case is proved some other way.

---

### T-0707 The paths that must not be rewritten

Source:      `references/dex4er__fakechroot`; `TOOL.md` section 6.7
Category:    interpose
Priority:    P1
Effort:      S
Status:      done

Problem:     Rewriting every path breaks the interposer's own machinery. The
             `*at` resolution reads `/proc/self/fd/<n>`, the classification
             reads the payload's ELF, and the sidecar lives in the store. If
             those go through the map, the interposer virtualizes itself.
Premise:     Read at file and line.
             `references/dex4er__fakechroot/tree/src/libfakechroot.c:116-128`
             parses `FAKECHROOT_EXCLUDE_PATH` into a prefix list, and
             `references/dex4er__fakechroot/tree/src/libfakechroot.c:174-186`
             tests every path against it and returns early on a match, taking
             care that a prefix matches only at a component boundary.
             ⛔ fakechroot is LGPL-2.1 and is read only. The mechanism is
             described here; no line of it is copied.
             `references/dex4er__fakechroot/tree/src/` holds 153 `.c` files, one
             per entry point, which is the other half of what the tree
             demonstrates: coverage is per entry point and there is no shortcut.
Approach:    A built-in exclusion list, not only a user-supplied one:
             `/proc` (one tree prefix covers `/proc/self/fd`,
             `/proc/self/cwd` and `/proc/self/exe`, the leaves the machinery
             reads, and whatever it names next), and the interposer's own
             object and memo under `/.podvm/`. The store takes no entry:
             payload paths never address the host store. Match at a component
             boundary so `/proctor` is not excluded by `/proc`. Then accept a
             user list on top of it, in `PODVM_EXCLUDE_PATH`.
Decision:    Built-in first, user list second, and the built-in entries are not
             removable. A user who removes `/proc/self/fd` from the list gets an
             interposer that cannot resolve `*at` calls, and the failure names
             nothing.
Prove:       `podvm run --rm -e PODVM_MAPS=/proc:/etc public.ecr.aws/docker/library/alpine:3.20 sh -c 'test -z "$(ls /proc)"'`
             (The `-v` spelling the entry was authored with does not exist;
             `-e` carries the same table. The broad map is the point: under a
             map covering `/proc`, the directory still reads as itself, and
             `/proc` is unmounted in payloads, so a `readlink` proof cannot
             run there.)

**Done 2026-09-22.** The never-rewrite predicate lives in `map.rs`:
`excluded` answers the built-in `/proc` tree first (not removable) and
the colon-separated `PODVM_EXCLUDE_PATH` list second, consulted from
both `longest` and `longest_to`, so machinery paths never rewrite
forward and results naming them never read back virtual. Unit tests pin
the tree, the boundary (`/proctor`, `/procself` still map), empty list
items, and both directions under broad tables. Driven green on host
podman with the shipped binary, four rows on alpine: `ls /proc` reads
empty under `/proc:/etc` (red without the change),
`readlink /proc/self/exe` still fails honestly where `/proc` is
unmounted, a
user-excluded `/mapped/hosts` is absent (red without), and a near-miss
`/mappe` list still maps. The T-0705 eight-row drive re-ran green
beside it, so narrow maps still rewrite and reverse.

---

### T-0708 Intercept the operations the runtime cannot provide

Source:      `TOOL.md` section 6.7
Category:    interpose
Priority:    P2
Effort:      M
Status:      done 2026-09-22

Problem:     A payload that calls `mknod`, `mount`, `unshare` or `clone` with
             namespace flags gets an error the runtime already knows the reason
             for, and the payload usually stops rather than adapting.
Premise:     Read. `TOOL.md` section 6.7 lists four: `mknod` becomes a regular file,
             `mount` becomes an in-memory table plus success, `unshare` becomes
             success and a memo, `clone` has its namespace flags stripped before
             the real call.
             pathmap already exports `mknod`, `mount_setattr`, `move_mount`,
             `open_tree`, `pivot_root`, `umount2` and `chroot`, which is
             evidence that the entry points are reachable; what it does with
             them is rewrite the path and call through.
Approach:    Implement the four. ⛔ Every one of them is a lie to the payload and
             each must be counted and reported: `inspect` carries the tally, and
             the banner says the tier is emulating them. A `mount` that returned
             success and mounted nothing is exactly the class of defect the
             honesty rules exist for, and the only thing that makes it
             acceptable is that podvm says so.
Decision:    Strip namespace flags from `clone` rather than failing it. Failing
             it stops shells and build systems that set them speculatively;
             stripping them produces a process that works with less isolation,
             which is the mode already reported.
             ⚠ Do not strip `CLONE_NEWNET` silently: an empty netns is
             `ENETUNREACH` for every connection, which reads as a network outage
             rather than as a stripped flag. Report it.
Prove:       `podvm create --name e1 public.ecr.aws/docker/library/alpine:3.20 sh -c 'mknod /tmp/n c 1 3 && test -f /tmp/n' && podvm start e1 && podvm wait e1 && podvm inspect --format '{{.Interpose.Emulated.mknod}}' e1 | grep -q '^[1-9]'; rc=$?; podvm rm -f e1 >/dev/null 2>&1; exit $rc`

**Done 2026-09-22.** The tally rides the ownership memo file through the
same descriptor (`dev` `u64::MAX`, the operation in the inode word: 1
mknod, 2 mount, 3 unshare, 4 clone). A mount header carries the flags and
both chain lengths, and the paths follow as 32-byte continuation records
in one `writev`, so concurrent payload processes cannot interleave a
chain. The path takes no allocation and no lock. `inspect` reads the
tally back (`Interpose.Emulated.mknod/mount/unshare/clone`, a dash where
the tier never ran), and the banner states the capability on every load.
Five entry points ship the four operations (`mknod`, the old-glibc
spelling `__xmknod`, `mount`, `unshare`, `clone`), all single-version on
glibc and one version per name on musl. `mknod` becomes a regular file
and refuses a directory with `EPERM`. `mount` records its chains and
answers 0. `unshare` records and answers 0, and forwards a flag-less call
genuinely. `clone` strips the eight namespace flags and forwards the
rest, exactly and uncounted where nothing stripped (the thread path). No
tally behind it means the real call on all five: a success nothing
counted never leaves the object. The `CLONE_NEWNET` strip is loud on the
clone path and the unshare path. A mknod tally that does not land unmakes
the stand-in and fails with the write errno; a review audit of the entry
text against the code found the discarded return it replaces.

Driven green on host podman 6.1.2 with the shipped binary
(`ELF_MAGICS=8`), ten rows, one marker each (`.dev/t0708-drive.sh`):
E1 mknod tally, E2 mount tally, E3 unshare tally, E4 a direct `clone()`
with `NEWUSER|NEWNET` through python ctypes (`CDLL(None)`, an `_exit`
child that never enters Python) asserting success, tally 1 or more, the
loud line in `logs`, and `EPERM` on a directory `mknod`, E4m the same on
musl, E5 eight threads starting and joining, E6 the banner line, E7 an
existing path failing honestly, E8 the `NEWNET` loud line, E9 the memo
symlinked to `/dev/full` where `mknod` fails and leaves no file (E9 reads
no `inspect`: the scan streams, and only a regular memo terminates it).
E4 replaces a fork-based row: busybox `unshare --fork` uses fork plus
`unshare` and never calls `clone` with namespace flags. Regression: 240
exits 0 with 10 rows run and 10 built and ran, zero decline lines, the
only transcript diffs the new banner line and installer progress noise;
the T-0705 eight-row drive and the T-0707 four-row drive stay green. Unit
guards: five `emulate` tests, the tally-scan test that drops torn chains,
and the interpose suite at 25 of 25 in the lane check.

⛔ **The `Prove` above is amended, and the committed spelling could not
work.** It ran `run --rm` and read `ps -lq`. A foreground run keeps no
record: `--rm` removes the memo `inspect` reads, and without it the
ephemeral memo is deleted on exit. `ps` takes no `-l` flag
(`crates/podvm/src/lifecycle.rs` `ps`). The tally rows use the
create/start/wait/inspect/rm cycle.

---

### T-0709 Select the interposer by `DT_NEEDED`, and refuse on the version predicate

Source:      `experiments/results/interposer-abi.txt`
Category:    interpose
Priority:    P0
Effort:      M
Status:      done

Problem:     T-0702 establishes that one object per libc is required. That
             leaves the question of which object a given payload gets, and
             answering it by trying one and seeing whether it loads costs a
             failed container and produces an error naming a symbol, which
             sends the reader to debug the symbol rather than the mechanism.
Premise:     ⭐ **Both facts are one `readelf` away, measured with no run.**
             `experiments/results/interposer-abi.txt`:
             check A, the SONAME discriminator. glibc's libc declares SONAME
             `libc.so.6`; musl's declares none, so an object linked against it
             records `libc.so` in `DT_NEEDED`. There is no ambiguity and no
             heuristic.
             check C, the version predicate, **predicted from ELF and then
             confirmed by the loader**: the object imports up to `GLIBC_2.34`,
             the pinned payload libc declares up to `GLIBC_2.31`, the prediction
             was `refused`, and the observed failure was
             `version 'GLIBC_2.34' not found (required by /i.so)`. The loader
             named the same version the prediction did.
             check D, the trap that makes a naive implementation refuse
             everything: a shipped `libc.so.6` is stripped. On this host
             `.symtab` has **0** defined symbols and `.dynsym` has **3136**. A
             reader asking `.symtab` concludes the payload's libc defines
             nothing, refuses every artefact, and reads exactly like a check
             that works.
Approach:    At extract time, for the rootfs just written:
             1. read the rootfs's libc `DT_NEEDED` and SONAME and pick the
                matching object;
             2. for every symbol the chosen object imports, assert the payload's
                libc defines it, **at a symbol version that libc declares**,
                reading `.dynsym` and never `.symtab`. Comparing names alone
                misses the common real failure, which is a build host newer than
                the target;
             3. on a mismatch, refuse **by name**: say which libc the rootfs
                carries and which the object was built against. T-0706 owns the
                refusal channel and T-0108 the wording.
             ⚠ A coarse guard in front of it is one glob and its value is the
             message: a rootfs carrying `ld-musl-*` handed a glibc object should
             say so in those words, not through a relocation error.
Decision:    Select and assert, never try. The assertion is cheap, it runs
             before anything is executed, and it turns a runtime failure inside
             somebody's container into a refusal with a reason.
Prove:       `./experiments/80-interposer-abi.sh` exits 0, and its check E is this reader answering what the loader answered in A, B and C


**Done 2026-09-09.** `src/abi.rs` of `pod-enter` (PodUtils/podcrates), driven by
`podvm system abi <object> <libc>` and asserted by check E of
`experiments/80-interposer-abi.sh`.

⛔ **The `Prove` above was amended, and the reason is that its second half could
not fail.** It read `podvm run --rm alpine:latest true 2>&1 | grep -qv 'symbol
not found'`; with no interposer to preload, no run can produce that string, so
the clause passes on a tree where nothing is implemented. Check E is the
assertion that can go red: the reader's verdict is compared with what the loader
actually did in checks A, B and C, over the same objects and the same pinned
payload libcs.

```
== E. podvm's own reader, against the same four situations
  control glibc obj -> host libc     rc=0    admitted
  glibc obj -> musl libc             rc=1    refused (names musl and glibc)
  control musl obj -> musl libc      rc=0    admitted
  musl obj -> host glibc libc        rc=1    refused
  C's situation, read not run        rc=1    refused, naming GLIBC_2.34
ok   E: the reader's answer is the loader's, on every arm that ran
```

⭐ **The libc is FOUND rather than named.** There is no table of
`lib/x86_64-linux-gnu/libc.so.6` in the reader: the payload's own `PT_INTERP`
names the loader that will run it, and on musl that file IS the C library while
on glibc the libc sits in the loader's own directory. A table would be a list of
the distributions somebody thought of, and `src/identity.rs` of `pod-rootfs` (PodUtils/podcrates)
already carries one for a coarser question.

Four things the implementation had to get right, and three of them were found by
running it rather than by reading:

1. ⛔ **`.dynsym` and never `.symtab`**, which is the entry's own trap and the
   only one that was known in advance. Check D measures it: 0 defined symbols in
   `.symtab` and 3136 in `.dynsym`;
2. ⛔ **A WEAK undefined symbol is not a requirement.** The first working reader
   refused its own control: the glibc object imports
   `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable` and
   `__gmon_start__`, none of which `libc.so.6` defines, and it loads. GCC's
   `crtbegin` emits them `STB_WEAK` and the loader binds them to 0. A reader
   that treated them as requirements **refuses every object gcc produces**,
   which is the same shape as the `.symtab` trap: it reads exactly like a check
   that works;
3. ⛔ **The version is checked BEFORE the name, because that is the loader's own
   order.** The pinned glibc 2.31 payload does not define `dlsym` in `libc.so.6`
   at all -- glibc 2.34 merged `libdl` into `libc` -- so the symbol is both
   undefined and at an undeclared version. The loader says `version 'GLIBC_2.34'
   not found`; a reader asking "is it defined" first said `dlsym` is missing,
   which is true and sends the reader to look for `libdl` rather than at the
   build host;
4. ⛔ **`VER_FLG_BASE` is not a version.** The first entry of `.gnu.version_d` is
   the file's own SONAME wearing a version entry's clothes, so collecting it put
   `libc.so.6` in the declared list beside `GLIBC_2.39`, where it sorts after
   every real one and became the "declares up to" the refusal reported.

⚠ **The OBJECT was still open until 2026-09-18, not the reader.** Nothing
preloaded anything: [T-0701](#t-0701-the-cdylib-build-constraints) and
[T-0702](#t-0702-one-object-per-libc-and-it-must-live-inside-the-rootfs) were the
cdylib and the placement, and this reader is what chooses between the two
objects they produce. The placement landed and the reader now serves it
through [T-0706](#t-0706-classify-the-payload-and-decline-with-a-named-reason)'s
channel.

⭐ **The reader checks the link set since 2026-09-18, not the libc alone.**
A Debian payload declined every run because the object imports
`_Unwind_Resume@GCC_3.0`, which no `libc.so.6` declares and `libgcc_s.so.1`
beside it does. `abi::union` merges the libc with the runtimes beside it
for the check; `podvm system abi` keeps the direct two-file question. The
multiarch search that found that libc is in the same change.

⭐ **A second thing this closed, in the harness rather than in podvm.**
`experiments/80-interposer-abi.sh` exited **2** on this container because
`musl-gcc` is absent from it, so question B and both musl arms of check E could
not be taken. It now falls back to `scripts/zig-cc.sh`, which is the compiler
`.cargo/config.toml` already names for the musl target and which carries the
musl sources it compiles against. The script exits **0** for the first time and
the conditions block says which compiler produced the object, because "the musl
arm ran" means something different depending on it.

---

### T-0710 The ownership memo lives where the payload can edit it, and answers by linear scan

Source:      `crates/podvm-interpose/src/memo.rs`; `experiments/results/interpose-ownership.txt`
Category:    interpose
Priority:    P1
Effort:      L
Status:      done 2026-09-19

Problem:     T-0704's memo makes `chown` answerable on a runtime that refuses
             it, and the shape it took to get there is a append-only log at
             `/.podvm/ownership.memo` **inside the payload's own rootfs**,
             read back by scanning it backwards for the last record naming a
             `(dev, ino)` pair. Three consequences, and the first is the one
             that matters:
             ⛔ **A wrong answer, never an error.** `lookup` stops at a 4 MiB
             scan ceiling. A rootfs past it answers with the ownership the
             payload set EARLIER rather than reporting that it cannot say, so a
             `chown` that succeeded reads back as the original uid and the
             payload concludes its own call did nothing.
             ⚠ The payload can `rm` it, truncate it, or fill it. Everything in
             the rootfs is the payload's, which is the property that makes
             `podvm exec` see what the first process wrote and also the
             property that makes the record forgeable.
             ⚠ It grows without bound: one 32-byte record per `chown`, and a
             `chown -R` over a distribution tree writes one per inode with no
             compaction, so the scan gets slower exactly where it is used most.
Premise:     Measured, `experiments/105-interpose-ownership.sh` checks C, D
             and E: `MEMO=none` where the real `chown` worked, `MEMO=32` after
             one virtualized call under each libc. The record is `dev` u64,
             `ino` u64, `uid` u32, `gid` u32, `set` u32, pad u32, written with
             one `O_APPEND` write and no lock, which is atomic for a
             fixed-size record and is why there is no lock to begin with.
             ⚠ fakeroot's own answer is a daemon holding the table in memory
             (`references/salsa-debian__fakeroot/tree/libfakeroot.c:875-907`), which podvm cannot copy: there
             is no process to hold it across `podvm exec`, and a daemon is the
             notification tier T-0606 is blocked on in a different shape.
Approach:    Three questions, and the first is the operator's to rule:
             1. **Where it lives.** Inside the rootfs, visible and forgeable, or
                beside the container record on the host, where the payload
                cannot reach it and an interposer must therefore be handed a
                descriptor to it at spawn. ⚠ The second costs nothing at
                `chown` time and everything at `exec` time, because a fresh
                chroot re-entry has to be given the same descriptor.
             2. **A bounded read.** An index, a fixed-size hash table sized at
                creation, or a compaction pass that rewrites the log at a
                threshold. Whatever it is, ⛔ a lookup that cannot answer must
                say so, and the caller's answer is then the real `stat`'s,
                marked degraded, never a stale record.
             3. **What a forged record may do.** It can name any `(dev, ino)`,
                so a payload can make any file read as owned by anyone. That is
                inside the payload's own trust boundary and harmless there;
                it stops being harmless if podvm ever reads the memo to decide
                something on the host, which nothing does today and which this
                entry should forbid in writing.
Decision:    **The memo lives on the host, beside the container record. Ruled by
             the operator on 2026-09-11.** The payload cannot read it, forge it,
             truncate it or delete it, so the forgery question closes instead of
             being argued about each time somebody reads the entry.
             ⛔ **The cost is real and it is in the spawn path, not at `chown`
             time.** `podvm exec` re-enters the chroot, so every entry must be
             handed a descriptor to the memo, and an entry that is not handed
             one must REFUSE rather than start with no memo: a second process
             that silently has no ownership record answers a `stat` with the
             real uid and contradicts the first process.
             ⛔ **Nothing on the host may read the memo to decide anything.** It
             is the payload's own view of ownership and it is not evidence. That
             was true while the record was forgeable and it stays true now, so
             the rule does not depend on where the file sits.
             ⚠ Question 2 is unchanged and is not the operator's: a lookup that
             cannot answer says so, and the caller falls back to the real
             `stat`, marked degraded. ⛔ Never a stale record.
Prove:       `experiments/105-interpose-ownership.sh` gains a check G: write
             more records than the ceiling admits, then read back a pair
             recorded before it, and assert the answer is either correct or a
             refusal, never a stale one. ⛔ The current code fails that check,
             which is why it is written before the fix rather than after.


**Done 2026-09-19.** The memo lives on the host beside the container record
(`podvm-supervise/src/table.rs` `memo_path`, `MEMO_CHILD_FD` 17,
`PODVM_MEMO_FD`), handed as a descriptor at every spawn (`run` foreground
ephemeral under staging, `create`/`run -d` moved to `containers/<id>/`,
`exec` re-handed, launcher and T-0412 steps sharing it) and refused where not
handed (`interpose::apply` returns `Err`). The interposer reads and writes
through the handed number only (`memo::memo_fd`, seek-to-end record, rewind
lookup); past 4 MiB `lookup` returns `BeyondCeiling` and the `stat` family
answers the real owner marked degraded, never a stale record.
`experiments/105-interpose-ownership.sh` check G drives it end to end with
`experiments/results/interpose-ownership.txt` carrying the run: 7 checks, exit
0 (OLD_STAT=0:42, NEW_STAT=0:0 refusal with the degraded line, 5242944 bytes).
A guest `podvm run` smoke test answered 0:42 through the host memo. Nothing
on the host reads the memo to decide anything.

⚠ **One lane trap this change records.** The 105 harness hands fd 9 and not
17: the old glibc payload's `/bin/sh` is dash, which redirects single-digit
descriptors only (`exec 17<>` answers `17: not found`, measured 2026-09-19).
podvm hands 17 by `dup2` with no shell involved, so production is unaffected.

---

### T-0711 The identity calls, and podvm's honesty rules point the other way from fakeroot's

Source:      `TOOL.md` section 6.7; `references/salsa-debian__fakeroot/tree/libfakeroot.c:669-678`
Category:    interpose
Priority:    P1
Effort:      L
Status:      done

Problem:     A payload that is already uid 0 still calls `setuid`, `setgid` and
             `setgroups`, and on the runtimes podvm targets they fail. A
             package manager that drops to an unprivileged user to run a
             maintainer script, `sshd`, and every build system with a "do not
             run as root" guard all take that path.
             ⛔ **The fork is unruled and the two answers are incompatible.**
             fakeroot answers by LYING: it records the requested id and reports
             it back through `getuid`, so the payload believes it dropped
             privilege. podvm's own honesty rules
             ([cli.md](cli.md) T-0804) forbid output implying a property the
             runtime does not provide, and "you are now uid 1000" on a process
             that still holds every capability bit is exactly that.
Premise:     T-0704 already virtualizes ownership and the wall is the same one:
             `EPERM`, or `EINVAL` where the runtime rejects the argument rather
             than the permission. ⚠ Unlike `chown`, this one is **not**
             confined to what a later `stat` reports: a payload that believes
             it dropped privilege takes different code paths afterwards, so a
             lie here changes behaviour rather than a reported number.
             ⚠ The `EINVAL` half of T-0704 is unexercised, and the same arm is
             load-bearing here, so the two should be measured together.
Approach:    Measure first, on the matrix `experiments/125-across-distributions.sh`
             already drives: which of `setuid`, `setgid`, `setgroups`,
             `seteuid` and `setresuid` a real target refuses, with which errno,
             and what the common callers do with the failure. ⚠ The answer may
             be that the honest refusal is FINE, because the callers already
             handle a failed drop; that is a measurement and not an assumption,
             and it decides the entry.
             Then rule, in writing, one of three:
             1. **Refuse honestly.** The call fails as it does today and the
                banner names it. Costs: a payload that treats a failed drop as
                fatal cannot run.
             2. **Lie, and say so everywhere else.** Record the id, answer
                `getuid` from the record, and mark the container degraded so
                `--strict` refuses it. ⛔ This is a behaviour change inside
                somebody else's process, which is a larger thing than the
                trust change T-0407 already carries.
             3. **Neither by default**, with a flag. ⚠ A flag is not a ruling:
                it is two rulings and a way to pick, and the default is still
                the decision.
Decision:    **Answer 3, and the default is the honest refusal. Ruled by the
             operator on 2026-09-11.** The call fails as the runtime fails it,
             the diagnostic names the mechanism and the errno, and a flag turns
             on the fakeroot behaviour for a payload that needs it.
             ⚠ **The entry's own warning about answer 3 is accepted rather than
             dismissed: a flag is two rulings and a way to pick.** Both rulings
             are therefore written here. The default is honest because
             [cli.md](cli.md) T-0804 forbids output that implies a property the
             runtime does not provide. The flag exists because a payload that
             treats a failed drop as fatal cannot otherwise run at all, and
             refusing every such payload is a third behaviour nobody chose.
             ⛔ **The flag marks the container degraded and `--strict` refuses
             it**, which is the same shape T-0407 already carries for the host
             CA bundle. ⛔ A run under the flag says so in the banner, because
             this entry exists to stop podvm and its own output disagreeing
             about who the payload is.
             ⚠ The measurement in `Approach` is still owed and still decides the
             SHAPE of the honest refusal: which errno each call returns on a
             real target, and what the common callers do with it. It no longer
             decides the default.
Prove:       `./experiments/106-interpose-identity.sh`, which runs a payload
             that calls `setuid(1000)` and then `getuid()` under the object on
             both libcs, asserts whichever of the three answers the ruling
             picks, and asserts the BANNER says the same thing the call did --
             because the failure this entry exists to prevent is podvm and its
             own output disagreeing about who the payload is.


**Done 2026-09-19.** The ruling's both halves are implemented, and `106`
drives them end to end with `experiments/results/interpose-identity.txt`
carrying the run: 14 checks, exit 0. Without the flag the object calls
through and names the failure with its errno (clause A holds the loaded
report identical to the bare one, musl honest checked against the same
kernel's bare glibc lines in C0); with `--user 1000:100` the setters
answer 0 and every getter answers the record on both libcs (B, C,
including `setreuid(2000,-1)` reporting `2000:1000:1000`, the kernel's
saved-follows-effective rule); `podvm run --user` makes `id` answer the
request on both libcs with the banner naming the faked identity (D);
`--strict` refuses naming `--user` through its Degraded parity row (E);
and a static payload with `--user` is declined by name and still runs (F).

The shape the entry ruled: `run`, `create` and both `exec` paths resolve
`--user` through one call (`crates/podvm/src/lifecycle.rs`
`apply_user`, numeric ids or names from the image's own files, a bare
name taking its primary group from passwd) into `PODVM_IDENTITY`, which
the object reads per process with no lock and no allocation
(`crates/podvm-interpose/src/identity.rs`), surviving `fork` by copy
and `execve` by re-read. The sixteen identity entry points are declared
in `interpose.map` and asserted against it by `105` check A. What the
entry's `Approach` still names as owed, the errno-by-call measurement
on a real target deciding the honest refusal's shape, stays open as
follow-up work, not as this entry: the default no longer depends on it.

**Partial, 2026-09-25.** Issue 52 reopens this entry on that owed
measurement: take the errno-by-call reading on a real target and
record it, then let the refusal text name the call and the errno.

**Done 2026-09-26.** The reading is taken and the refusal names
both. `106` gains the four calls the matrix names but the victim
did not (`seteuid`, `setegid`, `setresuid`, `setresgid`) and a
clause G that prints the errno-by-call table on the denying
machine at hand: recorded, never asserted absolutely, because
errnos vary by wall. The refusal text now carries the errno name
beside its number (`setgid failed with errno 1 (EPERM)`), from a
table of the wall's usual numbers with a unit test; an unmapped
number stays a bare number rather than a guess. `106` exits 0
(`experiments/results/interpose-identity.txt`, lane job kernel
`7.2.0-WSL2-STABLE`, uid 0 in a userns):

| call | rc | errno |
| --- | --- | --- |
| setuid(1000) | 0 | 0 (granted: this wall maps the id) |
| setgid(100) | -1 | 1 (EPERM) |
| setgroups(2, ...) | -1 | 1 (EPERM) |
| seteuid(1000) | 0 | 0 (granted, same mapping) |
| setegid(100) | -1 | 1 (EPERM) |
| setresuid(-1, 1000, -1) | 0 | 0 (granted, same mapping) |
| setresgid(-1, 100, -1) | -1 | 1 (EPERM) |
| setreuid(2000, -1) | -1 | 1 (EPERM) |
| setregid(200, -1) | -1 | 1 (EPERM) |

The calls run in order, so later rows inherit earlier ones'
effects (a granted drop shapes what follows); the victim source
in the script is the order. A granted call stays silent, as
before: success needs no diagnostic. All older clauses held
beside the new one, and the `wall_errnos_carry_their_names`
unit test is green with the interpose suite.
Keep the current default as the fallback. Fix area is
`crates/podvm-interpose/src/identity.rs` with `interpose.map`.
Risk is low: the gap is the honesty of the refusal wording, not a
silent wrong answer. Prove is the recorded reading with the
refusal naming the call and the errno beside it.

⚠ **Two substitutions this run records.** The job container cannot run a
docker daemon (dockerd fails creating the DOCKER chain, `iptables ...
Permission denied`, measured 2026-09-19), so clauses A, B and C0 run the
victims directly rather than under `docker run -v`: the fake answers the
record with or without a wall, and the control compares bare against
loaded rather than against the kernel, which is why no clause asserts an
absolute honest-failure rc. The musl victim cannot execute on the glibc
job container (its interpreter is the absent musl loader), so clause C
runs it staged inside the alpine rootfs through podvm itself.


---

### T-0712 A reach matrix holds its arguments constant, or it measures two things

Source:      `references/talaria0101__sandbox-insights/tree/scripts/ldpreload/victim_dyn.c`,
             `references/talaria0101__sandbox-insights/tree/scripts/ldpreload/victim_static.c`,
             and `references/talaria0101__sandbox-insights/tree/experiments/logs/35-interposition-reach.log`
Category:    interpose
Priority:    P2
Effort:      S
Status:      done

Problem:     [T-1110](milestones.md) drives a payload matrix across payload
             classes, and [T-0706](interpose.md) owns the named decline for the
             classes the tier cannot reach. **A matrix that changes two things
             between its rows measures neither**, and the fixture podvm would
             copy has exactly that defect.
Premise:     **Read at the line, and the header disagrees with the code.**
             That fixture's header and its README both say the three victims run
             "the same two syscalls each". The syscalls are the same and the
             ARGUMENTS are not: the dynamic victim asks for an **unmapped** gid
             42, while the static and the Go victim both ask for gid 0, which is
             **mapped**.
             **The reach result still stands**: no `SHIM:` line appears for
             the static or the Go victim in the log, so neither is reached, and
             that is what the fixture was built to show.
             **The wall result does not.** The log line
             `victim_static: lchown -> 0` reads as a static payload clearing the
             ownership wall, and it is nothing of the kind: it asked for a gid
             that is mapped. A reader taking that row as evidence would conclude
             that static payloads escape wall 1.
Approach:    podvm's own matrix holds the argument constant across every row and
             varies **only** the payload class. One unmapped target for every
             row, so "reached" and "cleared" are two columns of the same table
             rather than two experiments.
             Two columns, not one verdict: **seen** by the interposer, and
             **cleared** by it. The dynamic row is the only one that can be yes
             to both, and saying so is the point of the table.
Decision:    Followed as written on 2026-09-18: no alternative was tabled,
             and `245` implements the Approach verbatim.
Prove:       `./experiments/245-interpose-sweep.sh` prints `seen` and `cleared` as separate columns, and every row's `lchown` target gid is identical and unmapped


**Done 2026-09-18.** The sweep's table carries `SEEN` and `CLEARED` per row
and every class asks gid 42, unmapped: dynamic rows read yes/yes, the
static and Go victims decline by name, and `experiments/results/sweep245/`
holds the transcripts. The subject is one script for all rows. (Via `chown`
on a regular file, where `chown` and `lchown` coincide: no row ships a tool
that spells the latter, and the constant unmapped target is what the rule
is about.)

---

### T-1309 libdnf repodata downloads fail under the preloaded interposer

Source:      `experiments/240-distro-sweep.sh` rocky and rocky-minimal rows;
             `crates/podvm-interpose/src/lib.rs` (the interposed symbol set)
Category:    interpose
Priority:    P1
Effort:      M
Status:      done

Problem:     The rocky and rocky-minimal rows of `240` fail under podvm
             while the engine control builds and runs 42 on the same image.
             `dnf`/`microdnf` (libdnf) report `Yum repo downloading error:
             Invalid path: repodata/<hash>-primary.xml.gz` after a slow
             trickle. Fedora's dnf5 and the apk, apt, pacman, zypper and
             xbps rows are unaffected, so this is libdnf-family specific
             rather than a broken mirror.
Premise:     Measured 2026-09-19 on host podman, fixed URL, back to back,
             same image, same store shape:
             with the interposer, `dnf makecache` against
             `https://dl.rockylinux.org/pub/rocky/9/BaseOS/x86_64/os/`
             fails with the Invalid-path error; with `env -u LD_PRELOAD`
             the same command writes `Metadata cache created`. Mirror
             lottery and time flapping are refuted by the fixed URL and
             the minutes-apart pairing. DNS answers were read on both
             sides and are identical (one AAAA, Fastly), the resolver is
             `10.255.255.254` on both, and both share one network
             namespace, so neither names the cause. The interposer wraps
             no socket symbol (`crates/podvm-interpose/src/lib.rs`: chown,
             stat, open, path and identity families only), so socket
             shaping is refuted by reading; a file-path operation in the
             download path (`mkstemp`, `statx`, `openat` handling) is the
             suspect, unconfirmed.
             Correction measured 2026-09-22 on host podman 6.1.2: the
             suspect above is wrong. No file-path operation fails. An
             `LD_PRELOAD` shim that forwards `realpath` untouched
             reproduces the failure with no podvm and no interpose
             object. librepo calls `realpath(path, NULL)`. The forwarded
             call answers NULL with `EINVAL` on paths that `stat`
             confirms exist. Bare succeeds on the same paths. An
             in-process three-way resolution names the cause.
             `dlsym(RTLD_NEXT, "realpath")` returns the `GLIBC_2.2.5`
             compat. Its body tests `resolved` for NULL and answers
             `EINVAL` (read with `objdump -d` from the pinned image
             libc). `dlvsym` for `GLIBC_2.3` returns the default and
             resolves the same paths. Bare binds the default through
             the versioned PLT reference (`objdump -T` on librepo
             shows `realpath@GLIBC_2.3`). An unversioned `dlsym` returns
             the compat on this libc (measured in-process: the same
             pointer `dlvsym` answers for 2.2.5). The payload
             libc carries compat pairs for seven wrapped names:
             `realpath`, `glob`, `glob64`, `nftw`, `nftw64`,
             `posix_spawn`, `posix_spawnp` (read with `objdump -T`).
             `canonicalize_file_name` carries one version and is safe.
             A forward-only shim passes only where its own log shows
             the wrapper never fired. Every run with a confirmed firing
             wrapper fails.
             This blocks [T-1211](gate.md): `240` exits 1 while these two
             rows read no-compiler, and unsetting `LD_PRELOAD` in the sweep
             would test a different product, so the sweep stays as it is.
Approach:    Forward the seven multi-version names to their default
             versions with `dlvsym(RTLD_NEXT, name, version)` on glibc,
             with a `dlsym` fallback where the default is absent, so a
             payload with an older libc keeps today's answer. musl keeps
             `dlsym` and has no versions. Extend the `real!` resolver
             with an optional version. No second resolver ships. Add a
             unit test in the oracle pattern: the wrapped
             `realpath(dir, NULL)` beside libc's, with the same non-NULL
             answer. Prove the test red once by pointing the wrapper at
             the 2.2.5 compat, then green after the fix. Out of
             scope: the T-1211 conversions (done, and 240's attribution is
             what found this), the static 1 MB `/dev/urandom` shim the
             probes also showed (T-0401's area, reported separately), and
             the almalinux row, which fails identically under the engine
             and is the host's mirror, not the runtime. Versioned-pair
             exports per caller version stay out: the swept payloads all
             reference the defaults, and the fallback keeps older ones
             working.
Decision:    Fix in the interposer, not around it. The control exists to
             separate "podvm is missing a fixup" from "this machine
             cannot do it either", and here it names podvm: no sweep-side
             workaround is legitimate.
Prove:       `./experiments/240-distro-sweep.sh` exits 0 with both
             libdnf rows reading 42, on host podman, with the
             conditions block naming the driver.

**Done 2026-09-22.** The seven multi-version names resolve to their
defaults through `dlvsym` on glibc, with a `dlsym` fallback where
the default is absent, and musl keeps `dlsym`. The `real!`
resolver grows an optional version arm. No second resolver ships.
Guard: `realpath_null_resolved_matches_libc` calls the wrapped
`realpath(dir, NULL)` beside libc's and requires the same non-NULL
answer, plus `versioned_lookup_falls_back_to_dlsym` for the
fallback with a version no libc defines. The guard fires red on
the planted unversioned resolution on the rocky payload libc
(12 pass, 1 fail with the defect's signature) and green after
the fix there (13 of 13) and in the lane (14 of 14).
`versioned_lookup_falls_back_to_dlsym` covers the fallback branch
with a version no libc defines. The lane build holds the 2.27 ceiling (newest
need `GLIBC_2.14`). End to end on host podman 6.1.2 with the
shipped binary: `240` reads 10 rows, 10 ran, 10 built and ran
(`experiments/results/distro-sweep.txt`), both libdnf rows 0 0
42, each filtered run exiting 0 observed. A link-time `dlvsym`
reference was tried first and stamps `GLIBC_2.34`, which breaks
the ceiling; resolving it at runtime through the pinned `dlsym`
is the shape that holds. Refuted along the way, so nobody
re-tries them: a file-path operation in the download path (the
entry's first suspect), the socket layer, parenthood, command
shape, mirror content, byte corruption, ABI mismatch and
threads; the failure reproduces with a forwarding-only shim and
no interposer at all.

### T-1312 The glibc interposer needs newer symbols than the payload provides

Source:      the T-1111 nix unpack (`experiments/results/nix-acceptance.txt`
             carries the loader error);
             `crates/podvm-interpose/src/real.rs:28`
Category:    interpose
Priority:    P1
Effort:      S
Status:      done

Problem:     The nix 2.2.2 closure unpacks under podvm now (T-1311), but no
             nix binary starts: the loader refuses each one with
             `version 'GLIBC_2.34' not found (required by
             /.podvm/interpose.so)`. The closure ships glibc 2.27, and the
             preloaded object binds symbols its libc predates, so the whole
             T-1111 pipeline stops at REGISTER.
Premise:     Measured 2026-09-21 in the lane: `readelf -VW` on the shipped
             glibc object needs `GLIBC_2.34` for `dlsym` and `GLIBC_2.30`
             for `gettid`, and nothing else above 2.27. The `dlsym` reference
             is this object's own (`crates/podvm-interpose/src/real.rs:28`);
             glibc re-versioned the merged `libdl` symbols at 2.34, so any
             link against a newer libc stamps it. No crate code calls
             `gettid`: the reference arrives with Rust std, which this
             object links. Everything else the object needs is 2.14 or
             older, so these two are the whole gap.
Approach:    Link `dlsym` against the `dl-stub.S` import library, which
             records the reference under `libdl.so.2`: the library that
             defines it on both sides of glibc's 2.34 `libdl` merge. (The
             `2` is libdl's own SONAME version, unrelated to `libc.so.6`.)
             The reference carries `GLIBC_2.2.5` through `.symver` and the
             stub defines that node through `dl-stub.map`; the stub travels
             first on the link through a linker wrapper, because user link
             arguments arrive after the objects, where `--as-needed` drops
             the stub outright and the surviving reference binds `libc.so.6`
             instead. All three failures were measured. Define `gettid`
             locally through the raw syscall number so std's reference
             resolves inside the object instead of against the payload's
             libc. Assert the ceiling in `scripts/build-interpose.sh`: no
             `GLIBC_` need above 2.27, the closure's version, measured in
             the failing row. Out of
             scope: the musl object (no symbol versions, unaffected), a
             `no_std` rewrite (the larger answer, unneeded while the two
             references are the whole gap), and building against an older
             libc (fragile without the assertion, redundant with it).
Decision:    Fix in the interposer, not around it. Shipping a second older
             object, or exempting nix binaries from the preload, would both
             be a special case with a second thing to maintain; one object
             that loads anywhere back to 2.27 is the product T-0702
             describes.
Prove:       `./target/release/podvm-interpose-build` exits 0 with the ceiling
             assertion recorded in its output, and the 152 REGISTER row
             reads ok (recorded under the T-1111 run).

**Done 2026-09-21.** The glibc object links `dlsym` against the
`dl-stub.S` import library first on the line through the
`gnu-link-stub.sh` linker wrapper, recording `dlsym@GLIBC_2.2.5` under
`libdl.so.2`, and defines `gettid` as a bare assembly label that stays
local. `build-interpose.sh` asserts three things per object and all hold:
newest need `GLIBC_2.14` within the 2.27 ceiling, no uppercase `gettid`
in `.dynsym`, `libdl.so.2` needed on glibc, with the export set agreeing
with `interpose.map` both ways and the unit suite at 12 of 12. The loader
proof loads the object under the closure's own glibc 2.27 (static probe:
bare 0, preloaded 0, control 0). End to end on host podman 6.1.2 with
binary `bc9dbb58`: 152 reads `row REGISTER ok closure registered` and
`row FETCH ok "/nix/store/di36mqc6y19ivaa4qjrb2l82c6dqg7m3-source"`.
Refuted along the way, so nobody re-tries them: a `.symver` pin alone
binds `libc.so.6`; a plain `-ldl` resolves through the host linker script
into the same `libc.so.6` or vanishes under `--as-needed`; a Rust
`gettid` definition exports no matter the version script; `objcopy
--localize-symbol` leaves the dynamic entry; `.symver` in the stub
defines no version node; and the SONAME is `libdl.so.2`, not `.6`.

### T-1311 The interposed `fchmodat` drops the `flags` argument

Source:      the T-1111 nix unpack (the symptom is in
             `experiments/results/tar-symlink-modes-prefix.txt`);
             `crates/podvm-interpose/src/lib.rs:356`,
             `crates/podvm-interpose/src/stat_calls.rs:296`
Category:    interpose
Priority:    P1
Effort:      S
Status:      done

Problem:     Unpacking a tarball that carries symlinks fails under podvm
             while the engine control unpacks it cleanly. GNU tar reports
             `tar: ./l: Cannot change mode to rwxrwxrwx: No such file or
             directory` for each link and exits 2, so the nix 2.2.2 binary
             tarball (T-1111) does not unpack. Plain `chmod` on the same
             links succeeds, and regular files extract with their modes
             intact: only the mode-set on symlinks fails.
Premise:     Measured 2026-09-21 on host podman, same image, back to back:
             under `podvm run` the minimal tar-symlink repro exits 2 with
             one such error per link; in the plain driver container the same
             commands exit 0. The interposed `fchmodat` is declared with
             three arguments (`crates/podvm-interpose/src/lib.rs:356`) and
             forwards three (`crates/podvm-interpose/src/stat_calls.rs:296`),
             while libc takes four `(dirfd, path, mode, flags)`: the real
             call receives a fourth register the wrapper never set, so any
             caller passing flags gets the wrong answer. Tar's symlink
             mode-set passes `AT_SYMLINK_NOFOLLOW`. Every other
             flags-taking `*at` wrapper carries its flags (`unlinkat`,
             `faccessat`, `utimensat`, `linkat`, `execveat`, `openat2`,
             `fstatat`), so this is the one truncation, not the pattern.
             An empty path table returns the caller's pointer before any
             resolution (`crates/podvm-interpose/src/map/rewrite.rs:198`), which
             refutes the rewrite as the cause on runs with no table set.
             The ownership memo never sees `chmod`, which refutes that
             layer.
Approach:    Carry `flags` through the declaration and the wrapper. Add a
             unit test that compares the interposed `fchmodat` against
             libc's own for a symlink with `AT_SYMLINK_NOFOLLOW` and with
             flags 0. Drive the tar-symlink repro under podvm with the
             engine control beside it. Out of scope: the T-1309 libdnf
             failure (same family, a different call, still open), the
             T-0705 reverse mapping, and the non-native `$BIN version`
             line of 152 (T-1111's area, cosmetic).
Decision:    Fix in the interposer, not around it. The control names
             podvm, so no script-side workaround is legitimate: stripping
             symlink modes or passing tar permission-dodging flags would
             test a different product, the same ruling T-1309 carries.
Prove:       `./experiments/162-tar-symlink-modes.sh` exits 0 on host
             podman, with the conditions block naming the driver.

**Done 2026-09-21.** The declaration and the wrapper carry `flags`
(`crates/podvm-interpose/src/lib.rs:356`,
`crates/podvm-interpose/src/stat_calls.rs:296`); every other flags-taking
`*at` wrapper already did, so the audit closes with this one. Guard:
`tests::fchmodat_forwards_flags` compares the interposed entry point
against libc's own on a symlink with flags 0 and `AT_SYMLINK_NOFOLLOW`.
Pre-fix it fails with `left: (-1, 22), right: (0, 0)` on flags 0; post-fix
the suite is 12 of 12 with fmt and clippy clean. End to end on host podman
6.1.2 with the debian row 240 and 152 drive:
`experiments/162-tar-symlink-modes.sh` exits 1 on the pre-fix binary
(`TAR_RC:2` with one `Cannot change mode` error per link, control green;
`experiments/results/tar-symlink-modes-prefix.txt`) and 0 on the fixed
binary (unpack rc 0 with both links and the mode green on both sides;
`experiments/results/tar-symlink-modes.txt`).

---

### T-1327 The interposer builds per architecture, and the T-0704 citations point at it

Source:      issue 26, client beta testing 2026-09-22 (every binary
             embeds x86_64 objects; T-0704 cited for another subject);
             `.github/workflows/nightly.yml`, `crates/podvm-interpose`
Category:    interpose
Priority:    P2
Effort:      L
Status:      done 2026-09-23

Problem:     Every shipped binary embeds the x86_64 interposer pair
             (two ELF headers, `e_machine=0x3e`), so `interpose` declines
             off x86_64: on the project's own target class that leaves
             no rung at all on six of seven archs. No TODO entry covers
             per-arch objects (T-0702 is one object per libc), while the
             workflow and the release note cite T-0704, which is
             "ownership virtualization: the half a path interposer does
             not have", a different subject.
Premise:     Measured by the reporter across all seven beta.3 assets
             (ELF headers read per binary). The citation mismatch is
             read, not measured: T-0704's title names its subject.
Approach:    Build the pair per target arch in the nightly matrix (the
             cross link in `zig-cc.sh` is the shape to reuse), embed the
             matching pair per artefact, and fix the two T-0704 citations
             to name this entry. Out of scope: per-libc-per-arch
             coverage beyond what the matrix can prove (named per arch,
             not silently), changing the decline contract.
Decision:    Per-arch objects where the matrix proves them; where a
             target cannot build its pair, the artefact keeps the honest
             decline and the release note names the arch.
Prove:       `readelf -h` on each shipped binary's embedded objects
             reads back the binary's own `e_machine`; an off-arch
             payload on qemu-user selects `interpose` where the pair
             exists; `grep` finds no T-0704 citation on the publish path.
             Close issue 26 (objects third) with a comment showing the
             header reads and the per-arch build as the guard that stops
             recurrence.

**Done 2026-09-23.** Decision for the Decision, per-arch named: the
pair stays x86_64-only and every other arch keeps the honest
decline, by measurement rather than by deferral. The publish path
now cites this entry (nightly workflow comment, release-notes
template), so `grep T-0704` on the publish path is empty and the
citation half is closed in the same change.

What the lane measured, one arch at a time, is why the objects do
not follow the binaries. `podvm-interpose` carries x86_64's
`struct stat` field offsets as constants
(`crates/podvm-interpose/src/lib.rs:88-91`: `ST_UID 28`,
`ST_GID 32`, measured under both libcs by
`experiments/105-interpose-ownership.sh`), and an object that wrote
a uid at the wrong offset would corrupt whatever field is there:
the crate refuses any other architecture at compile time rather
than building wrong. The embedded pair's own headers read
`e_machine=0x3e` on every shipped binary (the reporter's
measurement, confirmed by the decline path below), and the
machine-mismatch decline (`interpose.rs:238-246`) is the channel an
off-arch payload takes: the object never reaches a loader that
cannot read it. The decline is pinned by unit test
`another_architecture_is_declined` in the same file; the smoke's
group-5 leg proves digest-named fixtures refuse end to end on the
host arch (honest fixture passes, flipped fixture refuses). Per-arch objects would need
per-arch offset measurements plus a matrix leg executing a payload
under each qemu-user pair; until those exist the artefact keeps
the decline and the release note names the arch, which is exactly
what the Decision already says. The guard is the citation fix plus
the compile-time refusal: a second arch's objects cannot arrive
silently, and no publish path claims T-0704 for them.

**Partial, 2026-09-25.** Issue 58 reopens this entry on the
coverage axis: the pair stays x86_64-only by decision, six of the
seven shipped binaries embed the x86_64 pair and decline the tier
by machine mismatch, and no payload runs off x86_64 at all. Make
the smoke report the embedded objects' `e_machine` beside their
digests, so a wrong-arch embed cannot read green. State the
x86_64-only interposer in `README.md` beside the rung list (done
2026-09-25 in the triage change), not only in the release note.
Either take the per-arch offset measurements and build the pair
per arch, or add the qemu-user payload-execution leg for one arch
(aarch64 first) so at least one non-x86_64 arch proves `run` end
to end. While it stays x86_64-only, name the consequence here:
on chroot-denied non-x86_64 hosts there is no fallback rung.
Fix area is `crates/podvm-interpose`,
`crates/podvm/src/interpose.rs` with `build.rs`,
`scripts/build-interpose.sh` `TARGETS`,
nightly-smoke.sh (now `podvm-smoke`), and `.github/workflows/nightly.yml`.
Risk if wrong is a release reading all-arch supported while six
archs run no payload with no fallback rung. Prove is the entry's
Prove: `readelf -h` on each shipped binary's embedded objects
reads back the binary's own `e_machine`, with the smoke asserting
it. Verified 2026-09-25: all seven `v0.1.0-beta.7` assets
sha256-ok, outer `e_machine` per arch, exactly two embedded
`0x3e` `ET_DYN` objects in each.

**Done 2026-09-26.** The smoke asserts the embed per arch, and
the aarch64 leg proves `run` end to end. `build.rs` records each
embedded object's ELF `e_machine` beside its digest
(`PODVM_INTERPOSE_{GNU,MUSL}_MACHINE`), `version --verbose`
renders the two `interpose-gnu-machine` and
`interpose-musl-machine` lines, and `nightly-smoke.sh` (now `podvm-smoke`) group 6
reports outer beside embedded: on x86_64 legs the embeds must
equal the artefact (a wrong-arch embed fails the smoke), on
other legs the mismatch is printed and expected, never
silent. Lane drive: the release binary reports
`SMOKE-EMBED-MACHINE x86_64-unknown-linux-musl outer=0x3e
gnu=0x3e musl=0x3e` beside groups 1 through 5 green.
`experiments/367-qemu-user-aarch64.sh` exits 0
(`experiments/results/qemu-user-aarch64.txt`): 5 clauses green
on the KVM base (binfmt aarch64 with qemu-aarch64-static, the
pinned 3.20 tag pulled with `--platform linux/arm64`, the
extracted payload reading AArch64, the foreign run exiting 0
with stderr naming the 0xb7-against-0x3e decline, EnteredRung
namespace). The per-arch objects stay unbuilt by the Decision
above; the consequence stands as written (no fallback rung on
chroot-denied non-x86_64 hosts). Issue 26 needs no action: it
closed with the objects decision, and this drive is its
coverage proof on record here.

### T-1601 Unblock the static build under Rust 1.99 `invalid_runtime_symbol_definitions`

Source:      CI run 36894578988, job 110478235939, 2026-10-01;
             `crates/podvm-interpose/src/create_calls.rs` `:83` and `:88`;
             `rust-toolchain.toml`
Category:    interpose
Priority:    P0
Effort:      S
Status:      done

Problem:     Main CI is red on all four jobs. The static build stops at
             `crates/podvm-interpose/src/create_calls.rs:88` with
             `error: invalid definition of the runtime `open` symbol used
             by the standard library`. The lint is deny-by-default, so it
             fires without `-D warnings`. It takes down the static build,
             the interposer tests, and plants 32e and 32f with it.
Premise:     The signature is deliberate and already documented in the
             file: stable Rust cannot define a C-variadic function at
             all (rust-lang/rust#44930), so `open_fixed!` takes a fixed
             third argument rather than `...`. The lint is a true report
             about the shape, not a false positive.
Approach:    Settle it the standing way, per `TODO/RULES.md` section 11:
             take whatever unlocks the task. Attribute the macro body
             `#[allow(invalid_runtime_symbol_definitions)]` beside the
             `#[no_mangle]`, and put `#![allow(unknown_lints)]` at the
             crate root so the attribute stays a no-op rather than a
             warning on a toolchain without the lint. Do NOT pin
             `rust-toolchain.toml`: the operator settled on 2026-10-02
             that build compatibility is not a goal, users take a
             published binary, and nightly is permitted. Do not port the
             signature either; the C shim is larger surgery on an object
             loaded into other processes than the allow costs.
Decision:    Allow the lint at the definition, forward-compatible through
             `unknown_lints`. The measured cost is one attribute and no
             behaviour change; the alternative pins the whole tree off
             stable.
Prove:       `cargo build --release -p podvm-gate --bin podvm-interpose-build`
             exits 0 in the Linux lane; the interposer clippy and test
             steps in `.github/workflows/gate.yml` pass;
             `./target/release/podvm-gate` exits 0; a CI run on the landed
             commit reports the static-build job green and plants 32e
             and 32f caught rather than MISS.

**Done 2026-10-02.** One attribute on the crate root and one on each
`open_fixed!` arm, and the two objects build under `rustc 1.99.0`
([the lane drive](../experiments/results/interpose-1.99-objects.txt),
exit 0): `libpodvm_interpose.so` is 350184 bytes on musl and 330264 on
glibc, each naming its own libc and each exporting exactly the 112
names `interpose.map` declares, against the ceiling the tracked wrapper
declares. The four `gate.yml` lint steps also run clean
on the same 1.99 toolchain: `cargo fmt --all -- --check` 0, the
interpose `fmt` 0, `cargo clippy --workspace --all-targets -- -D
warnings` 0, and `cargo test --workspace` 0 with 38 passing suites and
zero `FAILED` lines. `./target/x86_64-pc-windows-msvc/debug/podvm-gate.exe`
exits 0.

Two corrections to the entry's own reading, both measured.

The attribute went on **both** macro arms, not the one this entry named.
The second arm defines `openat` and `openat64` with the same fixed
third argument and the same `open*` runtime symbol family, so an allow
on one arm alone leaves the other exposed to the identical lint.

**The lint is not a 1.99 arrival.** `rustc -W help` on this host's 1.98.0
lists `invalid-runtime-symbol-definitions` as deny. What cannot be
reproduced here is the *failure*: 1.98's `cargo check` and
`cargo clippy` on this crate both exit 0, and the lint appears to need
the final link to see which symbols are exported, which this Windows host
cannot reach without a Linux C toolchain. So the premise stands as
written about what CI saw, and the "needs 1.99" reading is unsupported
either way; the lane drive above is the measurement that settles it.

Not driven here: `scripts/build-interpose.sh`, which resolves
`target/release/podvm-interpose-build` and finds nothing, because
`.cargo/config.toml` sets the musl build target and the release output
lands under `target/x86_64-unknown-linux-musl/release/`. That is a
defect in the wrapper's path resolution, not in this fix, it predates
this entry, and the lane job calls the same binary directly to get the
same objects. It has no entry of its own yet.

**CI confirms it. Runs 36974640619 and 36975394409, 2026-10-02, the
static-build job is green in both.** Its `interposer, one object per libc`, `build`, `no PT_INTERP`
and `binary size ceiling and breakdown` steps all report success, and
the `fmt, clippy, shell` job's `clippy` and `interposer clippy` steps
report success on the same commit. That is the clause the lane could not
settle: the lint fires at link time, which is exactly where this host
had no toolchain to reach.

Plants 32e and 32f are the one clause still open. They need the plant
harness, which compiles this crate and links it, and the harness was not
run in this session.

### T-1628 Record `162-tar-symlink-modes.sh` as retained, DELETE refuted by VC-2

Source:      `refactor/recon-c.md:175`;
             `refactor/06-entries/PLAN.md:168-170`;
             `refactor/06-entries/PLAN.md:22-27`
Category:    interpose
Priority:    P1
Effort:      S
Status:      done

Problem:     The ledger marks this script DELETE and a live entry names
             it in a proof that is still open. Anyone reading the
             ledger deletes a file the record depends on.
Premise:     DELETE is refuted, not merely doubted: the script is named
             in a live `Prove`, and it is retained as a deployment
             proof at the level the code conventions name.
Approach:     Record the refutation and the line that settles it in the
             interpose entries, so the script's fate is written where a
             reader of that category will find it.
Decision:     Retained as a deployment proof, not a unit test. That is
             the level statement the correction leaves implicit.
Prove:       `grep -n "162-tar-symlink-modes" TODO/interpose.md` names
             the refutation and the live `Prove` that settles it.

**Done.** The record stands on the refutation, not on the ledger's DELETE row.
`refactor/06-entries/PLAN.md:168-170` (VC-2) moves
`experiments/162-tar-symlink-modes.sh` from DELETE to RETAIN, and
`refactor/06-entries/PLAN.md:22-27` is the stated scope of the change, so
`refactor/recon-c.md:175` and
`refactor/06-entries/verdict-ledger.tsv` still carry the pre-correction form
by design. `docs/conventions/code.md:25-30` names the level: a script that
drives a real tarball carrying symlinks through the shipped binary is
deployment proof, so it stays shell. The proof is
`grep -n "162-tar-symlink-modes" TODO/interpose.md`, which matches 9 lines
now, 2 of them above this entry, and exits 0. T-1311's live `Prove` is line
1479.

**DELETE is refuted for `experiments/162-tar-symlink-modes.sh`, and the line
that settles it is this file.** `refactor/06-entries/PLAN.md:168-170` (VC-2)
records the refutation: `TODO/interpose.md:1479` is the `Prove` of T-1311, an
entry with `Status: done`, and it names the script. A script named in a live
`Prove` is not dead, whatever its ledger row says.

**The ledger row is PRE-VC and the correction is POST-VC, so the two disagree
by design.** `refactor/06-entries/PLAN.md:22-27` states the scope: three rows
change under the numbered corrections, and this one of them moves
`162-tar-symlink-modes.sh` from DELETE to RETAIN. `refactor/recon-c.md:175`
still carries the pre-correction ledger row, and
`refactor/06-entries/verdict-ledger.tsv` is that pre-correction form. A reader
who takes the ledger over the section 2 table lands this script in the wrong
wave, which is what the plan's own constraint warns against.

**It is a deployment proof, and `docs/conventions/code.md:25-30` is the level
it belongs to.** That section names three kinds and requires each to be stated:
pure tests for deterministic logic, fault tests for conditions a real service
cannot produce on demand, and integration and deployment proof for the actual
default path. The script drives a real tarball carrying symlinks through the
shipped binary with the plain driver beside it, back to back
(`experiments/162-tar-symlink-modes.sh:9-11`), on a pinned image digest
(`:25`). It is the default path, so it is deployment proof and it stays
shell.

**Deleting it would retire the only end-to-end check of this defect.** T-1311's
own Done record shows what it carries: exit 1 on the pre-fix binary with one
`Cannot change mode` error per link and a green engine control
(`experiments/results/tar-symlink-modes-prefix.txt`), and exit 0 on the fixed
binary. The guard is the unit test `tests::fchmodat_forwards_flags`; the
script is the proof that the unit test's subject is reached through the
product. Neither substitutes for the other, and `code.md:25-30` requires each
kind to be named for what it proves.

### T-1639 Record the six interpose members and their excluded-crate proofs as separate jobs

Source:      `refactor/recon-c.md:186`; `refactor/06-entries/PLAN.md:93-107`
Category:    interpose
Priority:    P1
Effort:      S
Status:      done

Problem:     The interposer is excluded from the workspace, so the
             proofs that run inside other people's processes are not
             part of the workspace test run. One job over the workspace
             cannot cover them.
Premise:     The exclusion is deliberate and the cost is known. The
             proofs need a separate job with their own target and linker.
Approach:     Record the six members and which proof runs where, beside
             the entry that owns the interpose build.
Decision:     Separate jobs, not a flag. The object is loaded into
             processes this project does not control, and a workspace
             run cannot see it.
Prove:       `grep -n "members" TODO/interpose.md` names the six members
             and the job each one's proof runs in.

**Done.** The record stands on the corrected counts. The entry's six is the
plan's word: `refactor/06-entries/PLAN.md:80-84` says nine members,
`grep -c '^    "crates/' Cargo.toml` returns 13 at `Cargo.toml:4-16`, and
there is one excluded crate at `Cargo.toml:24`, not six interpose members and
not six excluded-crate proofs. The separate jobs are real. Four steps in
`gate.yml:127-132`, `:193-196`, `:237-242` and `:260-263` reach the excluded
crate by `--manifest-path`, and `crates/podvm-gate/src/dev.rs:55-69` mirrors
them as `CHECK_STEPS`. The Prove is `grep -n "members" TODO/interpose.md`,
which matches 9 lines now, 1 of them above this entry, and exits 0.

**⛔ The entry's "six" is the refactor plan's word and it does not match the
tree.** `refactor/06-entries/PLAN.md:80-84` counts the workspace at **nine**
members and names them: probe, image, extract, complete, enter, supervise,
windows, ssh, cli. That count is stale. `grep -c '^    "crates/' Cargo.toml`
returns **13** on this tree today, at `Cargo.toml:4-16`. `podvm-interpose` is
excluded at `Cargo.toml:24` under the same `exclude` key the plan cites as
`:20`, and the line moved because the wave-4 and wave-5 crates landed. The
exclusion is the durable fact; both member counts are dated.

**What the six is not.** It is not six interpose members. There is one excluded
crate. It is not six excluded-crate proofs either: the ledger carries ten rows
whose name mentions interpose, spread over `KEEP-SHELL`, `SPLIT`, `DELETE`,
`RUST-TOOL`, and one RUST-TOOL for `scripts/build-interpose.sh`
(`refactor/06-entries/verdict-ledger.tsv:3,4,5,24,25,39,99,100,102,107`).
Neither figure is six, and the source does not support the number this entry's
title carries.

**The separate jobs are real, and they are in `gate.yml` by name.** Four CI
steps reach the excluded crate, and every one of them passes
`--manifest-path crates/podvm-interpose/Cargo.toml` rather than naming the
workspace, because `cargo test --workspace` cannot reach an excluded crate:

| step | where | what it proves |
| --- | --- | --- |
| the objects, one object per libc | `gate.yml:127-132` | both build, one per libc |
| interposer clippy | `gate.yml:193-196` | the excluded crate lints |
| interposer tests | `gate.yml:260-263` | the unit suite, gnu target |
| the objects, in the test job | `gate.yml:237-242` | the objects exist first |

Each names `--manifest-path crates/podvm-interpose/Cargo.toml` rather than the
workspace, and `gate.yml:260-263` carries `RUSTFLAGS` naming the crt-static
flag. `crates/podvm-gate/src/dev.rs:55-69` mirrors the same split as
`CHECK_STEPS`, one step per line, with the same manifest path.

**The two targets are not a preference.** `.cargo/config.toml:7-8` sets
`build.target = x86_64-unknown-linux-musl`, and `:10-11` adds
`target-feature=+crt-static` to it. The excluded crate's tests therefore name
`--target x86_64-unknown-linux-gnu` and `-C target-feature=-crt-static`, the
other side of both settings. `scripts/build-interpose.sh:17-19` records the
consequence: a plain `cargo build --release` lands under
`target/<triple>/release/` and `target/release/` is never written.

**The exclusion is the subject of the check, so the check must stay outside
it.** `refactor/06-entries/T-R005.md:148` states the rule: a crate that checks
whether the interposer builds as a `cdylib` cannot be a workspace member,
because adding it would invert the thing it measures. The check lives at
`crates/podvm-gate/src/libc_interpose.rs`, which its own module comment at
`:3-4` names as a port of `60-interposer-libc.sh`. That script is the subject
of the plan's row at `refactor/recon-c.md:164`, whose id is not a tracked
entry. The cost of the exclusion is that no single job covers the tree, and
the price is paid as four extra steps rather than as a flag.


### T-1640 Record that the interpose export check needs no `rlib` and no new test

Source:      `refactor/recon-c.md:187`;
             `refactor/06-entries/PLAN.md:93-107`
Category:    interpose
Priority:    P1
Effort:      S
Status:      done

Problem:     A crate type that cannot host a test file invites the
             obvious fix, and the obvious fix costs a second artefact on
             every build for a test that already exists.
Premise:     Adding a library target would fix name resolution and add a
             release artefact built for two targets on every run, under
             the byte ceiling. The export check already compares the
             dynamic symbol table against a version script for both
             targets.
Approach:     Record the decision and its cost, so a later session
             reading the failing test does not add the library target.
             One check needs a `cc` invocation and joins the gate binary.
Decision:     No `rlib`. The existing comparison is the check, and the
             plan does not add a second artefact to hold a second copy
             of it.
Prove:       `grep -n "interpose.map" TODO/interpose.md` names the
             export comparison that stands as the check, for both
             targets.

**Done.** No `rlib`, and the decision rests on the existing export check at
`crates/podvm-gate/src/interpose_build/build.rs:186-202`, inside the per-target
loop at `:790-797`. It compares `declared_exports` against `exported_symbols`
for equality at `:508`, and the map declares 112 names. Check B, the
`struct stat` and `struct statx` offsets at `:601-700`, needs `cc` and joins
the same binary. The Prove is `grep -n "interpose.map" TODO/interpose.md`,
which matches 11 lines now, 7 of them above this entry, and exits 0. The
plan's line reference at `refactor/06-entries/PLAN.md:102-105` is stale, so
the citation above is the code.

**No `rlib`, and the failure a later session will meet is a name-resolution
error, not a missing test.** `crates/podvm-interpose/Cargo.toml:15` declares
`crate-type = ["cdylib"]`, and a `cdylib` produces no importable Rust library
for a test target to name. A `tests/*.rs` in that crate fails with
`error[E0433]: cannot find module or crate`. Adding `rlib` to that list is the
obvious repair and it is the wrong one here.

**The export check is already written, and it stands as the check.** It lives in
`crates/podvm-gate/src/interpose_build/build.rs:186-202`, inside the per-target
loop at `:790-797`, so it runs once for every target `TARGETS` names. The
comparison is `declared_exports(&krate.join(MAP))` at `:501` against
`exported_symbols(&so, root)` at `:506`. `declared_exports` parses
`crates/podvm-interpose/interpose.map` (`:242-267`) and `exported_symbols`
runs `nm -D --defined-only` and keeps the `T` fields (`:270-285`). The two
lists are compared for equality at `:508`, not for containment, and the pass
line prints the count (`:510-512`). A name in the map the object does not
export is a silent non-interposition; a name the object exports that the map
does not list is a symbol some other library in the payload's process resolves
to podvm. The comment at `:494-500` states both halves.

**The map declares 112 names, and the check asserts that count against the
object.** `awk '/global:/{g=1;next} /local:/{g=0} g && /;/{n++} END{print n}'
crates/podvm-interpose/interpose.map` prints **112** and exits 0, and that
walk is the one `declared_exports` performs between the `global:` and `local:`
markers. T-1601's Done record reports the same 112 for both objects on the same
comparison, and this figure is measured rather than copied from there.

**⛔ The plan's line reference for this check is stale and names a file that no
longer holds it.** `refactor/06-entries/PLAN.md:102-105` puts the byte ceiling
at line 48 of the script and the export comparison at lines 197 to 214 of the
same file. That script is **41 lines** today and is a declaration stub: its own
header comment at `:2-3` names the plan row for porting it, and the build
itself moved into the `podvm-interpose-build` binary. The ceiling moved with
it and is read at runtime; the stub is where the ceiling is declared, and
`interpose_build.rs:18-20` records that no Rust source repeats the digits
because the gate scans for them. The committed per-libc reading is
`experiments/results/bloat-interpose.txt`, which carries one size per libc.
**A reader who followed the plan's citation would open a 41-line shell stub
and find no comparison in it.**

**The byte cost of `rlib` is bounded by that same ceiling, and the ceiling is
per object.** Both objects embed in the release binary, which the stub's
comment at `:7-9` states, so their growth hides in that binary's headroom. An
`rlib` is a third artefact for two targets and adds to every
`podvm-interpose-build` run and every `gate.yml` invocation. The empty
`[dependencies]` at `crates/podvm-interpose/Cargo.toml:18` means the cost is
not shared resolution; it is a second and third artefact alone.

**One check does need `cc`, and it joins the same binary rather than a new
target.** Check B is `struct stat` and `struct statx` offsets under both
libcs, at `interpose_build.rs:601-700`, reached from `main` at `:798`. It
compiles `OFF_C` (`:571-596`) and needs `cc` or `zig`; when the write fails it
prints `COULD NOT BUILD OR RUN` and returns 2 (`:608-610`), which is the third
state, not a pass. The comment at `:598-600` records that it answers a glibc
payload rather than one libc twice: coreutils' `stat` on a modern glibc asks
`statx(2)` and never reaches `stat`. It is already inside `podvm-gate`, so it
costs no new crate and no new library target.

---

### T-1643 A relative path resolves against the host cwd and gets the rootfs prefix twice

Source:      issues 86 and 131 (131 reposts the hidden issue 105);
             issue 114 section 3.1 and section 6 item 5;
             `crates/podvm-interpose/src/map/rewrite.rs:256`;
             `crates/podvm-interpose/src/stat_calls.rs:407`;
             A-V-04 of PodUtils/podcrates#39
Category:    interpose
Priority:    P1
Effort:      M
Status:      done

Problem:     On the userland rung every relative path fails with
             `ENOENT`. A relative read, a relative write with or without
             `-w`, a dirfd-relative `openat`, and `cd /; echo x > rel.txt`
             all miss the file. `apk info` cannot open its database. A
             tool that writes a relative output fails before it starts.
Premise:     `absolutize` (`map.rs:506`) takes the `AT_FDCWD` base from
             the real `getcwd` (`map.rs:507`). podvm starts the payload
             with its working directory inside the rootfs, so that base
             is a host path under the rootfs. `prepare` (`map.rs:613`)
             gives the joined path to `rewrite` (`map.rs:361`), and the
             `/` pair of the T-1407 table adds the rootfs prefix a second
             time. A descriptor base read from `/proc/self/fd/<n>` has
             the same shape. The interposer's own `getcwd` (`lib.rs:1151`)
             already reverse-maps the real cwd with `unrewrite`
             (`map.rs:457`, called at `lib.rs:1170`). `absolutize` does
             not.
Approach:    In `absolutize`, reverse-map the real cwd and the descriptor
             base with `unrewrite` before the join. Then `rewrite` adds
             the prefix once. Normalise `.`, `..` and `//` before the
             match. Keep the existing refusal where the cwd cannot be
             read.
Decision:    The input to `rewrite` is always a guest path. A base that
             already carries the rootfs prefix is a host path, and the
             interposer reverse-maps it before the join. The kernel cwd
             does not move: issue 86 pass 4 records that a move breaks
             the store paths the interposer uses. A resolution failure
             never falls back to the unmapped relative path.
Prove:       `cargo test --manifest-path crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu`
             exits 0 in the Linux lane with a new unit test: a cwd inside
             the table target absolutizes to one prefix, for `AT_FDCWD`
             and for a descriptor. `sh experiments/356-no-chroot-rung.sh`
             exits 0 with a new clause on the chroot-denying fixture: a
             relative read, a relative write under `-w /tmp`, a
             dirfd-relative `openat` and `cd /; echo x > rel.txt` each
             land inside the rootfs.

**Done 2026-11-09.** `absolutize` (the new `map/absolute.rs`) reverse-maps
the real cwd and a descriptor base with `unrewrite` before the join and
normalises `.`, `..` and `//`, so `rewrite` adds the prefix once. The
interposer tests exit 0 (54 passed), with
`a_cwd_inside_the_target_gets_the_prefix_once` and
`normalise_folds_dots_and_doubled_slashes`. [The drive](../experiments/results/relative-paths-1643.txt)
on the chroot-denying fixture: a relative read under `-w /etc`, a
relative write under `-w /tmp`, a dirfd-relative `openat` and a
relative write after `cd /` each land inside the rootfs. Clause 20 of
experiment 356 holds the four. `sh experiments/356-no-chroot-rung.sh` ran from a scratch copy that takes its five images from `mirror.gcr.io` (Docker Hub and `public.ecr.aws` answered HTTP 429 on 2026-11-09): each of its 64 steps exits as wanted, clauses 20, 21 and 22 among them, and the script exits 1 only on clause 1, because this host enters the namespace rung where the lane enters chroot ([the run](../experiments/results/no-chroot-rung-mirror.txt)).

**Filed 2026-11-06.** The measured facts below are quoted from issue 86
and its review, so the entry stands if GitHub hides the issue. They were
measured on release 0.1.0-beta.12 (sha256 `7ef3cf35...`) on a host that
denies chroot and user namespaces. The line numbers above were read on
this tree. A static probe with no interposer printed the real cwd:

```text
$ podvm run alpine:3.20 /bin/touch RELFILE
touch: RELFILE: No such file or directory
$ podvm run alpine:3.20 /bin/cat /lib/apk/db/installed   # prints the db
$ podvm run alpine:3.20 /bin/cat lib/apk/db/installed
cat: can't open 'lib/apk/db/installed': No such file or directory
$ podvm run python:3-alpine python3 -c \
    'import os; os.chdir("/"); print(os.getcwd()); open("etc/hostname")'
/
FileNotFoundError: [Errno 2] No such file or directory: 'etc/hostname'
static probe: getcwd=/workspace/podvm-ux/store/rootfs/sha256_1667.../rootfs
```

**Amended 2026-11-09.** For T-1643, the ask A-V-04 of PodUtils/podcrates#39 (COORD-015,
a message from a peer session) wants a Node CI job to run from an OCI image
with no root. This entry is one step of that chain: a Node job runs from
relative paths. Its clauses do not change. T-1646 holds the end-to-end
clause.

---

### T-1644 Admit a glibc payload against its whole NEEDED closure, read inside the rootfs

Source:      issues 87, 128 and 129 (128 and 129 repost the hidden
             issues 96 and 97); issue 114 section 3.2 and section 6
             item 4; `crates/podvm/src/interpose.rs:291`;
             `src/abi.rs` of `pod-enter` (PodUtils/podcrates);
             `src/abi/search.rs` of `pod-enter` (PodUtils/podcrates);
             `crates/podvm-interpose/src/real.rs:44`;
             A-V-04 of PodUtils/podcrates#39
Category:    interpose
Priority:    P1
Effort:      M
Status:      done

Problem:     On a chroot-denied host the loader family declined every
             glibc payload in release 0.1.0-beta.12. Each image failed on
             a different provider: bookworm-slim and gcc:13-bookworm on
             `__tls_get_addr`, bullseye-slim on `dlsym`. The no-chroot
             rung then runs musl images only. The beta.10 lane ran
             bookworm-slim at 0 (no-chroot-rung.txt clause 4).
Premise:     The gnu object needs `libdl.so.2`, `libgcc_s.so.1`,
             `libc.so.6` and `ld-linux-x86-64.so.2`. Its `dlsym` is
             pinned to `GLIBC_2.2.5` under `libdl.so.2` (`real.rs:44`,
             T-1312). `__tls_get_addr` is the loader's since glibc 2.34.
             The current tree admits against a union on the production
             path (`interpose.rs:281`, `abi.rs:518`): libc, the
             `libgcc_s.so.1` beside it (`interpose.rs:264`) and the
             interpreter (`interpose.rs:271`). The union omits
             `libdl.so.2` and the rest of `DT_NEEDED`. The interpreter
             path is a host join, and `Elf::read` follows its link on the
             host. Debian's `/lib64/ld-linux-x86-64.so.2` is an absolute
             link, so a host file, or no file, decides the set. The glibc
             arm of `libc_beside` tests `exists()` on a host path
             (`abi.rs:589`); the musl arm already uses `resolve_in`
             (`abi.rs:713`). No run on this tree has measured the three
             images since beta.12.
Approach:    Build the union from the object's whole `DT_NEEDED` closure
             as the rootfs holds it, plus the interpreter. Resolve every
             member, and the glibc libc path, with `resolve_in`. Match
             versions per provider namespace: `GLIBC_*` against libc and
             the loader, `GCC_*` against `libgcc_s.so.1`. Keep the
             `.dynsym`-only rule and the weak-undefined exemption of
             T-0709. Name the provider that would satisfy a missing
             import in the decline.
Decision:    The admission set is the set the loader resolves, read
             inside the rootfs. A host file never decides it. Issue 114
             names a deeper alternative: do not emulate a foreign rootfs
             libc. This entry keeps the loader family and corrects its
             admission; T-1652 owns the static rung.
Prove:       `cargo test -p podvm-enter` and `cargo test -p podvm`
             exit 0 with unit tests: a union read from a fixture closure
             admits an object that imports `__tls_get_addr`, `dlsym` and
             `_Unwind_Resume`, and a loader link that resolves only on
             the host is not read. `sh experiments/356-no-chroot-rung.sh`
             exits 0 with clauses that run
             `public.ecr.aws/debian/debian:bookworm-slim` `echo hi` and
             `gcc --version` on the gcc image at 0, each with the glibc
             object preloaded.

**Done 2026-11-09.** `abi/closure.rs` reads the whole `DT_NEEDED`
closure of the object inside the rootfs, plus the interpreter, with
`resolve_in`, and matches each version against its provider file
(`GLIBC_*` against libc and the loader, `GCC_*` against
`libgcc_s.so.1`). A loader link that resolves only on the host is not
read. `cargo test --workspace` exits 0 (985 passed), with
`a_closure_read_inside_the_image_admits_the_three_imports` and
`a_loader_link_that_resolves_only_on_the_host_is_not_read`. [The drive](../experiments/results/glibc-closure-1644.txt):
`echo hi` from debian:bookworm-slim and `gcc --version` from the gcc
image exit 0 with the glibc object preloaded, with the host loader
replaced by a stand-in. Clause 22 of experiment 356 holds them. `sh experiments/356-no-chroot-rung.sh` ran from a scratch copy that takes its five images from `mirror.gcr.io` (Docker Hub and `public.ecr.aws` answered HTTP 429 on 2026-11-09): each of its 64 steps exits as wanted, clauses 20, 21 and 22 among them, and the script exits 1 only on clause 1, because this host enters the namespace rung where the lane enters chroot ([the run](../experiments/results/no-chroot-rung-mirror.txt)).

**Filed 2026-11-06.** Quoted from issue 87 and its review, measured on
release 0.1.0-beta.12 with the embedded `interpose-gnu` object at file
offset 2353803 (sha256 `6159c1aa...`). No single object admits it:

```text
NEEDED libdl.so.2, libgcc_s.so.1, libc.so.6, ld-linux-x86-64.so.2
UND __tls_get_addr@GLIBC_2.3   # ld-linux defines it since glibc 2.34
UND _Unwind_Resume@GCC_3.0     # libgcc_s defines it
UND dlsym@GLIBC_2.2.5          # libdl before glibc 2.34, libc after
debian:bookworm-slim  -> 126  __tls_get_addr
debian:bullseye-slim  -> 125  dlsym
gcc:13-bookworm       -> 126  __tls_get_addr
system abi against libc.so.6     -> 1, `_Unwind_Resume` at `GCC_3.0`
system abi against ld-linux      -> 1, `dlsym` not defined
system abi against libgcc_s.so.1 -> 1, `dlsym` at `GLIBC_2.2.5`
system abi against libdl.so.2    -> 1, `dlsym` not defined
```

Issue 128 adds that the absolute `lib64/ld-linux-x86-64.so.2` link
decided the result for gcc:13, node:20 and golang:1.22.

**Amended 2026-11-09.** For T-1644, the ask A-V-04 of PodUtils/podcrates#39 (COORD-015,
a message from a peer session) wants a Node CI job to run from an OCI image
with no root. This entry is one step of that chain for a glibc image. Its
clauses do not change. T-1646 holds the end-to-end clause.

---

### T-1645 Intercept `dlopen` and `dlmopen`

Source:      issue 88; issue 114 section 2 item 4 and section 3.2;
             `crates/podvm-interpose/interpose.map:43`;
             `crates/podvm-interpose/src/exec/calls.rs:20`;
             `crates/podvm-interpose/src/real.rs:44`;
             A-V-04 of PodUtils/podcrates#39
Category:    interpose
Priority:    P1
Effort:      M
Status:      done

Problem:     A dynamic payload on the userland rung cannot load a guest
             shared object at run time. `python3 -c 'import subprocess'`
             fails to load `_posixsubprocess`, though the file is in the
             rootfs. CPython C extensions, NSS, gconv, PAM and plugin
             hosts all take this path.
Premise:     The interposer defines no `dlopen` and no `dlmopen`. The
             export map lists the path entry points, `execve` among them
             at `interpose.map:43`, and names neither. The T-0703 record hands both to T-0708
             and the loader, and neither entry defines them. The payload's
             loader opens the guest object by its guest path, and nothing
             rewrites it. The path entry points are macro-generated
             (`lib.rs:1014` is `execve`), and the next definition comes
             through `dlsym(RTLD_NEXT)` (`real.rs`).
Approach:    Define `dlopen` and `dlmopen`. Route a path argument that
             holds a `/` through `map::prepare`, as `open` does, and call
             the next definition. Leave a bare soname unchanged: the
             loader searches its own list, and the loader family points
             that list into the rootfs. `dlmopen` keeps its namespace
             argument. Add both names to `interpose.map`, and move the
             export comparison and the size ceiling with them.
Decision:    `dlopen` is a path entry point like `open`. A bare soname is
             a search, not a path. A load that glibc starts inside libc
             (NSS, gconv) calls the loader directly and does not reach an
             exported `dlopen`; the drive measures that arm and the entry
             records the result, a pass or a named limit.
Prove:       `cargo test --manifest-path crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu`
             exits 0 in the Linux lane with a test that loads a mapped
             object by its guest path. `./target/release/podvm-interpose-build`
             exits 0 with both objects exporting `dlopen` and `dlmopen`.
             `sh experiments/356-no-chroot-rung.sh` exits 0 with a clause
             that runs `python3 -c 'import subprocess'` from the python
             image at 0 on the fixture.

**Done 2026-11-09.** `dl_calls.rs` defines `dlopen` and `dlmopen`: a
name with a `/` goes through the table as `open` does, a bare soname
stays, and `dlmopen` keeps its namespace. Both names are in
`interpose.map`, and both built objects (glibc and musl) export them
(`nm -D`). The interposer tests exit 0 with
`dlopen_loads_a_mapped_object_by_its_guest_path` and
`only_a_slash_name_without_a_token_is_mapped`. [The drive](../experiments/results/dlopen-1645.txt):
`python3 -c 'import subprocess'` from the python image exits 0 on the
fixture. Clause 21 of experiment 356 holds it. `sh experiments/356-no-chroot-rung.sh` ran from a scratch copy that takes its five images from `mirror.gcr.io` (Docker Hub and `public.ecr.aws` answered HTTP 429 on 2026-11-09): each of its 64 steps exits as wanted, clauses 20, 21 and 22 among them, and the script exits 1 only on clause 1, because this host enters the namespace rung where the lane enters chroot ([the run](../experiments/results/no-chroot-rung-mirror.txt)).

**Filed 2026-11-06.** Quoted from issue 88, measured on release
0.1.0-beta.12 with the embedded musl object (file offset 2687787):

```text
$ readelf -Ws interpose-musl.so | grep -i dl
   UND dladdr
   UND dlsym
$ podvm run python:3-alpine python3 -c 'print(1+1)'
2
$ podvm run python:3-alpine python3 -c 'import subprocess'
ImportError: Error loading shared library
  /usr/local/lib/python3.14/lib-dynload/_posixsubprocess.cpython-314-x86_64-linux-musl.so:
  No such file or directory
```

The object is in the rootfs, 26544 bytes, and `open` reads it by the
same path.

**Amended 2026-11-09.** For T-1645, the ask A-V-04 of PodUtils/podcrates#39 (COORD-015,
a message from a peer session) wants a Node CI job to run from an OCI image
with no root. This entry is one step of that chain: Node loads native addons
with `dlopen`. Its clauses do not change. T-1646 holds the end-to-end
clause.

---

### T-1646 A dynamic grandchild re-enters through the host loader

Source:      issues 89 and 127 (127 reposts the hidden issue 95);
             issue 114 section 2 item 2 and section 6 item 3;
             `crates/podvm-interpose/src/exec/calls.rs:20-195`;
             `crates/podvm/src/lifecycle/entry.rs:449`;
             `experiments/356-no-chroot-rung.sh:453-454`;
             A-V-04 of PodUtils/podcrates#39
Category:    interpose
Priority:    P1
Effort:      L
Status:      done

Problem:     On the userland rung `run` runs one dynamic process. A
             dynamic grandchild fails 127: `sh -c 'echo a | /bin/cat'`
             prints `sh: /bin/cat: not found`. A static grandchild runs.
             So a shell pipeline, `make`, the helper scripts of `apk`
             and every compiler driver fail.
Premise:     The exec family (`lib.rs:1014-1020`: `execve`, `execv`,
             `execvp`, `execvpe`, `execveat`, `posix_spawn`,
             `posix_spawnp`) maps the path and calls the real function.
             The kernel then reads the mapped ELF's `PT_INTERP`, for
             example `/lib/ld-musl-x86_64.so.1`, and resolves it on the
             host root. Where the host has no such file, `execve` returns
             `ENOENT`. Where the host has a loader at that path, the child
             runs under the host loader and host libraries, which is the
             libc mixing T-1407 refused at the top level. That second case
             follows from the mechanism and is not measured. The top
             level avoids both through the loader family (`loader_plan`,
             `src/userland.rs` of `pod-enter` (PodUtils/podcrates)). The banner names
             the limit (`lifecycle.rs:1984`), and clause 17 of experiment
             356 pins the 127 as informational, not as a gate.
Approach:    In the exec family, read the head of the mapped file. For
             an ELF with `PT_INTERP`, resolve the interpreter inside the
             rootfs and exec the guest loader with `--library-path` and
             the mapped binary. Keep `argv[0]` and the busybox applet
             name. For a `#!` script, resolve its interpreter inside the
             rootfs the same way. A static ELF keeps the direct exec. The
             interposer has no dependencies
             (`crates/podvm-interpose/Cargo.toml:18`), so it carries its
             own bounded header reader, and a unit test pins it against
             `loader_plan` on one fixture. Promote clause 17 to a gate.
Decision:    A child enters through the same loader as its parent. The
             host loader never runs a guest binary. Where the interpreter
             does not resolve inside the rootfs, the exec fails with
             `ENOENT` and a named line, never a host fallback. Re-arming
             Syscall User Dispatch in a child belongs to T-1652.
Prove:       `cargo test --manifest-path crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu`
             exits 0 in the Linux lane with tests for the ELF, script and
             static arms. `sh experiments/356-no-chroot-rung.sh` exits 0
             with clause 17 as a gate: `sh -c 'echo a | /bin/cat'` and
             `sh -c '/bin/busybox echo grandchild-ok'` exit 0 on the
             fixture, and `gcc /tmp/h.c -o /tmp/h && /tmp/h` runs at 0 on
             the gcc image once T-1644 admits it. The script also exits 0 with a
             Node clause on the same fixture, as an unprivileged user: from the
             node image, `node --version` prints, and `npm ci` then `npm test` of
             a fixture project whose `postinstall` script starts a child exit 0.

**Done 2026-11-10.** For T-1646, the last clause of the partial record below is run. T-1648 in `TODO/enter.md` holds the full run of experiment 356 of 2026-11-10 ([the run](../experiments/results/no-chroot-rung.txt)). The two clauses that the script did not hold ran by hand on the same fixture ([the drive](../experiments/results/grandchild-exec-1646-rest.txt)): `gcc /tmp/h.c -o /tmp/h && /tmp/h` on the gcc image prints `h-ok` at 0 (clause 19 now holds it as `fx-gcc-build`), and as uid 65534 `node --version` and `npm ci` (whose `postinstall` starts a child) then `npm test` exit 0. The routes that T-1646 does not take stay named as gaps in `--strict` (T-1650).

**Partial, 2026-11-09.** The exec family (the new `exec` module of the
interposer) resolves a guest path inside the rootfs one link at a time
and reads the file head with its own bounded reader. A dynamic ELF runs
through the guest loader with `--library-path` (from the new
`PODVM_LIB_PATH`) and `--argv0`; a `#!` script runs its interpreter
resolved inside the rootfs, in the kernel's argument order; a static ELF
keeps the direct exec. An interpreter that does not resolve in the image
fails with `ENOENT`, never on the host. A `stat` or `access` miss retries
once through the walk, so a shell finds bare commands. The objects stay
under the size ceiling with the same 114 exports. The interposer tests
exit 0 (62 passed), and `cargo test --workspace` exits 0. [The drive](../experiments/results/grandchild-exec-1646.txt)
on the chroot-denying fixture: before, debian `sh -c '/bin/ls /'` exited
134 (the host loader ran the guest binary), alpine exited 127 and node
`execSync` failed with ENOENT; after, each of these exits 0, with
`#!/bin/sh -e` and `#!/usr/bin/env sh` scripts, a grandchild's
`/proc/self/exe`, and node:22-alpine `execSync('ls /')`. Clause 17 of
experiment 356 is a gate now, and clause 23 is new. Remaining: `npm ci`
and `npm test` (T-1697: libuv calls `statx` as a raw syscall), the gcc
`/tmp/h` clause, and the full experiment 356 run (`public.ecr.aws`
answered HTTP 429 on 2026-11-09). Not routed: `fexecve` and `execveat`
with flags, a non-x86_64 ELF, and `system()` and `popen()`, which spawn
inside libc past the interposer.

**Filed 2026-11-06.** Quoted from issue 89 and its review, measured on
release 0.1.0-beta.12 on the errand host:

```text
$ podvm run alpine:3.20 /bin/ls /               # top level works
bin dev etc ...
$ podvm run alpine:3.20 /bin/sh -c '/bin/ls /'
sh: /bin/ls: not found                           [exit 127]
$ podvm run alpine:3.20 /bin/sh -c 'echo a | /bin/cat'
sh: /bin/cat: not found                          [exit 127]
$ podvm run alpine:3.20 /bin/sh -c '/tmp/hello' # a static grandchild
STATIC_HELLO pid=17763                           [exit 0]
```

Issue 127 rebuilds the hidden issue 95: the same failure seen from the
child side, plus absolute in-image link targets and `..` that do not
resolve in the root.

**Amended 2026-11-09.** For T-1646, the ask A-V-04 of PodUtils/podcrates#39 (COORD-015,
a message from a peer session) wants a Node CI job to run from an OCI image
with no root. This entry is the last step of that chain: `npm` starts
children. The Prove gains one clause, a Node job from the node image. The
other entries of the chain (T-1643, T-1644, T-1645, T-1647 and T-1649) keep
their own clauses.

### T-1697 A raw `statx` of libuv misses the path table, so npm cannot find its modules

Source:      T-1646 (its drive of 2026-11-09); A-V-04 of
             PodUtils/podcrates#39; issue 114
Category:    interpose
Priority:    P1
Effort:      L
Status:      done

Problem:     On the userland rung, `npm ci` from node:22-alpine fails:
             node cannot `lstat` `/usr/local/lib/node_modules` (ENOENT).
             `readFileSync` of a guest file works, and `statSync` of the
             same file gives ENOENT. libuv calls `statx` as a raw
             syscall, so the preload never sees the path, and the kernel
             resolves it on the host.
Premise:     The interposer maps library calls only
             (`crates/podvm-interpose/src/exec/calls.rs:20` is the
             exec family; the stat family is in
             `crates/podvm-interpose/src/stat_calls.rs:296`). A raw
             syscall passes it. T-1652 plans Syscall User Dispatch for
             static payloads, which also catches a raw syscall of a
             dynamic one.
Approach:    Measure which raw syscalls node and libuv make on the path
             (`statx`, `openat2`, `io_uring` setup). Then choose: catch
             them with Syscall User Dispatch (T-1652) where the kernel
             has it, or turn off the raw path of libuv where it has a
             switch (`UV_USE_IO_URING=0` covers io_uring only), and say
             which in the banner. Refuse by name where neither works.
Decision:    A raw syscall that the table cannot see is a host access.
             The fix catches it or refuses it; it never lets it pass
             quietly.
Prove:       `cargo test --manifest-path crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu`
             exits 0 with a test that a raw `statx` of a guest path
             answers from the rootfs, and a drive on the chroot-denying
             fixture runs `npm ci` and `npm test` of a small project from
             node:22-alpine at 0.

**Done 2026-11-10.** For T-1697, the last clause of the partial record below is run. T-1648 in `TODO/enter.md` holds the full run of experiment 356 of 2026-11-10 ([the run](../experiments/results/no-chroot-rung.txt)).

**Partial, 2026-11-10.** Measured with strace on the chroot-denying
fixture (node 22.23.4, libuv 1.51.0, musl 1.2.6): node imports `syscall`
and no `statx`, and libuv stats through `syscall(__NR_statx, ...)`, so the
kernel got the guest path; io_uring carried no file operation. The
interposer now exports `syscall` (`raw_calls.rs`): 51 path-taking numbers
go to the existing entry points in the kernel's argument order, 25
path-taking numbers with no route answer `ENOSYS` with a line when a table
is set, `io_uring_setup` answers `ENOSYS` (libuv falls back to epoll), and
every other number goes to the kernel unchanged. The objects stay under
the ceiling, with 115 exports. The T-1697 gap leaves the capability list
of T-1650. The interposer tests exit 0 (65 passed) with a raw `statx` of a
guest file that answers from the rootfs; `cargo test --workspace` exits 0.
[The drive](../experiments/results/raw-statx-1697.txt): before, node
`statSync`, `npm --version` and `npm ci && npm test` exited 1; after, each
exits 0, and the earlier clauses stay at 0. Clause 25 of experiment 356
holds it. Remaining: the full experiment 356 run (`public.ecr.aws`
answered HTTP 429). A program that issues the `syscall` instruction
itself still passes the interposer (T-1652). npm now prints a line for
each compile-cache entry (T-1699).

**Filed 2026-11-09.** Found by the drive of T-1646: a probe in node
showed `readFileSync` of a guest file at 0 and `statSync` of the same file
at ENOENT.

### T-1699 `mkostemp` of a mapped template answers ERANGE, and npm prints a line for each cache entry

Source:      T-1697 (its drive of 2026-11-10);
             `crates/podvm-interpose/src/create_calls.rs:339`; T-0705
Category:    interpose
Priority:    P2
Effort:      S
Status:      done

Problem:     On the userland rung, each npm run prints many lines of
             "podvm: interpose: mkostemp: the mapped name does not fit the
             template" (69 for `npm --version`, 648 for `npm ci` and
             `npm test`). npm turns on node's compile cache under
             `/tmp/node-compile-cache`, and node writes each entry with
             `mkostemp`. The mapped template is longer than the guest one,
             so the T-0705 rule answers ERANGE. node skips the entry and
             npm exits 0, but the cache never works and the output is
             noise.
Premise:     The T-0705 rule maps the template and refuses when the mapped
             name does not fit the caller's buffer. With
             `NODE_DISABLE_COMPILE_CACHE=1` the lines stop.
Approach:    Run the real `mk*temp` on a mapped copy of the template, then
             write back only the chosen `XXXXXX` characters into the
             caller's guest template, so the caller sees a guest name of
             its own length.
Decision:    This changes the T-0705 decision: the caller's buffer keeps
             the guest name, and the file lands in the rootfs.
Prove:       `cargo test --manifest-path crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu`
             exits 0 with a test that `mkostemp` of a guest template in a
             deep rootfs creates the file in the rootfs and leaves a guest
             name in the buffer, and `npm --version` on the
             chroot-denying fixture prints no mkostemp line.

**Done 2026-11-10.** `on_template` (`create_calls.rs`) maps the
template into a buffer the interposer owns, runs the real `mk*temp` on
the copy, and writes back only the six chosen characters into the
caller's guest template; `mkstemp`, `mkostemp` and `mkdtemp` use it, and
`mkdtemp` returns the caller's own buffer. A template whose last six bytes
are not `X` answers EINVAL, as libc does. The interposer tests exit 0 (67
passed) with `mkostemp_creates_in_the_rootfs_and_keeps_the_guest_name`
and `mkdtemp_answers_the_guest_template_and_refuses_no_x_run`; both fail
with the old rule. [The drive](../experiments/results/mkostemp-1699.txt)
on the chroot-denying fixture: `npm --version` exits 0 and prints no
mkostemp line (69 before), and node's compile cache holds 69 entries after
one run (0 before); the host's `/tmp/node-compile-cache` does not change.

**Filed 2026-11-10.** Found by the drive of T-1697.

### T-1701 `mkstemp64`, `mkostemp64`, `mkstemps` and `mkostemps` pass the path table

Source:      T-1699 (its review of 2026-11-10);
             `crates/podvm-interpose/src/create_calls.rs:339`
Category:    interpose
Priority:    P2
Effort:      S
Status:      done

Problem:     The interposer exports `mkstemp`, `mkostemp` and `mkdtemp`
             only. A glibc program built with `_FILE_OFFSET_BITS=64` calls
             `mkstemp64` and `mkostemp64`, and a program that needs a
             suffix calls `mkstemps` or `mkostemps`. Each of these
             resolves its template on the host, past the path table.
Premise:     `on_template` already maps a template and writes back the
             chosen characters; the suffix forms put the `X` run before
             the suffix of the given length.
Approach:    Export the four names through `on_template`, with the X run
             found before the suffix for the `s` forms, and add them to
             `interpose.map` with the export comparison and the size
             ceiling.
Decision:    Each libc entry point that takes a path goes through the
             table, or it is named as a gap.
Prove:       `cargo test --manifest-path crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu`
             exits 0 with a test for each of the four names in a deep
             rootfs, and the built objects export them.

**Done 2026-11-10.** `on_template` takes a suffix length, and the macro
`mkstemp_fd!` defines the six descriptor entry points: `mkstemp`,
`mkostemp`, `mkstemp64`, `mkostemp64`, `mkstemps` and `mkostemps`. For the
`s` forms the six `X` sit before the suffix, which stays; a bad suffix
length answers EINVAL with the buffer unchanged, as libc does. The 64
forms look up their own name first, then the plain one. `interpose.map`
names the four, and both objects export 119 names under the size ceiling.
The interposer tests exit 0 (69 passed) with
`mkstemps_keeps_the_suffix_and_the_guest_name` and
`mkstemps_refuses_a_bad_suffix_length`, and the T-1699 test now covers the
64 forms; `nm -D --defined-only` of both objects shows the four names.

**Filed 2026-11-10.** Found by the work on T-1699.

### T-1703 The exec routes that T-1646 does not take still enter through the host

Source:      T-1646 (its Done record); T-1650 (the capability gap);
             `crates/podvm/src/complete/capability.rs:42`
Category:    interpose
Priority:    P2
Effort:      L
Status:      done

Problem:     On the userland rung a child that the payload starts through
             `fexecve`, through `execveat` with flags, or through
             `system()` or `popen()` inside libc, or a child ELF of another
             machine, does not enter through the image's loader: the
             kernel reads its loader or its `#!` interpreter on the host
             root. `--strict` names it as a capability gap and warns,
             because podvm cannot tell from the argv.
Premise:     T-1646 routes `execve`, `execv*`, `execle`, the `p` forms and
             `posix_spawn*` through the image's loader. `system()` and
             `popen()` call `posix_spawn` inside libc, past the preload.
             `fexecve` uses `execveat` with `AT_EMPTY_PATH`, which the
             exec family keeps on the old path.
Approach:    Route `fexecve` and `execveat` with flags by reading the
             descriptor's head as T-1646 reads a path. For `system()` and
             `popen()`, export them and run `/bin/sh` through the routed
             spawn. A foreign ELF stays a refusal by name. The gap leaves
             the list of T-1650 for each route that this entry closes.
Decision:    Each route that runs a payload's child is either routed or
             named as a gap; none passes quietly.
Prove:       `cargo test --manifest-path crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu`
             exits 0 with a test for each route, and a drive on the
             chroot-denying fixture runs a child through `system()` and
             through `fexecve` from the image at 0.

**Done 2026-11-11.** The interposer exports `fexecve`, `system`, `popen`
and `pclose`, and `execveat` takes each flag of the kernel:

* `execveat` with `AT_EMPTY_PATH` and an empty path (the form of
  `fexecve`) reads the guest name of the descriptor's file
  (`fd_guest` in `crates/podvm-interpose/src/map/absolute.rs`) and takes
  the route of that name. A file with no guest name (a `memfd`, a deleted
  file) takes the route of `/proc/self/fd/N`; the descriptor loses
  `FD_CLOEXEC` for the exec, so the guest loader can open it, and gets it
  back where the exec fails (`crates/podvm-interpose/src/exec/descriptor.rs`).
  `AT_SYMLINK_NOFOLLOW` answers `ELOOP` for a link and then routes the
  name. musl exports no `execveat`; there the real call is the system
  call.
* `system` and `popen` start `sh -c` through the routed `posix_spawn`, so
  the image's `/bin/sh` enters through the image's loader. `system` keeps
  the signal rule of POSIX; `pclose` waits for the shell of each stream
  that `popen` opened (`crates/podvm-interpose/src/exec/shell.rs`). With
  no anchored table each one is the real call.
* A child ELF of another machine or class fails its exec with `ENOEXEC`
  and one line that names T-1703: the image's loader cannot run it, and
  the kernel would read its loader on the host root.

The gap of T-1703 left `LOADER_GAPS`
(`crates/podvm/src/complete/capability.rs`); experiment 356, clause 24,
now finds the `--strict` warning of T-1652.

Proof: `cargo test --manifest-path crates/podvm-interpose/Cargo.toml
--target x86_64-unknown-linux-gnu -- --test-threads=1` passed 78 tests,
with `a_descriptor_enters_through_the_guest_loader` (a named file and a
`memfd`), `a_foreign_elf_refuses_and_nofollow_sees_a_link`, and
`system_and_popen_start_the_image_shell` (the running shell's exe is the
inode of the rootfs loader, not of the host loader).
`PODVM_432_MIRROR=1 sh experiments/432-exec-routes.sh` exited 0 on the
chroot-denying fixture (`experiments/results/exec-routes.txt`): a dynamic
musl probe in alpine 3.20 ran `system()` (`sys-42`, status 3),
`popen()` (`popen-42`, `pclose` 0), `fexecve()` of `/bin/busybox` and
`fexecve()` of a `memfd` copy of it, each at 0, on a host with no
`/lib/ld-musl-x86_64.so.1`. `scripts/build-interpose.sh` found 127
exports, exactly what `interpose.map` declares.

**Filed 2026-11-10.** T-1646 closed with these routes named as a gap of
T-1650; the gap needs an open owner, and the test
`each_listed_gap_names_an_entry_that_is_not_done` holds that rule. The
gate run 42771484085 at `36b15ec` failed on it, because dfb93ba closed
T-1646 and left the gap under it.

### T-1704 `realpath` of `/proc/self/exe` names the image's loader, while `readlink` names the program

Source:      issue 229; the podrunner design review (the owner's design
             review of PodUtils/podcrates#47), SPEC section 10.5 and WP-V1, its
             measurement M2 (`03-runner-payload.log`); A-V-04 of
             PodUtils/podcrates#39; T-0413 (the `/proc/self/exe` answers);
             `crates/podvm-interpose/src/path_calls.rs:25`;
             `crates/podvm-interpose/src/path_calls.rs:371`
Category:    interpose
Priority:    P1
Effort:      S
Status:      done

Problem:     On the userland rung, `realpath("/proc/self/exe")` gives the
             guest path of the image's dynamic loader. `readlink` of the
             same name gives the program. A runtime that finds its own
             directory with `realpath` looks for its files beside the
             loader. The .NET apphost of the official runner exits 154:
             "The application to execute does not exist:
             '/usr/lib/x86_64-linux-gnu/Runner.Listener.dll'".
Premise:     The interposer serves `readlink` and `open` of
             `/proc/self/exe` from the guest name of the program
             (`crates/podvm-interpose/src/path_calls.rs:371`,
             `crates/podvm-interpose/src/path_calls.rs:386`). Its
             `realpath` (`crates/podvm-interpose/src/path_calls.rs:25`)
             and `canonicalize_file_name`
             (`crates/podvm-interpose/src/path_calls.rs:74`) call the next
             function and reverse-map the answer. podvm does not rewrite
             `/proc`, so libc follows the kernel link to the loader that
             podvm started. M2 measured the split in one image, 3 runs
             each. That the apphost calls `realpath` is Inferred. The
             interposer exports no fortified `_chk` variant; glibc calls
             its internal function from them, so a fortified caller can go
             past the table (Inferred, not run).
Approach:    In `realpath` and `canonicalize_file_name`, answer an input
             of exactly `/proc/self/exe` from the guest name of T-0413,
             then resolve that name inside the table, so the answer is
             what `readlink` gives after the image's own symlinks. Build a
             test program with `-D_FORTIFY_SOURCE=2` and measure whether
             `__realpath_chk`, `__readlink_chk`, `__readlinkat_chk` or
             `__getcwd_chk` reach libc past the interposer. Export each
             variant that the measurement finds, keep glibc's length
             check, and route it through the same code.
Decision:    `readlink`, `open` and `realpath` of `/proc/self/exe` give
             one answer: the program, never the loader. The answer is a
             guest path.
Prove:       `cargo test --manifest-path crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu`
             exits 0 with a test that `realpath` and `readlink` of
             `/proc/self/exe` agree. `PODVM_356_MIRROR=1 sh
             experiments/356-no-chroot-rung.sh` exits 0 with a new clause:
             in `public.ecr.aws/debian/debian:bookworm-slim`, `realpath
             /proc/self/exe` prints
             `/usr/bin/realpath`, and a fortified test program prints its
             own guest path from `realpath` and from `readlink`.

**Done 2026-11-11.** `realpath` and `canonicalize_file_name` start from
the guest name of the program where their input is the exe link under the
override (`exe_input` in `crates/podvm-interpose/src/path_calls.rs`), and
resolve it through the table, so the answer is the program after the
image's links. The exe link is `/proc/self/exe` and, since this entry,
`/proc/PID/exe` for the process's own pid (`is_pid_exe` in
`crates/podvm-interpose/src/procfs/exe.rs`): GNU `realpath(1)` follows
`/proc/self` to `/proc/PID` and reads `exe` there, so it named the loader
with the self spelling alone. The fortified variants were measured on the
cloud host (glibc 2.39, a program built with `-D_FORTIFY_SOURCE=2` under the
gnu object): `__realpath_chk`, `__readlink_chk`, `__readlinkat_chk` and
`__getcwd_chk` each gave the host path where the plain call gave the guest
path. The interposer now exports the four (`crates/podvm-interpose/src/chk_calls.rs`,
`crates/podvm-interpose/interpose.map`), keeps glibc's length check
(`__chk_fail`, or `abort` where none resolves), and takes the route of the
plain call; the same measurement then gives the guest path for each. The
Prove, on the cloud host, 2026-11-11: `cargo test --manifest-path
crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu --
--test-threads=1` exits 0 (71 tests), with
`realpath_of_self_exe_names_the_program_as_readlink_does` (`realpath`, a
buffer `realpath`, `canonicalize_file_name`, `readlink` and the four `_chk`
variants agree; with `exe_input` disabled the test fails on `realpath`,
which then names the test binary) and `the_own_pid_spelling_is_the_exe`.
`PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh` exits 0 with
clause 28 ([the run](../experiments/results/no-chroot-rung.txt)): in
debian `realpath /proc/self/exe` prints `/usr/bin/realpath`, and the
fortified program from a volume prints `realpath /fort/fort` and
`readlink /fort/fort`.

**Filed 2026-11-10.** From issue 229 and the podrunner design review
(the ruling "podrunner", 2026-11-10): blocker 1 of the official runner on
the userland rung, M2 at `36b15ec`. No file of `crates/podvm-interpose`
changed between `36b15ec` and the filing. A wrong answer of a served call
is P1.

### T-1705 A host path that a loaded object reports gets the rootfs prefix a second time

Source:      issue 230; the podrunner design review (the owner's design
             review of PodUtils/podcrates#47), SPEC section 10.5 and WP-V1, its
             measurement M2 (`03-runner-payload.log`); A-V-04 of
             PodUtils/podcrates#39; T-1643 (the cwd base);
             `crates/podvm-interpose/src/map/rewrite.rs:111`;
             `crates/podvm-interpose/src/map/rewrite.rs:294`
Category:    interpose
Priority:    P1
Effort:      M
Status:      done

Problem:     A runtime that opens a file by a path that the loader gave
             it (`dladdr`, the link map, `/proc/self/maps`) gets a host
             path inside the rootfs. The interposer adds the rootfs prefix
             to it again, and the open fails with `ENOENT`. CoreCLR of the
             official runner fails this way: exit 137, "Failed to create
             CoreCLR, HRESULT: 0x8007045A".
Premise:     M2 strace: the first open of `libcoreclr.so` is correct, and
             a later open carries the rootfs prefix twice. `rewrite`
             (`crates/podvm-interpose/src/map/rewrite.rs:111`) treats each
             absolute input as a guest path. T-1643 reverse-maps a cwd or
             a descriptor base that carries the prefix
             (`crates/podvm-interpose/src/map/rewrite.rs:294`), not an
             absolute input. The interposer has no `dladdr`,
             `dl_iterate_phdr` or `/proc/self/maps` handling. podvm starts
             the image's loader with host library directories, so the link
             map holds host paths (Inferred). That CoreCLR takes the path
             from the loader is Inferred.
Approach:    Measure first with a test program that calls `dladdr` on its
             own function and opens the answer. Then take one of two
             forms and record the choice: (a) in `prepare`, an absolute
             input that lies under the host side of a pair of the table
             (the root or a volume) passes unchanged after the component
             checks; (b) the answers of `dladdr`, `dl_iterate_phdr` and
             `/proc/self/maps` are reverse-mapped to guest paths. Form (a)
             covers each source of such a path at once.
Decision:    A rewrite is idempotent: the interposer does not rewrite a
             second time a path that already names a place under a host
             side of the table. Only such a path passes, so the rule gives
             no reach outside the rootfs and the volumes. The interposer
             stays not a boundary.
Prove:       `cargo test --manifest-path crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu`
             exits 0 with a test that a host path under the root pair does
             not get the prefix again and a guest path still does.
             `PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh`
             exits 0 with a new clause: the `dladdr` test program in a
             glibc image opens its own object at 0.

**Done 2026-11-11, form (a).** Measured first, on the cloud host: a
program that opens the path `dladdr` gives for its own library, started as
podvm starts it (the loader with `--library-path` of a host directory,
under the gnu object and a table `/:ROOT`), got the host path
`ROOT/usr/lib/libx.so` and its `open` failed (exit 1): the rewrite added
the prefix a second time. Form (a) covers each source of such a path
(`dladdr`, the link map, `/proc/self/maps`) at once, so this entry takes
it. `rewrite` returns "no match" for a path under the host side of a pair
(`under_host_side` in `crates/podvm-interpose/src/map/rewrite.rs`), so the
path passes unchanged; a host side of `/` is left out, because each path
is under it. The same program then opens its library (exit 0). The limit,
in the comment of the function: a guest path that names the host source
of a volume under another guest name reaches that source. The Prove, on
the cloud host, 2026-11-11: `cargo test --manifest-path
crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu --
--test-threads=1` exits 0 (72 tests), with
`a_host_path_under_a_host_side_is_not_prefixed_again` (a host path under
the root pair and under a volume passes; a sibling of the rootfs, a guest
path and a path under a volume of `/` still get the prefix).
`PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh` exits 0 with
clause 29 ([the run](../experiments/results/no-chroot-rung.txt)): in an
image of debian with a library in `/usr/lib/x86_64-linux-gnu`, the
program prints its `dladdr` path (a host path under the rootfs, through
the image's `/lib` link) and `open ok` at 0. T-1719 records that
`lib_dirs` has no `/usr/local/lib`, found on the way.

**Filed 2026-11-10.** From issue 230 and the podrunner design review
(the ruling "podrunner", 2026-11-10): blocker 2 of the official runner on
the userland rung, M2 at `36b15ec`. The cause is the T-1643 class for an
absolute input.

### T-1710 A Unix socket path in `connect` and `bind` passes the path table

Source:      issue 235; the podrunner design review (the owner's design
             review of PodUtils/podcrates#47), SPEC sections 7.2, 9.5 and its
             unknown U21; its R2:1.2 (a host socket reached from a held
             payload); T-1652 (a raw `syscall`);
             `crates/podvm-interpose/src/path_calls.rs:25`
Category:    interpose
Priority:    P1
Effort:      M
Status:      done

Problem:     On the userland rung a payload names a Unix socket by a
             guest path. The interposer does not rewrite
             `sockaddr_un.sun_path`, so the kernel resolves the path on the
             host root. A `bind` creates the socket on the host, outside
             the rootfs. A `connect` reaches a host socket by its guest
             name. A `-v` pair does not apply to a socket path, so a
             caller cannot put a host socket at a guest path.
Premise:     `crates/podvm-interpose/src` holds no `connect`, `bind`,
             `sun_path` or `AF_UNIX` code (grep, 2026-11-10); the path
             calls start at `crates/podvm-interpose/src/path_calls.rs:25`.
             The behaviour is Inferred from that absence and not run.
             `sun_path` holds 108 bytes, and a host path under the store
             can be longer. A raw `syscall` goes past the interposer
             (T-1652).
Approach:    Measure the current behaviour on the fixture first. Then
             interpose `connect`, `bind`, `sendto` and `sendmsg` with a
             path address: rewrite the path through the table; where the
             result is longer than `sun_path`, open the parent directory
             with `O_PATH` and use `/proc/self/fd/N/NAME`. Reverse-map the
             answers of `getsockname`, `getpeername`, `accept` and
             `recvfrom`. Leave abstract and unnamed addresses unchanged.
Decision:    A socket path is a path, and the table applies to it as to
             `open`. An abstract name is not a path. The interposer stays
             not a boundary.
Prove:       `cargo test --manifest-path crates/podvm-interpose/Cargo.toml --target x86_64-unknown-linux-gnu`
             exits 0 with tests for the rewrite and the long-path form.
             `PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh`
             exits 0 with a new clause: a test program binds
             `/tmp/t.sock`, the socket is inside the rootfs and not at the
             host `/tmp/t.sock`, a second `exec` connects to it, and with
             `-v HOSTDIR:/run/ctl` a connect to `/run/ctl/s.sock` reaches a
             host listener.

**Done 2026-11-11.** `crates/podvm-interpose/src/sock_calls.rs` exports
`bind`, `connect`, `sendto` and `sendmsg`, which rewrite a path address of
`AF_UNIX` through the table as `open` does; `bind` makes a node, so a
read-only prefix of `PODVM_RO` answers `EROFS` for it. Where the host path
does not fit the 108 bytes of `sun_path` (a store path under the default
store does not), the call opens the parent directory with `O_PATH` and
uses `/proc/self/fd/N/NAME`, then closes the descriptor. A bind through
that long form keeps its guest name in a fixed table of the process
(`crates/podvm-interpose/src/sock_calls/names.rs`), and `getsockname`,
`getpeername`, `accept`, `accept4`, `recvfrom` and `recvmsg` answer the
guest name. An abstract or unnamed address passes unchanged.

⚠ The clause of the Prove ran in experiment 432, not 356: it needs the
probe program that 432 builds with zig cc. The measurement before the
change was not run; the premise stays Inferred from the absence of the
calls.

Proof: `cargo test --manifest-path crates/podvm-interpose/Cargo.toml
--target x86_64-unknown-linux-gnu -- --test-threads=1` passed 81 tests,
with `a_socket_path_binds_in_the_rootfs_and_reads_back_its_guest_name`,
`a_long_host_path_binds_through_the_parent_descriptor` (the long form,
and the guest name from `getsockname`) and
`an_abstract_name_passes_and_a_read_only_bind_answers_erofs`.
`PODVM_432_MIRROR=1 sh experiments/432-exec-routes.sh` exited 0 on the
chroot-denying fixture (`experiments/results/exec-routes.txt`): clause 5
bound `/tmp/tNNN.sock` through the long form, `getsockname` printed
`bound /tmp/tNNN.sock`, a second exec connected by the guest name and
read `sock-hi`, and no socket was at the host path; clause 6 connected
to `/run/ctl/s.sock` through `-v HOSTDIR:/run/ctl` and read `sock-hi`
from a listener of the host.

**Filed 2026-11-10.** From issue 235 and the podrunner design review
(the ruling "podrunner", 2026-11-10), U21: "`curl` under the interposer,
and the visibility of a host socket inside the unit, are Unverified". A
path call that misses the table is P1; the first measurement confirms or
corrects the premise.

### T-1721 An interposer for aarch64: the userland rung runs no arm64 payload

Source:      PodUtils/podrunner#52 (COORD-005 of podrunner, a message from
             a peer session); `crates/podvm-interpose/src/lib.rs:56`;
             `crates/podvm/build.rs`
Category:    interpose
Priority:    P2
Effort:      L
Status:      open

Problem:     On an arm64 host with no root and no user namespace, the
             released `podvm-aarch64` declines each arm64 payload on the
             userland rung: "/bin/sh is ELF machine 0xb7 and the glibc
             object is 0x3e, so the tier cannot serve it". podrunner's
             aarch64 runner cannot run there.
Premise:     The interposer is x86_64 only by design: `lib.rs` refuses to
             compile for another architecture, because its `stat`
             offsets, its system call numbers (`raw_calls`) and its
             `syscall` instruction are x86_64's. `build.rs` embedded the
             two x86_64 objects in each binary, also in `podvm-aarch64`.
             Since 2026-11-11 a binary for another target embeds none and
             says so, so the decline names the missing object, not a
             machine mismatch.
Approach:    Port the interposer to aarch64: the `stat`, `stat64` and
             `statx` offsets of aarch64 under glibc and musl (measured as
             experiment 105 measures x86_64's), the system call numbers of
             the generic table, `svc #0` in `raw_calls::forward`, the ELF
             machine of the exec routes (`EM_AARCH64`), and the loader
             names. Build the two aarch64 objects in
             `scripts/build-interpose.sh` and the release, and embed each
             object only into a binary of its machine. Prove it on a
             hosted `ubuntu-24.04-arm` runner, as `kvm.yml` proves KVM.
Decision:    No binary embeds an object of another machine; the release
             checks each embedded machine against the target.
Prove:       `cargo test --manifest-path crates/podvm-interpose/Cargo.toml --target aarch64-unknown-linux-gnu`
             exits 0 on an arm64 runner, and
             `PODVM_356_MIRROR=1 sh experiments/356-no-chroot-rung.sh`
             exits 0 there with the pinned image of 356 for arm64 on the
             chroot-denied fixture.

**Filed 2026-11-11.** From PodUtils/podrunner#52. The guard of `build.rs`
(an aarch64 binary embeds no x86_64 object) is in place: `cargo check -p
podvm --target aarch64-unknown-linux-musl` warns "the gnu interposer is
x86_64 and this binary is aarch64: it embeds none (TODO/interpose.md
T-1721)".
