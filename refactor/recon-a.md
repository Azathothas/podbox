# Recon A — host feasibility of the shell-retirement plan

Scope: `refactor/06-entries/PLAN.md` and `T-R000.md` .. `T-R005.md`.
Question answered: for each work unit the six entries name, how much is
native-rust-on-Windows and how much needs a Linux lane.

Host facts were measured in this session, not taken from the plan. Every row
below carries the command that settles it. No repository file was edited.

**Date of measurement:** 2026-10-01. Git state at start and end:
`git status --porcelain` returned exactly `?? refactor/` both times.

---

## 1. The lane, as measured

Two facts decide almost every row in this report. Both were measured here.

### 1.1 The podbox source does not build for Windows at all

```
cargo check --offline -p podbox-probe --target x86_64-unknown-linux-gnu
  -> exit 0, 1.54 s
cargo check --offline -p podbox-probe --target x86_64-pc-windows-gnu
  -> exit 101, 5 errors
     error[E0433]: cannot find `unix` in `os`      (x2)
     error[E0599]: no method named `as_bytes` found for struct `OsString`
     error[E0599]: no method named `pre_exec` found for mutable reference `&mut Command`
```

This is a **typecheck** failure, not a link failure. No linker is involved, so
"give me a `cc`" does not move it. The library is Unix-only at the type level.
`crates/podbox-probe/src/sys.rs` is the reference site.

Unix-only source files, `crates/<c>/src` only, counted with
`grep -rlE 'std::os::unix|libc::|nix::|pre_exec|flock\(|syscall\(' <c>/src | wc -l`
over `find <c>/src -name '*.rs' | wc -l`:

| crate | Unix-only / total src files |
| --- | --- |
| podbox-enter | 8/10 |
| podbox-ssh | 8/14 |
| podbox-cli | 6/30 |
| podbox-image | 6/19 |
| podbox-complete | 5/8 |
| podbox-probe | 2/17 |
| podbox-extract | 2/9 |
| podbox-windows | 2/7 |
| podbox-supervise | 1/3 |

Eight of nine crates. `podbox-supervise` at 1/3 is the only crate where a single
`mod` could gate the whole thing, and its one Unix file is
`src/launcher.rs`, which every other crate depends on transitively.

### 1.2 `cargo check` for a **Linux** target works natively on Windows

This is the finding the plan never states and it changes what parallel agents can
do.

```
$ CARGO_TARGET_DIR=/tmp/recon-t1 cargo check --offline -p podbox-probe --target x86_64-unknown-linux-gnu
   Compiling syscalls v0.8.1
   Checking linux-raw-sys v0.12.1
   Checking podbox-probe v0.1.0-beta.12
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.54s
rc=0
```

Native Windows, rustc host `x86_64-pc-windows-msvc`, target
`x86_64-unknown-linux-gnu`, cross typecheck of the Unix-only library, exit 0.
`rustup target list --installed` shows the std is present for
`x86_64-unknown-linux-gnu` and `x86_64-unknown-linux-musl`.

**Where it stops, measured per crate, `cargo check --offline --tests --target x86_64-unknown-linux-gnu -p <c>`:**

| crate | rc | first error |
| --- | --- | --- |
| podbox-probe | 0 | — |
| podbox-enter | 0 | — |
| podbox-windows | 0 | — |
| podbox-extract | 101 | `error: failed to run custom build command for 'ring v0.17.14'` |
| podbox-complete | 101 | same |
| podbox-supervise | 101 | same |

`ring` compiles C. `.cargo/config.toml:63` sets `CC_s390x_unknown_linux_gnu` and
friends but **sets nothing for `x86_64-unknown-linux-gnu`**; cc-rs therefore
falls back to `x86_64-linux-gnu-gcc`, which does not exist on this host:

```
cargo check --offline --workspace --target x86_64-unknown-linux-gnu
  -> exit 101
     error occurred in cc-rs: failed to find tool "x86_64-linux-gnu-gcc": program not found
```

And the project's own `scripts/zig-cc.sh` does not work there either. On Windows,
`cc-rs` tries to exec it directly:

```
cargo check --offline --workspace --target x86_64-unknown-linux-musl
  -> exit 101
     error occurred in cc-rs: command `"...\scripts/zig-cc.sh" ... ` failed to start:
     %1 is not a valid Win32 application. (os error 193)
```

`zig version` is 0.16.0 (`/c/ProgramData/scoop/shims/zig`), so the tool exists;
the wrapper is a `#!/usr/bin/env bash` script and Windows cannot exec it.

**So: native Windows can typecheck a Linux target for the three crates that do
not reach `ring`, and for nothing else.**

---

## 2. Lane state, measured

### 2.1 `wsl-toolkit` (`run-in-base.sh`) is NOT usable right now

```
wsl-toolkit version                       -> 6.0.0
wsl-toolkit --instance podbox base status --probe
```

verbatim:

