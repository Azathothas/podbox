# interpose

`crates/podbox-interpose`. `TOOL.md` section 6.7, milestone M6.

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
Premise:     ⭐ **Measured here.** `experiments/60-interposer-libc.sh` check A
             records that the crate cannot be built as a cdylib at all under the
             workspace's own `+crt-static`:

             ```
             error: cannot produce cdylib for `podbox-interpose` as the target
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
             table. The rule podbox holds is **no re-entry under the lock**.
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


**Done 2026-09-09.** `crates/podbox-interpose/`, and all four constraints are
asserted rather than intended.

⭐ **`interpose.map` is the version script and check A compares the OBJECT with
it**, in both directions and for both libcs: 17 declared, 17 exported, and 0
`rust_eh_personality`. ⛔ Comparing the object with the script rather than
counting exports is what makes it a check: a name in the script that `src/lib.rs`
does not define is a silent non-interposition, and one the object exports that
the script does not list is a symbol some other library in the payload's process
resolves to podbox.

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
             `experiments/60-interposer-libc.sh` to show a musl-linked object
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

             ⭐ **And on 2026-09-08 it was measured a second time, with podbox's
             OWN Rust cdylib rather than the C reference interposer, which is
             what the correction above said could not be done here.**
             `experiments/60-interposer-libc.sh` exits **0** for the first time,
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
             podbox declares the syscalls rather than the libc struct variants
             and avoids that whole class.
Decision:    Two objects, not one with runtime detection. Runtime detection
             cannot change a struct offset the compiler already emitted.
Status note: **no longer blocked, and the musl gap is closed.** The measurement
             this entry waited on is taken by `experiments/80-interposer-abi.sh`.
             ⭐ And podbox's own Rust cdylib IS musl-linked now:
             `scripts/build-interpose.sh` passes `scripts/zig-cc.sh` as the
             linker for that target and asserts the resulting `DT_NEEDED`, so
             the object records `libc.so` where it recorded `libc.so.6` before.
             ⛔ Without the assertion the script exited 0 having produced TWO
             GLIBC OBJECTS, which is the whole requirement inverted and reads as
             success.
             ⭐ **THE EMBEDDING IS RULED AND HALF BUILT, on 2026-09-12.** This
             `Approach` says "embed both", and the two objects are built by a
             script rather than by cargo, so `include_bytes!` in `podbox-cli`
             would make an ordinary `cargo build` fail on a tree where the
             script has not run, which breaks `cargo test --workspace`, the
             acceptance and every contributor's first command.
             ⚠ **A FOURTH SHAPE WAS TAKEN, AND IT IS NOT ONE OF THE THREE THIS
             ENTRY LISTED.** `crates/podbox-cli/build.rs` COPIES what
             `scripts/build-interpose.sh` left behind, and writes an EMPTY file
             where an object is absent. `include_bytes!` always compiles, a
             fresh clone with no zig still builds, and an empty object is read
             at run time as "carries none" so the payload gets
             [T-0706](#t-0706-classify-the-payload-and-decline-with-a-named-reason)'s
             named decline rather than a preload that cannot load.
             ⛔ **The rejected three, and why.** A `build.rs` that RUNS the
             script is a cargo inside a cargo, which waits on a lock its own
             parent holds. ⚠ That hazard was MEASURED rather than assumed:
             `experiments/158-interpose-embedding.sh` ran a nested build in two
             shapes on 2026-09-12 and **both completed**, so it does not fire
             here. It is still refused, because it would fire on somebody
             else's machine and copying cannot. A `build.rs` that refuses is
             loud and still breaks the plain build. The objects committed as
             artefacts are build output in the tree, which
             `docs/conventions/git.md` section 4 forbids.
             ⛔ **AND THE ORDER HAD TO MOVE WITH IT.** `scripts/dev.sh` check
             and the gate workflow both built the binary BEFORE the objects, so
             with this shape they would have embedded two placeholders and
             passed. The interposer step now runs first in both.
             ⚠ **The PLACEMENT was still open until 2026-09-18**, which is this
             `Approach`'s second sentence: the selected object written INSIDE
             the rootfs before the chroot, and `LD_PRELOAD` set to the path the
             payload will see. It landed in `crates/podbox-cli/src/interpose.rs`
             (`place`, called from `apply`); the Done note below carries the
             run.
Prove:       `./experiments/80-interposer-abi.sh` exits 0, and `podbox run --rm public.ecr.aws/docker/library/alpine:3.20 sh -c 'test -f /.podbox/interpose.so && env | grep -q "^LD_PRELOAD=/\.podbox/interpose\.so"'`. ⛔ **The reference was an unqualified tag until 2026-09-12**, which resolves through the engine's shortname aliases to a quota-bearing registry, so this acceptance could not be run under the rule `scripts/common/distro-matrix.sh` states. It names the M5 row for that distribution now. [T-1209](gate.md) owns the other 42 lines with the same defect. ⛔ **The second half read `/proc/self/environ` until 2026-09-18**, and the chroot has no `/proc`, so that line could not pass under podbox: `grep` answered `No such file or directory` while the object sat placed beside it. The environ is read through the payload's own `env` instead.


**Done 2026-09-18.** The placement half landed in `crates/podbox-cli/src/interpose.rs` (`place`, and `apply` calling it): the selected object is written to `/.podbox/interpose.so` inside the rootfs before the chroot, sibling plus rename, and `LD_PRELOAD` names it, podbox's object first where the caller set one. `run`, `exec` and `create` share the one call. `experiments/159-interpose-placement.sh` clause A is this `Prove`, pinned to the M5 digest, and `experiments/results/interpose-placement.txt` carries the run: file present, `LD_PRELOAD` in the payload's `env`, exit 0.

⭐ **The resolver follows absolute links under the rootfs, because the first
shape declined every dynamic payload.** Alpine's `/bin/sh` points at the
absolute `/bin/busybox`, which names the host's file when read from outside
and the image's from inside. The naive read found nothing, declined `sh`,
and the payload still ran, so the tier read as absent on every image whose
shell is an absolute link. `resolve_in` walks the guest path component by
component and splices an absolute target back under the root, the way the
guest kernel would; `..` past the root and a link loop are refused rather
than followed.

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
             position: podbox extracts a rootfs and inspects it from outside.
             Cover the families, not a list of names: `open*`, `stat*`, `exec*`,
             `chdir`, `readdir`, `getcwd`, `readlink`, `realpath`, the xattr
             family, `link*`, `rename*`, `mkdir*`, `mk*temp`, `glob`, `scandir`,
             `ftw`, and the `*at` variants of each.
             ⭐ The `*at` mechanism to copy is
             `references/VHSgunzo__pathmap/tree/path-mapping.c:161-187`
             `absolute_from_dirfd()`: a relative path is made absolute by
             `readlink`ing `/proc/self/fd/<dirfd>`, and `/proc/self/cwd` for
             `AT_FDCWD`, before any matching happens.
             ⚠ **And one defect in it that podbox must not copy.** On a
             `readlink` failure that function does `return path`, handing the
             **unmapped relative path** to the real call with no diagnostic. A
             path that could not be resolved then reaches the real tree
             silently. podbox fails the call with `ENOENT` and writes one line
             to fd 2, because a silent escape from a virtual view is the
             `sandlock` failure mode in another costume.
Decision:    A macro over the families, as
             `references/VHSgunzo__pathmap/tree/path-mapping.c:250-262` does, so
             a new entry point is one line rather than a copied body. Hand-write
             only the ones whose argument shape the macro cannot express, which
             in pathmap is `fchownat` at
             `references/VHSgunzo__pathmap/tree/path-mapping.c:1242-1256`.
Prove:       `./experiments/161-path-rewrite.sh` exits 0 and `nm -D --defined-only crates/podbox-interpose/target/x86_64-unknown-linux-gnu/release/libpodbox_interpose.so | grep -c ' T ' | awk '$1 >= 60'`. ⛔ **The first half named `50-interpose-tier.sh` until 2026-09-18**, which measures pathmap, somebody else's object, and cannot verify this one: the wrong subject with a green exit, the same defect class [T-0709](#t-0709-select-the-interposer-by-dt_needed-and-refuse-on-the-version-predicate) amended its own `Prove` for. `161` drives podbox's object through podbox on both libcs, and `experiments/results/interpose-paths.txt` carries the run.


**Done 2026-09-18.** The object interposes 89 names (`src/map.rs` plus ten
macros and twelve hand-written shapes in `src/lib.rs`): the `open`, `stat`
(already there), `exec`, `chdir`, `opendir`/`scandir`, `readlink`/`realpath`,
`xattr`, `link`, `rename`, `mkdir`, `mkstemp`/`mkostemp`/`mkdtemp`, `access`,
`chmod`, `truncate`, `utime`, `statfs`/`statvfs`, `remove`, `fopen`, `glob`,
`ftw`, `tmpfile` families and every `*at` variant of each, `execveat`
included. Functions taking only descriptors stay out, which is
`experiments/100-interpose-symbols.sh`'s own rule: there is nothing in them
to rewrite. The table
is one variable, `PODBOX_MAPS`, in `FROM:TO[,FROM:TO...]` pairs mirroring
pathmap's syntax; longest match wins at a component boundary, and
`/.podbox/` never rewrites. With no table every call forwards unchanged, so
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
             return the error to the caller. podbox swallows `EPERM` **and**
             `EINVAL`, and records which it swallowed.
             The switch that decides whether to attempt the real call at all is
             `references/salsa-debian__fakeroot/tree/libfakeroot.c:669-678`
             `dont_try_chown()`, a cached read of an environment variable.
             podbox probes once instead of asking the user, and caches the
             verdict the same way.
Decision:    Probe once and cache, rather than fakeroot's environment variable.
             A user who has to know to set a variable has to know the wall
             exists, and the audience is automated.
Prove:       `./experiments/105-interpose-ownership.sh` exits 0, and then `podbox run --rm public.ecr.aws/docker/library/alpine:3.20 sh -c 'chown 0:42 /tmp/f && stat -c %u:%g /tmp/f' | grep -qx '0:42'` once T-0702 places the object

**Done 2026-09-22.** Both items the entry left open landed under their own
entries with driven evidence. The wiring is [T-0702](#t-0702-one-object-per-libc-and-it-must-live-inside-the-rootfs),
done 2026-09-18: `place` writes the object to `/.podbox/interpose.so`
before the chroot and `apply` sets `LD_PRELOAD` to it, driven by
`experiments/159-interpose-placement.sh` clause A with the run in
`experiments/results/interpose-placement.txt`. The identity half is
[T-0711](#t-0711-the-identity-calls-and-podboxs-honesty-rules-point-the-other-way-from-fakeroots),
done 2026-09-19: the operator's answer 3 rules the fork this entry left
open, so the `Approach` sentence asking for success-plus-memo on
`setuid`, `setgid` and `setgroups` is superseded by honest refusal by
default with `--user` turning on the fakeroot behaviour, driven by
`experiments/106-interpose-identity.sh` (14 checks, exit 0). The object
half is re-driven since: `experiments/105-interpose-ownership.sh`
check G runs it end to end through the host memo with
`experiments/results/interpose-ownership.txt` carrying 7 checks, exit 0,
and a guest `podbox run` smoke test answered `0:42`.

**Partial, 2026-09-09.** The object does it and it is measured under both libcs;
what is not in is the wiring that puts the object inside a rootfs `podbox run`
enters, which is
[T-0702](#t-0702-one-object-per-libc-and-it-must-live-inside-the-rootfs), and the
identity half of the `Approach`.

⛔ **THE SIDECAR CANNOT BE THE MEMO, and this `Approach` said it would be.**
`crates/podbox-extract/src/sidecar.rs` writes JSONL **beside the rootfs in the
store**, and this object runs after the `chroot`, where the store does not exist.
It is also JSON, and parsing it would allocate on an interposed path, which
[T-0701](#t-0701-the-cdylib-build-constraints) constraint 2 forbids. The memo is
a fixed-width binary file at `/.podbox/ownership.memo`, **inside** the rootfs,
keyed by `(dev, ino)` rather than by path: a path is renamed, hard-linked and
resolved through symlinks, and the question is about the inode.

⛔ **AND IT MUST CROSS PROCESSES, which rules out a table in memory.** This
entry's own `Prove` is `sh -c 'chown 0:42 /tmp/f && stat -c %u:%g /tmp/f'`, and
those are two `execve`s: a memo in this object's memory would be gone before the
question was asked. fakeroot solves that with a DAEMON, `faked`, which podbox has
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
the runtime podbox targets gives `EINVAL` for an unmapped id.

⛔ **The control comes first and it is the one that could have made every other
arm pass for the wrong reason.** On a machine that CAN chown, the object must
change nothing and write no memo: check C asserts `stat` reports `0:42` because
the KERNEL did it and `/.podbox/ownership.memo` does not exist. An interposer
that reported the caller's intent where the real call would have worked is weaker
than the bare chroot it replaces.

⚠ **What is not in**, and it is named rather than left to be discovered:

- **`setuid`, `setgid` and `setgroups`**, which this `Approach` asks for in one
  sentence. They need a per-PROCESS identity memo that survives an `execve`, and
  an inode-keyed file cannot carry one. fakeroot's answer is an environment
  variable, which this entry's `Decision` rejected for the file question and
  which may be right for this one; it is a fork nobody has ruled on;
- **the wiring**: nothing sets `LD_PRELOAD` yet, so `podbox run` does not load
  this object at all. T-0702 is that entry, and until it lands the `Prove` above
  runs through `docker run -v` rather than through podbox.

Prove, run 2026-09-09:

```
$ ./experiments/105-interpose-ownership.sh
  A: both objects export exactly what interpose.map declares, 17 each,
     and rust_eh_personality is in neither
  B: sizeof=144 dev=0 ino=8 mode=24 uid=28 gid=32, under both libcs
  C: the real chown worked and podbox wrote no memo
  D: the bare chown failed (rc=1) and podbox's answered 0:42     glibc
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
             podbox writes the virtual name when it fits and returns `ERANGE`
             when it does not,
             rather than silently returning the real one: `ERANGE` is a defined
             answer the caller handles and a real path is a leak.
