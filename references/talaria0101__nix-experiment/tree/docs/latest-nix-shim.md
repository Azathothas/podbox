# A wrapper/shim to run the latest Nix on this sandbox — design, PoC, and open work

Status: **design validated in parts, integration incomplete.** This document
lays out what it would take to run current Nix (2.35.x) in this sandbox, what
we have already proven with a working prototype, and precisely which pieces
remain. Everything here is derived from source reading of Nix 2.35.2 / 2.3.18
and from experiments in this repo (logs/06-pty-shim-poc.log).

## 1. Why the latest Nix cannot run as-is (complete list of walls)

For **evaluation / substitution** (no local builds) the containerbase static
2.35.2 binary already works on the host and in a chroot with its own store,
except:

| # | Wall | Who hits it | Root cause |
|---|------|-------------|------------|
| W1 | `opening pseudoterminal master` | every build, any Nix ≥ 2.3.0 | `startBuilder()` calls `posix_openpt()` unconditionally (src/libstore/build/derivation-builder.cc). This host has no `/dev/ptmx`, `mount(2)` is blocked (no devpts), and `mknod` of char 5:2 is blocked by seccomp. |
| W2 | `reading symbolic link "/proc/self/exe"` | static Nix ≥ 2.2 at startup | Nix locates itself via `/proc/self/exe`; the chroot has no procfs and cannot mount one. |
| W3 | store DB schema conflict | two Nix versions sharing one state dir | Nix migrates the SQLite schema on open (one-way, by design). We did not test the breakage; per-version chroots sidestep it entirely. |
| W4 | sandboxed builds | Nix ≥ 2.3 with `sandbox = true` | build user setup + `clone(CLONE_NEWUSER\|NEWPID\|NEWNS\|…)`: all blocked (`setgroups`, `unshare` → EPERM). |

W4 is fundamental to this seccomp filter — the shim architecture below keeps
`sandbox = false` and does **not** attempt to emulate namespaces.

## 2. The shim architecture (proven mechanisms)

`scripts/nix-pty-shim.c` is an `LD_PRELOAD` shim. Preloading works for
**dynamically linked** Nix binaries — the official binary tarballs are exactly
that (unlike the containerbase static build). Requirements: the closure of a
dynamic current Nix (e.g. `nix-2.35.2` from cache.nixos.org, built by Hydra)
placed inside the chroot's `/nix/store`.

### 2.1 Emulated pty (fixes W1) — implemented and proven

Nix's exact call sequence in `startBuilder()` / `commonChildInit()`:

```
posix_openpt(O_RDWR|O_NOCTTY)      -> master fd
ptsname_r(master)                  -> slave path
[grantpt / unlockpt]               -> (glibc defaults)
open(slavePath, O_RDWR|O_NOCTTY)   -> slave fd (parent)
tcgetattr(slave); cfmakeraw; tcsetattr(slave, TCSANOW, ...)
fork(); child: setsid(); dup2(slave, 2); dup2(2, 1); execve(builder)
parent: close(slave); poll/read(master) until builder exit
```

The shim replaces the pty with an `AF_UNIX` socketpair:

| intercepted | replacement |
|---|---|
| `posix_openpt` | `socketpair()`; return `sv[0]` as master, stash `sv[1]` |
| `grantpt`, `unlockpt` | return 0 |
| `ptsname`, `ptsname_r` | return `/dev/.nix-pty-emu/0` |
| `open`/`open64`/`openat` of that path | `dup()` of the stashed slave fd |
| `tcgetattr`/`tcsetattr` on marked fds | fake success (sockets give ENOTTY) |
| `isatty` on marked fds | 1 |

This makes the pty a plain bidirectional pipe — which is all Nix uses it for
(builder logs + the `"\1"` build-setup handshake line).

**PoC result** (`logs/06-pty-shim-poc.log`, harness in
`scripts/ptyharness.c` reproducing the exact call sequence above):
without the shim `posix_openpt: No such file or directory`; with the shim the
whole flow succeeds — emulated slave handed out, `exec`'d child writes through
it, parent reads to EOF. A real `nix 2.3.18` build with the shim preloaded
executed the builder and produced the correct derivation output
(`/nix/store/…-pty-shim-poc` containing `pty-shim-works`).