```
  instance podbox: distribution wsl-toolkit-podbox, state C:\Users\AjamX\AppData\Local\wsl-toolkit\instances\podbox
  image       ghcr.io/pkgforge-dev/archlinux:latest
  automount   ro, and the guest's drives read ro
  toolset     none
  running     false
  disk        11.4 GiB  ...\base\ext4.vhdx
  usable      false
  cgroup      not measured by this guest
  binfmt      not measured by this guest
  ! a container did not run as toolkit (exit 125): Error: current system boot ID differs from
    cached boot ID; an unhandled reboot has occurred. Please delete directories
    "/tmp/wsl-toolkit-run-1000/containers" and "/tmp/wsl-toolkit-run-1000/libpod/tmp" and
    re-run Podman

  ⛔ STALE-RUN-STATE
     what   the engine's cached boot id is not this boot's, so every container is refused before it starts
     costs  nothing runs. Re-provisioning does not clear it, because the state is the engine's and not the distribution's
     RUN    wsl-toolkit base ensure --repair
```

The lane cannot run one container. The repair it names starts the Podman machine:

```
podman machine list
  podman-machine-default  wsl  5 weeks ago  18 hours ago  10  2GiB  100GiB   (stopped)
podman info
  Cannot connect to Podman ... try `podman machine start`
```

`docs/containers.md:53` requires recording host engine state before starting it
and restoring it at session end. Starting a VM and repairing the base is an
operator action, not something a recon does silently. It is cheap (an 11.4 GiB
vhdx, already provisioned) but it is the operator's call, and it is the price of
admission for the entire plan.

Separately, four kept job records are already turning the host record gate red:

```
py scripts/check-todo.py    -> rc=1, "check-todo: 4 problem(s)"
  wsl-toolkit: lane job still kept: /home/toolkit/.wsl-toolkit/jobs/b3969df4fcaf50e2
  wsl-toolkit: lane job still kept: ...\jobs\3cdaf199fcd81079
  wsl-toolkit: lane job still kept: ...\jobs\503d623307d1d2a9
  wsl-toolkit: lane job still kept: ...\jobs\b3969df4fcaf50e2
```

So `T-R000`'s `Prove` (`py scripts/check-todo.py` exits 0) **does not pass today,
before any wave-0 edit is made.** Wave 0's proof is blocked by lane debt from a
previous session, not by the work.

### 2.2 `wslc` 3.0.1.0 — what was confirmed, and the wedge

Confirmed by direct measurement:

```
MSYS_NO_PATHCONV=1 "/c/Program Files/WSL/wslc.exe" info
  WSL version: 3.0.1.0 ; Session manager version: 3.0.1
MSYS_NO_PATHCONV=1 "/c/Program Files/WSL/wslc.exe" run --rm \
  -v "C:/Users/AjamX/Downloads/podbox:/work" -w /work \
  docker.io/library/rust:1.98.1-bookworm sh -c '...'
  -> exit 0, 19.4 s cold
```

Inside that container, measured: Debian 12 bookworm; `cc` = gcc 12.2.0;
**`id -u` = 0**; `nproc` = 20; `python3`, `readelf`, `nm`, `git`, `file`, `xz`,
`sha256sum`, `bash`, `cargo 1.98.1`, `rustc 1.98.1`, targets
`x86_64-unknown-linux-gnu` and `x86_64-unknown-linux-musl` present.
**`jq` MISSING, `openssh` MISSING, `zig` MISSING, `cargo-bloat` MISSING.**

Bind-mount semantics confirmed, including the write path:

```
inside:  mkdir -p /work/.tmp/recon-a && echo probe > /work/.tmp/recon-a/probe.txt  -> wrote
host:    ls -l .tmp/recon-a/probe.txt                                             -> 6 bytes
         git status --porcelain                                                   -> only ?? refactor/
         git check-ignore -v .tmp                                                 -> .gitignore:28
```

`.tmp/` is gitignored, so that probe did not dirty the tree. The bind mount does
write into the real checkout; section 5 is about whether that is the right
property.

**Then the lane wedged, and it has not recovered.** Sequence, all after the
first four successful runs:

| command | timeout | outcome |
| --- | --- | --- |
| `wslc run … sh -c 'sh scripts/common/bootstrap-env.sh --check …'` | 200 s | rc=124 |
| `wslc run … sh -c 'echo start; id -u; ls /work'` | 150 s | rc=124 |
| `wslc run … sh -c 'echo recovered; id -u'` | 240 s | rc=124 |
| `wslc create --name recon-a-probe … sh -c 'sleep 300'` | 160 s | returned, `wslc list` shows nothing |
| `wslc images` | 90 s | rc=124 |
| `wslc images` (unbounded) | 240 s | killed |
| `wslc run … sh -c 'echo alive; id -u'` (no volume) | 120 s | rc=124 |

Controls that still answer while `run`/`images` hang:

```
wslc info   -> WSL version: 3.0.1.0, Sessions: 1     (60 s, rc=0)
wslc list   -> empty header                            (60 s, rc=0)
tasklist | grep -iE 'wslc|wsl'
  vmmemwslc-cli-AjamX  12680  1,403,152 K
  wslcsession.exe      709880
  wslservice.exe       709808
  vmmemWSL             775792  1,274,320 K
```

**Conclusion, stated as the evidence supports it:** `wslc` worked for four runs
and then stopped starting containers while its read-only metadata commands kept
answering. That is a wedged image/session layer, not a mount problem (a mount
problem would fail fast with an error). Its cause is not established here.
What would settle it: after a full Windows reboot, re-run
`wslc info && wslc run --rm docker.io/library/rust:1.98.1-bookworm sh -c 'id -u'`.
Until then, `wslc` cannot be counted on as a lane. **Neither lane is currently
green.** The plan's "run it on the Linux base lane" is, right now, an aspiration
in both directions.