Decision:    `ERANGE` over a silent real path. The buffer case is rare and a
             leaked host path in a `configure` script's output is a build that
             bakes in the wrong prefix.
Prove:       `podbox run --rm -e PODBOX_MAPS=/mapped:$PWD public.ecr.aws/docker/library/alpine:3.20 sh -c 'cd /mapped && test "$(pwd)" = /mapped'`
             (The `-v` spelling the entry was authored with does not exist;
             `-e` carries the same table and the assertion is unchanged.)

**Done 2026-09-22.** `map::unrewrite` mirrors `rewrite` with the sides
swapped (longest TO wins, the `/.podbox/` guard holds, `-2` past the
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
Prove:       `podbox run --rm -v "$(command -v podbox):/podbox:ro" public.ecr.aws/docker/library/alpine:3.20 /podbox --version 2>&1 | grep -q 'interpose: declined'`. ⚠ **That drives row 2 of the table and not row 3**, and the substitution is stated rather than quiet: podbox's own release binary is `x86_64-unknown-linux-musl` under the workspace's `+crt-static`, which [T-0701](#t-0701-the-cdylib-build-constraints) records, so it carries no `PT_INTERP` and is a static payload this tree already builds. ⭐ The command IS the check on that: a payload with a `PT_INTERP` would not be declined and the line would fail. ⛔ **The line named an unqualified Go image until 2026-09-12**, which resolves through the engine's shortname aliases to a quota-bearing registry. ⚠ **Row 3, the Go payload, has no acceptance now**, because every row of `DISTRO_ROWS_M5` is a base distribution and none is a Go image, and inventing a reference is what [T-1209](gate.md) refuses. That entry's `Approach` step 2 is where the row is asked for. ⛔ **The `-v` in that line never ran: `run -v` is a refused `None` row.** `experiments/159-interpose-placement.sh` clause B stages the same static binary with `extract` plus a host copy plus `run` instead, and asserts the decline line and exit 0.


**Done 2026-09-18.** The classifier landed in `crates/podbox-cli/src/interpose.rs` (`classify`, read once per payload, advisory, safe direction on every doubt) and runs at `run`, at `exec` on both paths, and at `create` through the shared `prepare`. Where it says unreachable the tier is declined by name on stderr and the payload still runs. `experiments/results/interpose-placement.txt` carries the run: row 2 declines naming the static payload with exit 0, the dynamic control is preloaded and not declined. ⭐ **Row 3 is proved without an image.** The Go markers outrank a present interpreter by unit test over all four section names, because [T-1209](gate.md) refuses an invented reference and no `DISTRO_ROWS_M5` row is a Go image. That answers [PROGRESS.md](PROGRESS.md)'s open question with the second option: the classification's Go case is proved some other way.

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
             object and memo under `/.podbox/`. The store takes no entry:
             payload paths never address the host store. Match at a component
             boundary so `/proctor` is not excluded by `/proc`. Then accept a
             user list on top of it, in `PODBOX_EXCLUDE_PATH`.
Decision:    Built-in first, user list second, and the built-in entries are not
             removable. A user who removes `/proc/self/fd` from the list gets an
             interposer that cannot resolve `*at` calls, and the failure names
             nothing.
Prove:       `podbox run --rm -e PODBOX_MAPS=/proc:/etc public.ecr.aws/docker/library/alpine:3.20 sh -c 'test -z "$(ls /proc)"'`
             (The `-v` spelling the entry was authored with does not exist;
             `-e` carries the same table. The broad map is the point: under a
             map covering `/proc`, the directory still reads as itself, and
             `/proc` is unmounted in payloads, so a `readlink` proof cannot
             run there.)

**Done 2026-09-22.** The never-rewrite predicate lives in `map.rs`:
`excluded` answers the built-in `/proc` tree first (not removable) and
the colon-separated `PODBOX_EXCLUDE_PATH` list second, consulted from
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
             acceptable is that podbox says so.
Decision:    Strip namespace flags from `clone` rather than failing it. Failing
             it stops shells and build systems that set them speculatively;
             stripping them produces a process that works with less isolation,
             which is the mode already reported.
             ⚠ Do not strip `CLONE_NEWNET` silently: an empty netns is
             `ENETUNREACH` for every connection, which reads as a network outage
             rather than as a stripped flag. Report it.
Prove:       `podbox create --name e1 public.ecr.aws/docker/library/alpine:3.20 sh -c 'mknod /tmp/n c 1 3 && test -f /tmp/n' && podbox start e1 && podbox wait e1 && podbox inspect --format '{{.Interpose.Emulated.mknod}}' e1 | grep -q '^[1-9]'; rc=$?; podbox rm -f e1 >/dev/null 2>&1; exit $rc`

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
(`crates/podbox-cli/src/lifecycle.rs` `ps`). The tally rows use the
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


**Done 2026-09-09.** `crates/podbox-enter/src/abi.rs`, driven by
`podbox system abi <object> <libc>` and asserted by check E of
`experiments/80-interposer-abi.sh`.

⛔ **The `Prove` above was amended, and the reason is that its second half could
not fail.** It read `podbox run --rm alpine:latest true 2>&1 | grep -qv 'symbol
not found'`; with no interposer to preload, no run can produce that string, so
the clause passes on a tree where nothing is implemented. Check E is the
assertion that can go red: the reader's verdict is compared with what the loader
actually did in checks A, B and C, over the same objects and the same pinned
payload libcs.

```
== E. podbox's own reader, against the same four situations
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
the distributions somebody thought of, and `crates/podbox-complete/src/identity.rs`
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
for the check; `podbox system abi` keeps the direct two-file question. The
multiarch search that found that libc is in the same change.

⭐ **A second thing this closed, in the harness rather than in podbox.**
`experiments/80-interposer-abi.sh` exited **2** on this container because
`musl-gcc` is absent from it, so question B and both musl arms of check E could
not be taken. It now falls back to `scripts/zig-cc.sh`, which is the compiler
`.cargo/config.toml` already names for the musl target and which carries the
musl sources it compiles against. The script exits **0** for the first time and
the conditions block says which compiler produced the object, because "the musl
arm ran" means something different depending on it.

---

### T-0710 The ownership memo lives where the payload can edit it, and answers by linear scan

Source:      `crates/podbox-interpose/src/memo.rs`; `experiments/results/interpose-ownership.txt`
Category:    interpose
Priority:    P1
Effort:      L
Status:      done 2026-09-19

Problem:     T-0704's memo makes `chown` answerable on a runtime that refuses
             it, and the shape it took to get there is a append-only log at
             `/.podbox/ownership.memo` **inside the payload's own rootfs**,
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
             `podbox exec` see what the first process wrote and also the
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
             (`references/salsa-debian__fakeroot/tree/libfakeroot.c:875-907`), which podbox cannot copy: there
             is no process to hold it across `podbox exec`, and a daemon is the
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
                it stops being harmless if podbox ever reads the memo to decide
                something on the host, which nothing does today and which this
                entry should forbid in writing.
Decision:    **The memo lives on the host, beside the container record. Ruled by
             the operator on 2026-09-11.** The payload cannot read it, forge it,
             truncate it or delete it, so the forgery question closes instead of
             being argued about each time somebody reads the entry.
             ⛔ **The cost is real and it is in the spawn path, not at `chown`
             time.** `podbox exec` re-enters the chroot, so every entry must be
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
(`podbox-supervise/src/table.rs` `memo_path`, `MEMO_CHILD_FD` 17,
`PODBOX_MEMO_FD`), handed as a descriptor at every spawn (`run` foreground
ephemeral under staging, `create`/`run -d` moved to `containers/<id>/`,
`exec` re-handed, launcher and T-0412 steps sharing it) and refused where not
handed (`interpose::apply` returns `Err`). The interposer reads and writes
through the handed number only (`memo::memo_fd`, seek-to-end record, rewind
lookup); past 4 MiB `lookup` returns `BeyondCeiling` and the `stat` family
answers the real owner marked degraded, never a stale record.
`experiments/105-interpose-ownership.sh` check G drives it end to end with
`experiments/results/interpose-ownership.txt` carrying the run: 7 checks, exit
0 (OLD_STAT=0:42, NEW_STAT=0:0 refusal with the degraded line, 5242944 bytes).
A guest `podbox run` smoke test answered 0:42 through the host memo. Nothing
on the host reads the memo to decide anything.

⚠ **One lane trap this change records.** The 105 harness hands fd 9 and not
17: the old glibc payload's `/bin/sh` is dash, which redirects single-digit
descriptors only (`exec 17<>` answers `17: not found`, measured 2026-09-19).
podbox hands 17 by `dup2` with no shell involved, so production is unaffected.

---

### T-0711 The identity calls, and podbox's honesty rules point the other way from fakeroot's

Source:      `TOOL.md` section 6.7; `references/salsa-debian__fakeroot/tree/libfakeroot.c:669-678`
Category:    interpose
Priority:    P1
Effort:      L
Status:      done

Problem:     A payload that is already uid 0 still calls `setuid`, `setgid` and
             `setgroups`, and on the runtimes podbox targets they fail. A
             package manager that drops to an unprivileged user to run a
             maintainer script, `sshd`, and every build system with a "do not
             run as root" guard all take that path.
             ⛔ **The fork is unruled and the two answers are incompatible.**
             fakeroot answers by LYING: it records the requested id and reports
             it back through `getuid`, so the payload believes it dropped
             privilege. podbox's own honesty rules
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
             this entry exists to stop podbox and its own output disagreeing
             about who the payload is.
             ⚠ The measurement in `Approach` is still owed and still decides the
             SHAPE of the honest refusal: which errno each call returns on a
             real target, and what the common callers do with it. It no longer
             decides the default.
Prove:       `./experiments/106-interpose-identity.sh`, which runs a payload
             that calls `setuid(1000)` and then `getuid()` under the object on
             both libcs, asserts whichever of the three answers the ruling
             picks, and asserts the BANNER says the same thing the call did --
             because the failure this entry exists to prevent is podbox and its
             own output disagreeing about who the payload is.


**Done 2026-09-19.** The ruling's both halves are implemented, and `106`
drives them end to end with `experiments/results/interpose-identity.txt`
carrying the run: 14 checks, exit 0. Without the flag the object calls
through and names the failure with its errno (clause A holds the loaded
report identical to the bare one, musl honest checked against the same
kernel's bare glibc lines in C0); with `--user 1000:100` the setters
answer 0 and every getter answers the record on both libcs (B, C,
including `setreuid(2000,-1)` reporting `2000:1000:1000`, the kernel's
saved-follows-effective rule); `podbox run --user` makes `id` answer the
request on both libcs with the banner naming the faked identity (D);
`--strict` refuses naming `--user` through its Degraded parity row (E);
and a static payload with `--user` is declined by name and still runs (F).

The shape the entry ruled: `run`, `create` and both `exec` paths resolve
`--user` through one call (`crates/podbox-cli/src/lifecycle.rs`
`apply_user`, numeric ids or names from the image's own files, a bare
name taking its primary group from passwd) into `PODBOX_IDENTITY`, which
the object reads per process with no lock and no allocation
(`crates/podbox-interpose/src/identity.rs`), surviving `fork` by copy
and `execve` by re-read. The sixteen identity entry points are declared
in `interpose.map` and asserted against it by `105` check A. What the
entry's `Approach` still names as owed, the errno-by-call measurement
on a real target deciding the honest refusal's shape, stays open as
follow-up work, not as this entry: the default no longer depends on it.

⚠ **Two substitutions this run records.** The job container cannot run a
docker daemon (dockerd fails creating the DOCKER chain, `iptables ...
Permission denied`, measured 2026-09-19), so clauses A, B and C0 run the
victims directly rather than under `docker run -v`: the fake answers the
record with or without a wall, and the control compares bare against
loaded rather than against the kernel, which is why no clause asserts an
absolute honest-failure rc. The musl victim cannot execute on the glibc
job container (its interpreter is the absent musl loader), so clause C
runs it staged inside the alpine rootfs through podbox itself.


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
             between its rows measures neither**, and the fixture podbox would
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
Approach:    podbox's own matrix holds the argument constant across every row and
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
             `crates/podbox-interpose/src/lib.rs` (the interposed symbol set)
Category:    interpose
Priority:    P1
Effort:      M
Status:      done

Problem:     The rocky and rocky-minimal rows of `240` fail under podbox
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
             no socket symbol (`crates/podbox-interpose/src/lib.rs`: chown,
             stat, open, path and identity families only), so socket
             shaping is refuted by reading; a file-path operation in the
             download path (`mkstemp`, `statx`, `openat` handling) is the
             suspect, unconfirmed.
             Correction measured 2026-09-22 on host podman 6.1.2: the
             suspect above is wrong. No file-path operation fails. An
             `LD_PRELOAD` shim that forwards `realpath` untouched
             reproduces the failure with no podbox and no interpose
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
             separate "podbox is missing a fixup" from "this machine
             cannot do it either", and here it names podbox: no sweep-side
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
             `crates/podbox-interpose/src/real.rs:28`
Category:    interpose
Priority:    P1
Effort:      S
Status:      done

Problem:     The nix 2.2.2 closure unpacks under podbox now (T-1311), but no
             nix binary starts: the loader refuses each one with
             `version 'GLIBC_2.34' not found (required by
             /.podbox/interpose.so)`. The closure ships glibc 2.27, and the
             preloaded object binds symbols its libc predates, so the whole
             T-1111 pipeline stops at REGISTER.
Premise:     Measured 2026-09-21 in the lane: `readelf -VW` on the shipped
             glibc object needs `GLIBC_2.34` for `dlsym` and `GLIBC_2.30`
             for `gettid`, and nothing else above 2.27. The `dlsym` reference
             is this object's own (`crates/podbox-interpose/src/real.rs:28`);
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
Prove:       `./scripts/build-interpose.sh` exits 0 with the ceiling
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
             `crates/podbox-interpose/src/lib.rs:334`,
             `crates/podbox-interpose/src/lib.rs:989`
Category:    interpose
Priority:    P1
Effort:      S
Status:      done

Problem:     Unpacking a tarball that carries symlinks fails under podbox
             while the engine control unpacks it cleanly. GNU tar reports
             `tar: ./l: Cannot change mode to rwxrwxrwx: No such file or
             directory` for each link and exits 2, so the nix 2.2.2 binary
             tarball (T-1111) does not unpack. Plain `chmod` on the same
             links succeeds, and regular files extract with their modes
             intact: only the mode-set on symlinks fails.
Premise:     Measured 2026-09-21 on host podman, same image, back to back:
             under `podbox run` the minimal tar-symlink repro exits 2 with
             one such error per link; in the plain driver container the same
             commands exit 0. The interposed `fchmodat` is declared with
             three arguments (`crates/podbox-interpose/src/lib.rs:334`) and
             forwards three (`crates/podbox-interpose/src/lib.rs:989`),
             while libc takes four `(dirfd, path, mode, flags)`: the real
             call receives a fourth register the wrapper never set, so any
             caller passing flags gets the wrong answer. Tar's symlink
             mode-set passes `AT_SYMLINK_NOFOLLOW`. Every other
             flags-taking `*at` wrapper carries its flags (`unlinkat`,
             `faccessat`, `utimensat`, `linkat`, `execveat`, `openat2`,
             `fstatat`), so this is the one truncation, not the pattern.
             An empty path table returns the caller's pointer before any
             resolution (`crates/podbox-interpose/src/map.rs:448`), which
             refutes the rewrite as the cause on runs with no table set.
             The ownership memo never sees `chmod`, which refutes that
             layer.
Approach:    Carry `flags` through the declaration and the wrapper. Add a
             unit test that compares the interposed `fchmodat` against
             libc's own for a symlink with `AT_SYMLINK_NOFOLLOW` and with
             flags 0. Drive the tar-symlink repro under podbox with the
             engine control beside it. Out of scope: the T-1309 libdnf
             failure (same family, a different call, still open), the
             T-0705 reverse mapping, and the non-native `$BIN version`
             line of 152 (T-1111's area, cosmetic).
Decision:    Fix in the interposer, not around it. The control names
             podbox, so no script-side workaround is legitimate: stripping
             symlink modes or passing tar permission-dodging flags would
             test a different product, the same ruling T-1309 carries.
Prove:       `./experiments/162-tar-symlink-modes.sh` exits 0 on host
             podman, with the conditions block naming the driver.

**Done 2026-09-21.** The declaration and the wrapper carry `flags`
(`crates/podbox-interpose/src/lib.rs:334`,
`crates/podbox-interpose/src/lib.rs:989`); every other flags-taking
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
