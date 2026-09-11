# Running Nix on this restricted sandbox — the whole journey

Goal: run Nix (https://github.com/DavHau/nix-portable and
https://github.com/containerbase/nix-prebuild as starting points) inside this
container sandbox, and document everything that worked, everything that broke,
and every patch/workaround needed to get there.

**Final answer (TL;DR):** a pure-`chroot(2)` sandbox — no VMs, no namespaces,
no mounts — running the *official* **nix 2.2.2** binary tarball (the last Nix
whose build output uses a pipe instead of a pty) plus **nixpkgs 22.05** (the
last Nixpkgs that accepts Nix 2.2). Inside the chroot:

- `fetchTarball` over TLS works,
- Nixpkgs evaluates,
- `nix-build` **compiles packages locally** (substituting the compiler
  closure from cache.nixos.org),
- the built artifact **runs** (`Hello, world!`),
- sandboxed builds fail with a clean, explained error (namespace syscalls are
  blocked by the host seccomp filter).

Everything is reproducible from `scripts/`, raw evidence lives in `logs/`.

---

## 0. The environment (what we're fighting)

| Property | Value |
|---|---|
| Kernel | 6.18.39-gentoo-dist-bin, x86_64, 16 cores |
| Identity | uid 0 (root), `CapEff` nominally full |
| Hardening | `NoNewPrivs=1`, `Seccomp=2` (one filter) |
| FS | writable tmpfs `/` (16 GiB) — **except `/` itself** |

Blocked syscalls / operations (probe source: `scripts/seccomp-probe.c`,
results in logs and notes below):

| Operation | Result |
|---|---|
| `unshare(CLONE_NEWUSER)` / `CLONE_NEWNS` | `EPERM` |
| `mount(2)` (any) | `EPERM` |
| `ptrace` | `EPERM` |
| `setuid` / `chown` | `EINVAL` (filter maps them) |
| `setgroups`, `keyctl`, `bpf`, `perf_event_open` | `EPERM` |
| `mknod` char devices | only `0:0` allowed (overlay whiteout); `1:3` (null), `1:5` (zero), `5:x` (tty/ptmx) etc. all `EPERM` |
| `chroot(2)` | **works** ← the key to the final solution |
| `/dev/ptmx`, `/dev/pts` | do not exist; no devpts can ever be mounted → **no ptys at all on the host** |
| `readdir("/")` | `Permission denied` (`ls /`, `find /` fail; opening entries *by name* works, e.g. `/dev/null`, `/usr/bin`) |
| `mkdir`/`creat` at `/` top level | `Permission denied` (can't create `/nix`, no `/var`, `/run`) |

`mknod`-allowlist probe: of majors 0,1,2,3,4,5,6,7,9,10,11,13,14,21,29,81,108,
136,166,180,202,226,234,254,511 (minor 0) **only char 0:0 succeeded**.

---

## Attempt 1 — DavHau/nix-portable v012 → **dead**

Logs: `logs/01-*.log` · Patch: `patches/01-nix-portable-find-root.patch`,
`scripts/01-patch-nix-portable.py`

1. **First failure — launcher itself.** nix-portable runs
   `find / -mindepth 1 -maxdepth 1` to collect bind sources; `readdir("/")` is
   denied in this sandbox, so it bailed with `find: /: Permission denied`.

2. **Patch constraint discovered the hard way.** nix-portable is a bash
   script with a ZIP payload appended; member offsets are absolute, so any
   change of file size breaks self-extraction (`unzip: short read`). The
   patch must be *byte-size preserving*: replace the `find` line with a
   hard-coded list of the (enumerated-by-probing) top-level dirs, padded with
   spaces to exactly 82 bytes. Applied via
   `scripts/01-patch-nix-portable.py`; `ls`-probing showed the root contains
   `bin dev etc home lib lib64 opt proc sbin tmp usr`.

3. **After the patch, the runtimes fail exactly as the seccomp probe
   predicts:**
   - default/direct mode → `error: opening pseudoterminal master: No such file
     or directory` (nix wants a pty; there is no `/dev/ptmx` and devpts can't
     be mounted),
   - `NP_RUNTIME=bwrap` → `bwrap: Failed to make / slave: Operation not
     permitted` (`mount` blocked),
   - `NP_RUNTIME=proot` → `proot error: ptrace(TRACEME): Operation not
     permitted`,
   - `NP_RUNTIME=userns` → `unshare(CLONE_NEWUSER)` `EPERM`.

   (Also fixed on the way: nix-portable's bundled busybox `unzip -qqoj` needed
   the byte-size-preserving patch; host `unzip` and the bundled one both
   worked fine on an unmodified file, confirming the offset diagnosis.)

**Verdict:** all four runtimes need something the seccomp filter forbids
(mount-ns, userns, ptrace, pty). Unfixable without host changes.

## Attempt 2 — containerbase/nix-prebuild 2.35.2 → **works partially, on the host**

Logs: `logs/02-*.log`

The release is a single **statically linked** `nix` 2.35.2 binary (37 MB,
sha512 verified). No store libs needed — this is the most portable Nix
available. What works and what doesn't, with a relocated store
(`--store /some/dir/nix-store`, since `/nix` can't be created on the host):

| Capability | Status |
|---|---|
| `nix --version`, `store ping`, eval | ✓ (needs `build-users-group =` empty else it wants `/nix/var/nix/userpool`) |
| GitHub tarball fetch, TLS, DNS | ✓ |
| **substitute-only builds**: `nix build github:NixOS/nixpkgs/nixos-25.05#hello` into relocated store | ✓ (23 MB closure, `result → hello-2.12.1`) |
| path-flake builds (`nix build .#hello` with `inputs.nixpkgs.url = "github:..."`) | ✓ (`flake-demo/`) |
| **indirect refs** (`nixpkgs#hello`, `nix registry list`) | ✗ `opening file "/": Permission denied` |
| local builds | ✗ `opening pseudoterminal master` |
| running built artifacts | ✗ interpreter is `/nix/store/.../ld-linux.so`; that path can't exist on the host |

Two host quirks documented on the way:

- **Bare flake IDs are parsed as *path* refs.** Even with
  `--flake-registry ""` and no user registry, `nixpkgs#lib.version` dies
  instantly with `opening file "/"` — independent of any registry content,
  consistent with scheme-less refs falling into path-flake handling that
  walks parent directories up to `/` (exact throw site not pinned; ptrace is
  unavailable here). Workaround: always use explicit refs
  (`github:owner/repo/rev`) or a local path flake (proven in
  `flake-demo/`, logs `02-nix-prebuild-path-flake-workaround.log`).
- `chown` is remapped to `EINVAL` by the filter, so `tar x` of archives with
  owners warns (`Cannot change ownership … Invalid argument`) — harmless with
  `--no-same-owner`.

## Attempt 3 — QEMU micro-VM (Alpine linux-virt kernel + busybox initramfs)

Scripts kept in `scripts/vm-*.sh`, logs `logs/03-*.log` (built, ~2 s boot,
but **superseded per task constraints** — no VMs in the final solution).
It got as far as a bootable 15 MB initramfs carrying busybox-static + the
static nix + virtio-net modules + CA bundle. Documented here only because the
investigation produced the version-forensics used later.

## Attempt 4 — the hinted `nix-community/nix-user-chroot` → **structurally impossible**

Log: `logs/04a-nix-user-chroot.log`

The tool (now Rust) needs `unshare(CLONE_NEWUSER | CLONE_NEWNS)` + bind
mounts to make `/nix` appear. On this host it panics:

```
thread 'main' panicked at src/main.rs:406:70:
unshare failed: EPERM
```

Same story for any user-namespace or proot approach. **But plain `chroot(2)`
is permitted** — so we invert the idea: instead of mounting `/nix` onto the
host, build a private root in which `/nix` *already is* the root.

## Attempt 5 (final) — **nix-in-chroot, the real solution**

Scripts: `scripts/chroot-create.sh`, `scripts/chroot-run.sh` ·
Logs: `logs/04-chroot-create.log`, `logs/04c-chroot-run.log` (raw, unedited)

### 5.1 Version forensics: why exactly Nix 2.2.2

Reading `derivation-builder.cc` / `local-derivation-goal.cc` / `build.cc`
across all Nix tags (evidence in the log excerpts above):

- **Nix ≤ 2.2.2**: builder output is captured through a *pipe*.
- **Nix ≥ 2.3.0** (including the whole maintenance-2.3 line, 2.4 … 2.35):
  `posix_openpt()` in `startBuilder()`, unconditional — no setting can
  disable it. This sandbox has no `/dev/ptmx`, cannot `mount` devpts, and
  cannot `mknod` ptmx (`5:2` blocked) → **every Nix ≥ 2.3 cannot build here**,
  not on the host, not in a chroot.

Sourcing 2.2.2: channels.nixos.org and GitHub releases no longer carry the
old binary tarballs (the Hydra product for 2.3.18 still exists in the binary
cache — `logs/04b-fetch-nix-2.3-tarball.log` — but is pty-era), while
`https://nixos.org/releases/nix/nix-2.2.2/nix-2.2.2-x86_64-linux.tar.bz2`
still serves 200 (23 MB, contains `store/` + `.reginfo`).

### 5.2 The rootfs (scripts/chroot-create.sh)

Assembled by hand, no package manager needed:

- **official nix-2.2.2 binary tarball** → extracted to `rootfs/nix` (its
  `store/` + `.reginfo`; all binaries are glibc-dynamic with RPATHs and ELF
  interpreters pointing into `/nix/store/...` — real paths *inside the
  chroot*),
- **busybox-static** (musl, from the Alpine `busybox-static` apk) at
  `/bin/busybox` + applet symlinks — the shell needs no libs,
- static **nix 2.35.2** (containerbase) at `/usr/bin/nix` for completeness
  (works on the host side; inside the chroot it dies on
  `readlink /proc/self/exe` — it wants to locate itself, and there is no
  procfs — documented, not used there),
- `/etc/{passwd,group,resolv.conf,nsswitch.conf,nix/nix.conf}`, the host CA
  bundle at `/etc/ssl/certs/ca-certificates.crt` (`NIX_SSL_CERT_FILE`),
- `nix.conf`: `build-users-group =` (single-user), `sandbox = false`
  (namespaces are blocked), cache + key pinned,
- **`/dev` is fake** (see below).

### 5.3 The two nasty gaps in the chroot, and their fixes

**(a) `/dev/null`, `/dev/zero`, `/dev/urandom` cannot be device nodes.**
`mknod` allowlist = char 0:0 only. Regular-file stand-ins:

- `null`, `zero`, `full`, `tty`, `console` → empty regular files (reads = EOF,
  writes accumulate; fine for shell/gcc/make),
- `random`/`urandom` → **4 KiB of genuine host entropy**. Discovered by
  failure: nix 2.2.2's `CurlDownloader` contains a `std::random_device`
  (gcc-7.3 libstdc++), which `fopen()`s `/dev/urandom`; an empty file made it
  throw `random_device could not be read` during the first download. The pool
  fixes it. Honest caveat: not cryptographically sound (same bytes per open,
  finite), but harmless here — modern glibc/openssl use `getrandom(2)`,
  which works.

  An alternative LD_PRELOAD shim (`scripts/urandom-shim.c`) that emulates
  urandom via `getrandom(2)` is included; it works for dynamic binaries but
  can't intercept libstdc++'s `syscall()`-based opens, so the entropy-pool
  file is the primary fix.

**(b) No `/proc` → no `/dev/fd/N` → bash process substitution broken.**
`mount -t proc` is blocked, so the chroot has no procfs. This bites in the
*nixpkgs fixup hooks*, not in nix itself:

- `patchelf` setup hook line 7: `done < <(find ...)` → fix: `dontPatchELF`
  (skips RPATH shrinking only; cosmetic),
- `make-symlinks-relative.sh` line 6: same pattern → `dontRewriteSymlinks`,
- preemptively also `dontPatchShebangs` and `noAuditTmpdir` (same `< <(...)`
  pattern; a scan of all 38 `pkgs/build-support/setup-hooks/*.sh` at 22.05
  found exactly these process substitutions — see the log trail).

With those four attrs the whole fixup completes (man-page gzip + `strip`
hooks use plain `find | while`, they run fine).

### 5.4 The other half of the version pin: nixpkgs 22.05

Nixpkgs enforces a minimum Nix at eval: 22.11 aborts with
*"This version of Nixpkgs requires Nix >= 2.3"*. **22.05 still evaluates on
2.2.2** (verified: `lib.version == "22.05pre-git"`). Since we need the last
pipe-based Nix, the combo is forced:

> **nix 2.2.2 + nixpkgs 22.05** — the newest *release pair* where the *whole*
> pipeline (fetch → eval → build → run) works under this seccomp filter
> (minver verified upstream: 22.05 = "2.2", 22.11 = "2.3"; 23.11 additionally
> requires the Nix ≥ 2.4 builtin `builtins.isPath`).

### 5.5 Results (raw log: `logs/04c-chroot-run.log`)

```
=== register binary-tarball closure in the nix DB ===
DB-REGISTERED                          (nix-store --load-db < /nix/.reginfo)
=== nixpkgs-22.05 eval via fetchTarball (needs TLS+DNS) ===
unpacking 'https://github.com/NixOS/nixpkgs/archive/refs/tags/22.05.tar.gz'...
=== BUILD (local compile; sandbox=false, pipe-based builder output) ===
... full gcc build of hello (autoconf, make, 5/7 tests PASS) ...
post-installation fixup ... gzipping man pages ... stripping ...
/nix/store/i4pfn82f4d7slfhay5428v8cbr1iklm1-hello-forced-local-2.12
=== RUN the artifact inside the chroot ===
Hello, world!
=== negative test: sandboxed build ===
error: setgroups failed: Operation not permitted
error: unable to start build process
```

The `setgroups` failure is the *expected* proof that the seccomp filter —
not our setup — is the wall for sandboxed builds.

## Capability matrix (final state)

| | host + static nix 2.35.2 (`--store dir`) | chroot + nix 2.2.2 |
|---|---|---|
| eval (incl. flakes) | ✓ (use explicit refs, not `nixpkgs#`) | ✓ (Nix-language of the 2.2 era) |
| fetch (TLS/DNS) | ✓ | ✓ |
| substitute from cache.nixos.org | ✓ | ✓ |
| **local builds** | ✗ (pty) | **✓** (sandbox=false) |
| **run artifacts** | ✗ (`/nix` path) | **✓** |
| sandboxed builds | ✗ | ✗ (clean `setgroups` EPERM; needs namespaces) |
| modern flakes inside chroot | n/a | ✗ as built (static 2.35 needs `/proc/self/exe`; see docs/latest-nix-shim.md for the dynamic+shim blueprint) |

## Extensions

### fastfetch inside the chroot (logs/05-fastfetch-in-chroot.log)

A from-source package build to prove the toolchain generalises:
`/root/fastfetch.nix` builds **fastfetch 1.12.2** (cmake, gcc-11 stdenv from
the substituted 22.05 closure) with nix 2.2.2 in the chroot, then runs it.
Output shows only what is truthfully reachable without procfs (OS from
/etc/os-release, kernel via uname, uptime via syscall, locale); /proc-fed
modules (CPU model, Memory, Disk) are absent — an honest limitation of the
chroot, not a build failure.

Why not nixpkgs 23.11's fastfetch? It exists (`pkgs/tools/misc/fastfetch`)
but 23.11's `lib/strings.nix` requires `builtins.isPath` (Nix >= 2.4) and its
minver gate is `2.3` — beyond a one-line patch. 22.05 + from-source is the
era-consistent route.

### A wrapper/shim for running the latest Nix (docs/latest-nix-shim.md)

`scripts/nix-pty-shim.c` — an LD_PRELOAD shim emulating the pseudoterminal
that every Nix >= 2.3 insists on (socketpair instead of devpts) plus a
`/proc/self/exe` readlink escape hatch. Proven with a harness reproducing
nix's exact call sequence (logs/06-pty-shim-poc.log): without the shim
`posix_openpt: No such file or directory`; with it the full flow — exec'd
builder writing over the emulated pty, parent reading to EOF — succeeds, and
`nix-instantiate 2.3.18 --version` works under the `/proc/self/exe` shim.
A real 2.3.18 build with the shim executed the builder and produced correct
output; the remaining goal-completion integration is scoped in the doc.
The full blueprint for latest-Nix-in-a-chroot (dynamic 2.35 closure + shim +
per-version chroot/store) is in `docs/latest-nix-shim.md`.

## Reproduce

```sh
scripts/chroot-create.sh   # downloads tarballs, assembles chroot/rootfs (~55 MB)
scripts/chroot-run.sh      # registers DB, fetches nixpkgs, builds hello, runs it
```

Caveats, honestly stated:

- builds must be sandboxed=false (namespace syscalls are host-blocked);
  builders run as root inside the chroot,
- no `/proc`: fixup hooks using process substitution are disabled via the
  four `dont*` attrs; anything needing procfs at build time will fail,
- `/dev/urandom` is a finite entropy pool (fine for builds, not for crypto),
- Nix language/infra is pinned to the 2.2.2/22.05 era; no flakes,
- `nix-instantiate --version` aborts with a `/proc/self/exe` read error
  (cosmetic; every real operation works).

## Artifact map

- `scripts/` — all stages (00 probe … 04 chroot); each is idempotent
- `patches/01-nix-portable-find-root.patch` — the byte-size-preserving
  nix-portable patch + explanation
- `logs/` — raw unedited command output for every attempt
- `notes/seccomp-probe.c` output — the syscall matrix above
- `docs/latest-nix-shim.md` — wrapper/shim design for running latest Nix
- `flake-demo/` — proof that path-flakes + static 2.35.2 work on the host