---

## 3. The three-way separation: authoring / compiling / executing

This is the part the plan never asks. Each work unit has three independent
questions and they have three different answers on this host.

| activity | native Windows | proof |
| --- | --- | --- |
| **authoring** — writing Rust or markdown | ALWAYS possible | `Write`/`Edit`; no toolchain |
| **compiling (typecheck)** | `cargo check` native: no. `cargo check --target x86_64-unknown-linux-gnu` native: yes for probe, enter, windows only | §1.1, §1.2 |
| **compiling (link / build script)** | no. `ring`'s C step has no Windows-usable `CC_*` for `linux-gnu`, and `scripts/zig-cc.sh` is `os error 193` | §1.2 |
| **executing the test binary** | no. The binary is ELF; it cannot be built for Windows and cannot run on Windows | §1.1 |

**The rule this gives:** *a pure unit test can be WRITTEN natively on Windows
and PROVEN only on a Linux lane.* Everything in waves 0, 1, 2 and 5 that is
"pure" is authorable today by a Windows agent with no lane at all, and is
provable only after the lane is green. Authoring in parallel is worth doing
precisely because the proving is the bottleneck and the authoring is not.

### Per-wave compile reachability, measured

| crate containing wave-2 targets | `cargo check --tests --target x86_64-unknown-linux-gnu` (native) |
| --- | --- |
| podbox-probe | exit 0 |
| podbox-complete | exit 101, `ring` |
| podbox-enter | exit 0 |
| podbox-interpose | excluded from workspace; separate manifest, own target |
| podbox-cli | exit 101, `ring` |
| podbox-image | exit 101, `ring` (pulls ureq/rustls) |
| podbox-extract | exit 101, `ring` |

So **3 of the 7 crates holding wave-2 test targets are typecheckable natively**;
the rest need a lane before a single `cargo check` of a new test can run.

---

## 4. Findings table

`host` values: `native-windows` / `linux-lane` / `both-required` / `blocked`.

### Wave 0 — 14 record corrections (`T-R000.md`)

| id | unit | host | why | exact proof command |
| --- | --- | --- | --- | --- |
| W0-01 | `TODO/cli.md:63` parity 220 -> 265; mark `parity-drive.txt:7` (164) and `cli.md:683-685` (160) historical | native-windows | text edit + read; `sed -n '232,537p' crates/podbox-cli/src/parity.rs \| grep -c 'Row {'` returns **265** (verified) | `git diff TODO/cli.md` then `py scripts/check-todo.py` |
| W0-02 | `TODO/cli.md:182` `clause-1 41/41` -> `40/40` | native-windows | text edit only | `py scripts/check-todo.py` |
| W0-03 | `TODO/cli.md:1246` nine ASCII tests -> ten | native-windows | count edit | `py scripts/check-todo.py` |
| W0-04 | `TODO/gate.md:1431` drop 0.936 and 1.869; keep `0.935` at `perf-kvm.txt:133` | native-windows | text; `sed -n '133p' experiments/results/perf-kvm.txt` reads `... kvm guest.kvm.boot kvm 0.935 s wall ok` (verified) | `py scripts/check-todo.py` |
| W0-05 | `TODO/probe.md:171` ENOSYS -> `FAIL errno=3 ESRCH` at `attribute.txt:6`, name commit `6eb941f` | native-windows | text; `sed -n '6p' experiments/results/attribute.txt` reads `kcmp(-1,-1,...) [control] FAIL errno=3 ESRCH` (verified) | `py scripts/check-todo.py` |
| W0-06 | `TODO/interpose.md:87`, `:1445-1446` stale `lib.rs` lines | native-windows | re-read the file, edit numbers | `py scripts/check-todo.py` |
| W0-07 | `TODO/image.md` T-0211 Done re-anchor after W0-08 | native-windows | depends on W0-08 only | `py scripts/check-todo.py` |
| W0-08 | `157-lock-inheritance-prove.sh:157` sed re-anchor, then confirm a real mutation lands | native-windows | it is a shell `sed` edit plus `git hash-object`; the anchor is wrong — `:157` reads `s/if !sys::close_in_children\(lock\.fd\) \{/if !true \{/` while `store.rs:1083` reads `if !sys::close_in_children(fd) {` (both verified) | `sh experiments/157-lock-inheritance-prove.sh` under Git Bash, then `py scripts/check-todo.py` |
| W0-09 | `TODO/gate.md:434` stale `Source` line | native-windows | text | `py scripts/check-todo.py` |
| W0-10 | `experiments/README.md:35` `392-kvm-guest.sh` row overstated | native-windows | text; the row claims a passing proof that `TODO/PROGRESS.md` records as failed | `py scripts/check-todo.py` |
| W0-11 | `371-…:193` writes `windows-365.txt`, `370-…:205` writes `windows-364.txt` — wrong result names | native-windows | text in two scripts, or record the collision | `py scripts/check-todo.py` |
| W0-12 | `TODO/gate.md:964-968` six scripts "each exit 0" vs `EXIT:2` at `:1037` | native-windows | text | `py scripts/check-todo.py` |
| W0-13 | `TODO/image.md:1884,1894` "24 of 24" -> 30 | native-windows | `grep -c '#\[test\]' crates/podbox-image/src/store.rs` returns **30** (verified) | `py scripts/check-todo.py` |
| W0-14 | `experiments/lib/engine.sh` `ENG_NETWORK` — no correction, recorded | native-windows | confirmed present; nothing to do | `py scripts/check-todo.py` |