The same shim covers **any pty-era Nix**, including 2.35: the call sequence is
unchanged since 2.3 (verified in 2.35.2's `derivation-builder.cc`).

### 2.2 `/proc/self/exe` (fixes W2) — implemented and proven

`readlink("/proc/self/exe")` returns `$NIX_SHIM_EXE` when set (the wrapper
script exports its own real store path). Proven: `nix-instantiate --version`
of the dynamic 2.3.18 binary, which previously aborted with
`reading symbolic link "/proc/self/exe"`, prints its version with the shim.

Applies to dynamic binaries only. The static containerbase binary cannot be
preloaded — use the official dynamic closure instead.

### 2.3 Store separation (fixes W3) — design

Give every Nix generation its own chroot + store pair:

```
chroot-2.2/rootfs/nix/store      # nix 2.2.2 era (today's working solution)
chroot-235/rootfs/nix/store      # nix 2.35 dynamic closure + shim + its own db
```

Store dir is compiled in (`/nix/store`), so isolation is achieved by separate
chroots, not by flags. The DB then also lives per-chroot (`/nix/var/nix/db`),
which removes W3. Builds under 2.35 write into `chroot-235`'s store; artifacts
run inside that chroot (their interpreters resolve).

### 2.4 Sandboxing (W4) — out of scope by design

Keep `sandbox = false`. Emulating user/mount namespaces in userspace is out of
scope (that is what proot does, and `ptrace` is blocked here). This is the
same trade-off the working 2.2.2 solution makes.

## 3. Open integration issues (found during the PoC, with evidence)

Running the shim against a real `nix 2.3.18` end-to-end (build hook →
`startBuilder` → builder) exposed three fine points that the harness does not
exercise. All are shim-engineering, none are new kernel walls:

1. **vfork vs fork.** Nix starts builders with `allowVfork = true`
   (`!buildUser && !isBuiltin`). A vforked child shares the parent's memory:
   the shim's bookkeeping (`g_master`/`g_slave`/marks) and its mutex would be
   mutated in the parent's live state. The shim already interposes
   `vfork -> fork` (semantically identical for Nix here).
2. **Helper lifetime.** A short-lived forked helper (the build hook /
   vfork child — the exact role was not fully disambiguated in the PoC logs)
   interacted with the shim's slot state after receiving fds inherited from
   the parent. Note the hook instance itself uses its *own* pipe
   (`HookInstance::builderOut`), not the goal pty. The shim's slot teardown
   must be keyed per process (pid + master fd) so a helper closing an
   inherited fd cannot affect the goal's state.
3. **Post-build EOF.** In the PoC the parent kept polling the master after the
   builder exited successfully (correct output was already in the store). With
   the slave-ownership transfer and the child made passive after fork, the
   remaining suspect is the goal machinery waiting on a second fd (hook
   channel), not the pty. Next step: run with `build-hook =` set to an empty
   string (`--option build-hook ""`) to eliminate the hook from the loop, and
   mark all shim fds `O_CLOEXEC` at creation.

## 4. Full blueprint for "latest Nix in this sandbox"

```
scripts/nix-pty-shim.so            (exists; fixes W1+W2 for dynamic Nix)
+ dynamic nix-2.35.2 closure       (substitutable from cache.nixos.org)
+ dedicated chroot                 (fixes W3)
+ nix.conf: sandbox=false, build-users-group=
+ wrapper script exporting:
    LD_PRELOAD=/usr/lib/nix-pty-shim.so
    NIX_SHIM_EXE=/nix/store/<nix-2.35.2>/bin/nix
    NIX_SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
  -> modern flakes evaluation, substitution, AND local builds
```

Estimated remaining effort: resolving §3.2/§3.3 (goal-completion path), which
is bounded debugging with the logging shim (`NIX_SHIM_DEBUG=1` writes
`/tmp/nix-shim.log`) — the kernel/seccomp layer is fully solved by the
already-proven interpositions.