**Wave 0 rollup: 14/14 `native-windows`.** Not one of the 14 needs a compiler.

**But the wave's own `Prove` does not pass.** `py scripts/check-todo.py` exits
**1** today with four `wsl-toolkit: lane job still kept` problems. That is lane
debt, and `wsl-toolkit --instance podbox gc --job <id> --apply` is the named
remedy — which needs the base, which is broken. So W0-01..W0-14 are authorable
now and the closure condition is blocked on §2.1.

### Wave 1 — delete three scripts, repoint records (`T-R001.md`)

| id | unit | host | why | exact proof command |
| --- | --- | --- | --- | --- |
| W1-01 | delete `experiments/388-interactive-shell.sh`; repoint `TODO/podssh.md:79` | native-windows | deletion + text; the 12 tests at `crates/podbox-ssh/tests/session_interactive.rs:255-482` exist (`grep -c '#\[test\]'` = **12**, verified) and none of them names `388` (`grep -c 388` = **0**, verified) — the plan's own ⛔ is right | `git grep -n '388-interactive-shell' TODO/` then `py scripts/check-todo.py` |
| W1-02 | delete `experiments/220-extract-path-safety.sh`; repair **six** `Prove`/citation sites | native-windows | `grep -rn '220-extract-path-safety' TODO/` must return every hit before the change; `drive.rs` holds 22 `#[test]` (verified) | same |
| W1-03 | `cargo test --workspace` — the wave's `Prove` | linux-lane | no native build exists; §1.1 | `sh scripts/windows/run-in-base.sh` |

**Wave 1 rollup: 2/3 `native-windows`, 1/3 `linux-lane`.** Note the split the plan
misses: **the deletion and every repoint are native work**; only the green-suite
proof needs the lane. A Windows agent can complete W1-01 and W1-02 and leave the
proof for one Linux run.

### Wave 2 — 15 clauses + 3 more (`T-R002.md`)

| id | unit | target | host | why | exact proof command |
| --- | --- | --- | --- | --- | --- |
| W2-01 | `160` `rmi` on a held image exits non-zero | `podbox-cli/src/images.rs` | both-required | authoring native; crate reaches `ring` so no native `cargo check`; execution is Linux | `cargo test --workspace` in lane |
| W2-02 | `160` `image prune` exits 0, prints `skipped:` | `podbox-cli/src/images.rs` | both-required | same | same |
| W2-03 | `80` three exit codes of `system::abi` | `podbox-cli/src/system.rs:604` | both-required | `fn abi` is at `:157`, its `mod tests` at `:604` (verified; the entry cites `:191-214` and `:603`, both off by one) | same |
| W2-04 | `80` check B, `struct stat`/`statx` offsets under both libcs | stays shell | linux-lane | four `LD_PRELOAD` pairings against live loaders; needs a Linux loader | `sh experiments/80-interposer-abi.sh` |
| W2-05 | `90` check D, NSS dispatcher path | `podbox-enter/src/abi.rs` | both-required | **typecheckable natively** (`cargo check --tests --target x86_64-unknown-linux-gnu -p podbox-enter` -> 0); execution still Linux | same |
| W2-06 | `90` check A live form | `podbox-complete/src/identity.rs:336` | both-required | crate reaches `ring`; integration by the entry's own label | same |
| W2-07 | `90` check C **deleted** | n/a | native-windows | deleting a clause is a record edit | `py scripts/check-todo.py` |
| W2-08 | `149` clause 6, guest user networking | `podbox-probe/src/nongoals.rs:253-302`, `report.rs:1069` | blocked | shells to `experiments/361-guest-usernet.sh` with a 1500 s bound; no Rust arm and no fixture exist | `sh experiments/361-guest-usernet.sh` in lane; fixture must be written first |
| W2-09 | `170` clause 1, two `probe --json` runs | `podbox-image/src/probe_cache.rs:233-296` | both-required | crate reaches `ring`; needs `jq`, **MISSING** in the measured container | `cargo test --workspace` in lane |
| W2-10 | `170` clause 3 engine half | stays shell | linux-lane | live engine | engine lane |
| W2-11 | `70` check B, real `alpine` layer | wave 3 fixture | blocked | fixture does not exist | after W3-FIXTURE |
| W2-12 | `70` check A, `WANT` overlay predicate | `podbox-extract/src/drive.rs` | both-required | crate reaches `ring` | `cargo test --workspace` in lane |
| W2-13 | `85` two tests on a `symlink`-planted temp rootfs | `podbox-complete/src/write.rs:643` | both-required | `write.rs` exists; crate reaches `ring`; needs `symlink(2)` at runtime | same |
| W2-14 | `105` check G | `podbox-interpose/src/memo.rs` | both-required | crate is excluded from the workspace (`Cargo.toml:20`), so `--workspace` never reaches it; needs its own command | `cargo test --manifest-path crates/podbox-interpose/Cargo.toml --target x86_64-unknown-linux-gnu` (gate.yml:213) |
| W2-15 | `106` clauses A, B, C0 | `podbox-interpose/src/identity.rs`, `cfg(test)` above `:282` | both-required | same exclusion | same |
| W2-16 | `325` clauses 1, 2 | `podbox-cli/src/parity.rs` (`mod tests` at **707**) | both-required | crate reaches `ring` | `cargo test --workspace` |
| W2-17 | `330` case-name set | `podbox-probe/src/exit.rs` | **native-windows authorable + typecheckable** | `podbox-probe` typechecks natively with `--tests`; execution still Linux | `cargo check --tests --target x86_64-unknown-linux-gnu -p podbox-probe` |
| W2-18 | `362` Windows refusal | `podbox-cli/src/lifecycle.rs`, `windows/dos.rs`, `windows/mod.rs` | both-required | crate reaches `ring`; the subject is Windows behaviour but the harness is Linux | `cargo test --workspace` |
| W2-19 | plant per new test (15 plants) | `scripts/plant.sh:51` `FILES` | **blocked** | `plant.sh:51` lists 25 files and **does not include any of the 15 target source files**; the entry says so. `plant.sh:74` refuses to start on a dirty tree over those files. The plants are also not implementable as described until the list is extended, which is wave 4's subject | `sh scripts/plant.sh` |

**Wave 2 rollup:** native-windows 2 (W2-07, plus W2-17's authoring half);
native typecheck + Linux execution 3 (W2-05, W2-17, and W2-14/15 via their own
manifest); both-required 11; linux-lane 2; blocked 3 (W2-08, W2-11, W2-19).

### Wave 3 — structural change, registry fixture, 7 test files (`T-R003.md`)

| id | unit | host | why | exact proof command |
| --- | --- | --- | --- | --- |
| W3-STRUCT | `podbox-cli` targets are `['bin','custom-build']`, no `lib`; pick black-box or add `[lib]` | native-windows | decision + manifest edit; `crates/podbox-cli/Cargo.toml` has one `[[bin]] name="podbox"` and no `[lib]` (verified) | `cargo metadata --format-version 1 \| python -c "import json,sys;print([t['kind'] for t in json.load(sys.stdin)['packages'][0]['targets']])"` |
| W3-FIXTURE | `crates/podbox-image/tests/common/registry.rs` — bind ephemeral loopback port, answer manifest GET / blob GET / blob HEAD, return ETag + digest, shut down | both-required | authoring native; `serve_once` at `registry.rs:955` is inside `mod tests` at `:950` (verified) so there is nothing to reuse; **binding a loopback socket and answering HTTP is exactly what `std::os::unix` sockets are for, and `podbox-image` does not typecheck natively** | `cargo test -p podbox-image --test store_digest` in lane |
| W3-01 | `140-space-precheck.rs` clause 4 | `podbox-image/tests/` | both-required | crate reaches `ring` | `cargo test -p podbox-image` in lane |
| W3-02 | `355` surviving end-to-end half -> `curated_refusals.rs` | `podbox-cli/tests/` | both-required | needs W3-STRUCT first | `cargo test --workspace` in lane |
| W3-03 | `150` pulled-image half -> `acquisition.rs` | `podbox-image/tests/` | both-required | needs W3-FIXTURE | same |
| W3-04 | `340` detached-stdio path -> `detached_stdio.rs` | `podbox-cli/tests/` | both-required | needs a live container runtime | same |
| W3-05 | `373` store gate half -> `store_gates.rs` | `podbox-cli/tests/` | both-required | needs W3-STRUCT | same |
| W3-06 | `365` real mount-name path -> `podbox-probe/tests/namespace.rs` | both-required | **typecheckable natively** (probe -> 0); the assertion itself is `unshare(CLONE_NEWNS)`, a Linux-only syscall, so it can never pass on Windows | `cargo check --tests --target x86_64-unknown-linux-gnu -p podbox-probe` for authoring; run in lane |
| W3-07 | `364` logs tail, `system df` -> `qol.rs` | `podbox-cli/tests/` | both-required | crate reaches `ring` | `cargo test --workspace` in lane |

**Wave 3 rollup:** native-windows 1 (W3-STRUCT authoring); both-required 7;
the fixture is the single dependency for W3-03 and W3-01.

### Wave 4 — `crates/podbox-gate`, 6 binaries (`T-R004.md`)

| id | unit | host | why | exact proof command |
| --- | --- | --- | --- | --- |
| W4-01 | `podbox-gate` replaces `scripts/check-todo.py` (1,738 lines of Python) | both-required | authoring native and fast (no cargo dependency at all — it parses markdown); the crate must still be **built** to be run, and building needs `ring` unless the new crate takes no `ring`-reachable dep | `cargo build -p podbox-gate && ./target/debug/podbox-gate` |
| W4-02 | `podbox-count` replaces `scripts/todo-count.py` | both-required | same | same |
| W4-03 | `podbox-plant` replaces `scripts/plant.sh`; `check-todo.py:300 check_tree` must record the new path; `gate.yml:55-56` must change; 13 `Prove` lines repoint | both-required | ⛔ **the gate must be green before the port runs the new binary over the tree, and the gate is red today (§2.1)** | `grep -rn 'scripts/plant.sh' TODO/*.md \| grep -c Prove` (returns 13) |
| W4-04 | `podbox-dev` replaces `dev.sh`, `session-start.sh`, `310-session-startup.sh` | both-required | it *drives* the lane; it cannot be proven without the lane | `sh scripts/windows/run-in-base.sh` |
| W4-05 | `podbox-smoke` replaces `nightly-smoke.sh`, `398-gate-diagnostics.py` | both-required | same | same |
| W4-06 | `podbox-interpose-build` replaces `build-interpose.sh` | both-required | needs `cc` for both libcs and `nm -D` against `interpose.map`; `interpose.map` declares 112 names by `build-interpose.sh`'s own `awk` at `:199-200` | `./scripts/build-interpose.sh` in lane |
| W4-07 | shared "row grammar" + exit-code contract, two characterisation tests **before** porting | both-required | this is the entry's own ⛔ and it is the only thing that makes W4-01 behaviour-preserving | characterisation tests in lane |
| W4-08 | coupled path moves: `check-todo.py:228 CEILING_SCRIPT = "experiments/110-bloat-delta.sh"`, `plant.sh:51` `FILES`, `plant.sh:191-193` awk `CEILING_BYTES`, `gate.yml:131` | both-required | verified present at all four sites | `grep -n 'CEILING' scripts/check-todo.py` |
| W4-09 | `gate.yml:186 py_compile` glob must change | native-windows | it is a workflow-text edit plus one `py_compile` run; **but `python3` on this Windows host is a Store stub** (`docs/containers.md:142`), so run it as `py -m py_compile …` or in lane | `py -m py_compile scripts/*.py` |
| W4-10 | full four-job gate | linux-lane | `:99` interposer, `:102` musl release build, `:159` workspace clippy, `:199` workspace tests, `:213` interposer tests | `sh scripts/windows/run-in-base.sh` |

**Wave 4 rollup:** native-windows 1 (W4-09); both-required 8; linux-lane 1.

### Wave 5 — 3 crates, 11 binaries (`T-R005.md`)

| id | unit | host | why | exact proof command |
| --- | --- | --- | --- | --- |
| W5-01 | `podbox-buildstate` replaces `scripts/build-state.py` | both-required | authoring native; it reads file metadata, so a native proof is possible **only if** the new crate takes no `ring`-reachable dep — a pure std crate would typecheck natively | `cargo test -p podbox-buildstate` |
| W5-02 | `podbox-release-licenses` replaces `scripts/release-licenses.py` | both-required | same | same |
| W5-03 | `podbox-verify` replaces `scripts/verify-release.sh` | both-required | drives a release build | `sh scripts/verify-release.sh` first, then the binary |
| W5-04 | `podbox-size` replaces `experiments/110-bloat-delta.sh` | linux-lane | **needs `cargo-bloat`, which is MISSING in the measured container** and installed by `bootstrap-env.sh`; the measured container has no `jq`, no `zig`, no `openssh` either | `./experiments/110-bloat-delta.sh ci` after `bootstrap-env.sh rust bloat cc zig tools` |
| W5-05 | `podbox-prove-t0211` replaces `120-reproducible-build.sh` + `157-lock-inheritance-prove.sh` | linux-lane | both mutate git state in a copy of the tree and both run under bash | after W0-08 |
| W5-06 | `document-state` replaces `scripts/document-state.py` | both-required | same as W5-01 | |
| W5-07 | `release-notes` replaces `scripts/release-notes.sh` | both-required | same | |
| W5-08 | `podbox-reconcile` replaces `experiments/395-reconcile-repository.py` | both-required | same | |
| W5-09 | `podbox-publish` replaces `experiments/399-publication.py` | both-required | same | |
| W5-10 | `podbox-podvm` replaces `experiments/145-podvm-parity.sh` | linux-lane | the live-payload rows need a guest | lane |
| W5-11 | `podbox-podvm-workload` replaces `experiments/154-tcg-workload-spread.sh` | linux-lane | TCG spread across guests | lane |
| W5-12 | single-script bins: `153`, `251`, `366`, `190`, `125` into existing `tests/` dirs | both-required | `153` (755 lines) and `190` (624 lines) are deployment-level; `251` needs a real pty | `cargo test --workspace` in lane |
| W5-13 | `scripts/package-ssh.sh` -> `podbox-release` | both-required | needs `openssh`, MISSING in the measured container | |
| W5-14 | `experiments/60-interposer-libc.sh` stays OUT of `members` | both-required | the exclusion is the subject | `grep -n 'exclude' Cargo.toml` -> `exclude = ["crates/podbox-interpose"]` |
| W5-15 | `394-ssh-package.sh`, `397-exported-build.py` assigned to **no wave** | **blocked** | ⛔ the entry says so itself; an implementor must resolve it before wave 5 starts | decision required |
| W5-16 | `Cargo.toml` three `members` rows; 9 -> 12 -> 13 | native-windows | manifest edit | `grep -c '^    "crates/' Cargo.toml` returns **9** (verified) |

**Wave 5 rollup:** native-windows 1 (W5-16); both-required 8; linux-lane 3;
blocked 1.

### Per-wave host rollup

| wave | native-windows | both-required | linux-lane | blocked | total |
| --- | --- | --- | --- | --- | --- |
| 0 | 14 | 0 | 0 | 0 | 14 |
| 1 | 2 | 0 | 1 | 0 | 3 |
| 2 | 2 | 11 | 2 | 3 | 18 |
| 3 | 1 | 7 | 0 | 0 | 8 |
| 4 | 1 | 8 | 1 | 0 | 10 |
| 5 | 1 | 8 | 3 | 1 | 13 |
| **total** | **21** | **34** | **7** | **4** | **66** |

Reading: **roughly a third of the named work needs no lane at all, a little over
half is authorable now and provable later, and a little over a tenth needs a
Linux kernel.** The plan's per-entry blanket "nothing compiles on the Windows
host" is true of its *proofs* and false of its *work*.

---

## 5. `run-in-base.sh` versus `wslc`

### What each does, from the source

`run-in-base.sh` (`scripts/windows/run-in-base.sh`, 171 lines):

- `--workspace .` copies the tree into the base. Measured payload:
  619 MiB and 11,984 entries excluding `target`, `.git`, `.tmp`, `.dev`
  (`du -sh --exclude=target --exclude=.git --exclude=.tmp --exclude=.dev .`
  and a matching `find`). It **keeps `.git` and `references/`** (line 133-138).
- `EXCLUDES="codegraph.db codegraph.db-wal codegraph.db-shm daemon.log daemon.pid target .dev .tmp"` (line 143).
- **It must not have path conversion disabled** (line 18-21, and
  `docs/containers.md:135`). `wslc` requires `MSYS_NO_PATHCONV=1`. The two lanes
  have **opposite** Git Bash rules. Measured directly: without it, Git Bash
  rewrites `-w /work` and the call fails.
- Evidence comes back through `/out` only (line 26-30). A measurement that
  writes into `/work` has written into a copy.
- The job travels as `--input job.sh=FILE`, so two jobs from one checkout cannot
  replace one another's payload (line 124-125).
- Default job runs `./scripts/dev.sh check >/out/linux-check.txt` (line 93).

`wslc` bind-mounts the checkout (measured: a file written in `/work/.tmp`
appeared on the Windows side).

### Which property matters for this plan

**Bind-mounting is a hazard here, and copying is not merely an inconvenience.**
The decisive mechanism is already in the repository. `scripts/plant.sh:74-79`:

```
if ! git diff --quiet -- $FILES || ! git diff --cached --quiet -- $FILES; then
  echo "SKIP: one of the files this script plants into has uncommitted changes:"
```

`plant.sh` **refuses to start on a dirty tree**, and wave 2 adds 15 plants and
wave 4 ports `plant.sh` itself. Under a bind mount, every one of those plants
mutates the real checkout. The author's stated intent
(`run-in-base.sh:28`) — *"A measurement that writes only into the workspace has
written into a copy"* — is exactly the property this plan needs: 121 scripts, many
of them git-mutating (`157`, `120`, `395`, `399`, `110`), all of which must be
provable without leaving the checkout dirty. `run-in-base.sh` gives that for
free. A bind mount takes it away and would make `wslc` unusable for the very
waves it looks most attractive for.

**Is copying an advantage for parallel agents?** Yes, and specifically for these
waves:

- Two concurrent `wsl-toolkit` jobs get independent copies and independent
  `--input` payloads (line 124-125). Two concurrent bind-mounted `wslc` jobs get
  the same working tree and the same `target/`.
- `wslc` has no `--exclude`, so a bind mount carries `target/` (which the copy
  excludes). Measured `du` shows `crates/` alone is 3.2 MiB but the checkout has a
  populated `target/`; a bind mount cannot exclude it.
- Two agents running `git`-mutating plants simultaneously against one tree would
  collide. The copy lane is the only lane where wave 2's 15 plants can run
  unattended.

**Against `wslc`, honestly:**

- `MSYS_NO_PATHCONV=1` plus a full quoted path, and the **opposite** rule to
  `run-in-base.sh`. One wrapper cannot serve both without branching on which
  binary it is invoking.
- No `--privileged`, no `--cap-add`/`--cap-drop`, no seccomp/appamor/userns
  flags (confirmed in `wslc run --help`). podbox's whole subject is namespace,
  mount, device and ownership denial. The 29 KEEP-SHELL scripts need exactly
  those facilities. `wslc` cannot provide them, so it is not a substitute for the
  base lane for any deployment-level clause.
- It is not in the repository's own contract. `docs/methodology/gate.md` and
  `AGENTS.md` name `run-in-base.sh`. Introducing a lane the gate does not know
  about makes every lane-touching unit a TREE edit as well as a HOST decision.

### Recommendation

**Repair the `wsl-toolkit` base (`base ensure --repair`, with
`podman machine start`) and use `run-in-base.sh` as the single Linux lane. Use
`wslc` for exactly one thing: as an unblocker for `cargo check`/`clippy` while the
base is down, never as the wave's proof.**

Reasons, in order of weight:

1. The bind mount makes `plant.sh` unsafe, and `plant.sh` is a required proof in
   waves 2 and 4.
2. `wslc` cannot express the capabilities podbox measures, so it cannot close any
   `deployment` or `blocked` unit. It is a typechecker, not a lane.
3. `run-in-base.sh` already answers every question the six entries ask
   (`sh scripts/windows/run-in-base.sh JOB.sh`), already has 8 `TODO/` entries
   citing it, and already carries the evidence-collection contract.
4. `wslc`'s requirement for `MSYS_NO_PATHCONV=1` is a wrapper fork, not a flag.

**Cost of the repair, which is what the operator is being asked to approve:**
`podman machine start` (an 11.4 GiB vhdx, already provisioned, last up 19 hours
before this session) then `wsl-toolkit base ensure --repair`, then four
`gc --job <id> --apply` calls so the record gate can pass. Total: one VM start,
one repair, four directory removals. No repository file changes. Until it is
done, `run-in-base.sh JOB.sh` still exits 2 with `exit 125` from the base.

---

## 6. Every ⛔ in the plan, classified

`HOST` = solved by getting a lane. `TREE` = a repository property, solved by
editing, applies on any host. This is what tells a parallel agent what it can
attack with no lane at all.

| ⛔ | where | class | note |
| --- | --- | --- | --- |
| `157`'s sed names `lock.fd`; source reads `fd` | T-R000:8, T-R005 | **TREE** | verified both sides; pure text edit |
| `160` `prune` returns 0 but the saved result says otherwise | T-R001, T-R002 | **TREE** | `images.rs:961-986` prints `skipped:` then `0`; `store-gc.txt:8` reads `prune_skipped 1` |
| `388`'s tests do not name the script | T-R001 | **TREE** | `grep -c 388` = 0 |
| `220`'s six citation sites | T-R001 | **TREE** | `grep -rn` |
| Three target paths in the wave-2 table do not exist | T-R002 | **TREE** | verified: `podbox-probe/src/probe_cache.rs`, `podbox-enter/src/identity.rs`, `podbox-supervise/src/nongoals.rs` all MISSING |
| `cargo test --workspace` does not reach `podbox-interpose` | T-R002, T-R005 | **TREE** | `Cargo.toml:20 exclude` |
| Plant mechanism not implementable as described | T-R002 | **TREE** | `plant.sh:51` lists 25 files, none of them the 15 targets |
| `podbox-cli` has no `[lib]` | T-R003, PLAN | **TREE** | manifest has `[[bin]]` only |
| `serve_once` is test-private, nothing to reuse | T-R003 | **TREE** | `registry.rs:950` `mod tests`, `:955` `fn serve_once` |
| `crates/podbox-interpose` cannot host `tests/*.rs` | PLAN, T-R004 | **TREE** | `Cargo.toml:15 crate-type = ["cdylib"]` |
| `110` has three consumers, not two | T-R005 | **TREE** | `check-todo.py:228`, `plant.sh:51`, `plant.sh:191-193`, plus `gate.yml:131` |
| `394`, `397` assigned to no wave | T-R005 | **TREE** | decision required |
| `gate.yml:186 py_compile` will fail on deleted Python | T-R004 | **TREE** | workflow text |
| `py scripts/check-todo.py` exits 0 | T-R000 | **HOST** | it exits 1 today on four kept lane jobs |
| `cargo test --workspace` on this host | PLAN:154, all entries | **HOST** | no native build exists at all (§1.1) |
| KVM, ReactOS, nested-KVM host stop | PLAN:153, T-R003 | **blocked** | operator-present, not either lane |

**12 of 17 ⛔ are `TREE`.** They are solvable by editing on this host, today,
with no lane and no compiler. That is the single most useful fact in this
report for whoever schedules the parallel agents.

---

## 7. What could NOT be settled, and what would settle it

| open question | why it is open | command that settles it | cost |
| --- | --- | --- | --- |
| Is the `wslc` wedge transient or structural? | 7 consecutive `run`/`images` calls timed out; `info`/`list` still answer; no error was emitted | reboot Windows, then `wslc info && wslc run --rm docker.io/library/rust:1.98.1-bookworm sh -c 'id -u'` | one reboot |
| Does `wsl-toolkit` actually run `cargo test --workspace` green once repaired? | base is `usable false`, exit 125; never got past it | `podman machine start && wsl-toolkit base ensure --repair && sh scripts/windows/run-in-base.sh` | one VM start + full workspace test |
| Does `wslc` have user namespaces, mount namespaces, or seccomp? | the probe script was queued into the wedge and never ran | run the following inside a working `wslc` container; **the plan needs the answer** because the 29 KEEP-SHELL scripts exist on exactly these facilities | seconds, once the lane is alive |
| Does `zig` install cleanly inside the base container for the `CC_*` targets? | `zig-cc.sh` fails natively with os error 193, so a native workaround is out | `./scripts/common/bootstrap-env.sh rust cc zig tools` in lane | a few minutes |
| Do the four kept `wsl-toolkit` job directories hold anything a session still needs? | `gc` is destructive | `wsl-toolkit --instance podbox gc --json`, read it, then `gc --job <id> --apply` per id | seconds |
| Does `py scripts/check-todo.py` reach exit 0 once the four lane problems clear? | cannot separate lane debt from record state | after the `gc` calls above | seconds |

**Two corrections to the plan's own record, found while verifying:**

- `T-R002`'s table cites `crates/podbox-cli/src/system.rs:603` for the target
  `mod tests`. The actual line is **604**. `fn abi` is at **157**, not 191.
  `T-R001` cites `:603` too. Both are off by one and the second by 34.
- `T-R002` cites `crates/podbox-cli/src/parity.rs` `mod tests` at **`:706`**;
  it is at **`:707`**. The plan spends wave 0 on exactly this class of error and
  introduces two more.
