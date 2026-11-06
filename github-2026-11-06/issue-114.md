This is the reference dossier for the errand host: **every limit → mechanism → workaround**, measured, with the podbox issue each row belongs to. It is the map behind #86–#94 and #109. Written after a blind consumer round with the released binary (0.1.0-beta.12, `7ef3cf35…`) on a host that denies userns/chroot/mount/ptrace/`process_vm_readv`/`/proc/<pid>/mem` and has no `/dev/fuse`.

The two facts the whole errand turns on, both measured here: **`openat2(RESOLVE_IN_ROOT)` gives chroot semantics with no privileges, and `PR_SET_SYSCALL_USER_DISPATCH` gives a syscall interceptor inside the process with no ptrace.** Everything else is a shim over a primitive the host already grants.

---

# podbox on the constrained errand host — complete limit → mechanism → workaround dossier

Everything below was measured on **this** host, not inferred. Commands and outputs are quoted.
The question is not "can podbox run a container" (it can, for one dynamic musl process) but
"what does the *errand* need — real rust/go/zig/cmake/qemu toolchains, a UI/headless-browser round,
niche toolchains — and what is the exact mechanism for each thing that stands in the way."

Host: kernel `7.2.2-artix1-1.1`, uid 966 gid 965, x86_64.

---

## 0. The host, measured — what is denied and what is available

Denied (all measured):

| primitive | result |
| --- | --- |
| `clone(CLONE_NEWNS/NEWUSER/NEWPID)` | EPERM |
| `mount(2)` | EPERM |
| `chroot(2)` | EPERM |
| `ptrace(PTRACE_ATTACH/SEIZE)` | EPERM |
| `process_vm_readv(2)` | EPERM |
| `open("/proc/<pid>/mem")` | EACCES |
| `/dev/fuse` | absent |
| `/dev/kvm` | absent |
| `/state`, `/state/home` | rw but `execve` refused (policy, invisible in `/proc/mounts`) |
| `/tmp` | tmpfs rw, exec ok |
| `/workspace` | btrfs rw, exec ok |

Available (all measured — this is the important half):

| primitive | evidence |
| --- | --- |
| `memfd_create(2)` | returns an fd |
| `execveat(fd, "", AT_EMPTY_PATH)` | works (ENOEXEC only on empty content) |
| `openat2(2)` + `RESOLVE_IN_ROOT` | `/bin/cat` → ELF magic `7f454c46`; `../../etc/os-release` clamped; `/no/such/file` → `ENOENT` |
| **`PR_SET_SYSCALL_USER_DISPATCH`** | `prctl` returns 0; traps and emulates syscalls (see §1) |
| `seccomp(SECCOMP_SET_MODE_FILTER, NEW_LISTENER)` | filter installs, but no arg-reading without `process_vm_readv` |
| `qemu-x86_64` static user emulator | runs musl payloads with `-L` |
| `mmap(PROT_EXEC)` on a file on a noexec mount | works (data mmap, not `execve`) |

The whole errand turns on the two bold facts: **`openat2(RESOLVE_IN_ROOT)` gives chroot semantics with no privileges, and Syscall User Dispatch gives a syscall interceptor inside the process with no ptrace.**

---

## 1. The decisive new mechanism: `PR_SET_SYSCALL_USER_DISPATCH`

The `supervise` rung was declared dead because reading the traced process's path argument needs
`process_vm_readv`, which is EPERM here. That reasoning applies to **ptrace and seccomp-notify**, not
to SUD. SUD delivers `SIGSYS` to a handler **in the same address space**, so the argument pointer is
readable with an ordinary dereference.

Measured trap + emulate:

```
prctl(PR_SET_SYSCALL_USER_DISPATCH, PR_SYS_DISPATCH_EXCLUSIVE_ON, 0, 0, &selector) -> 0
traps=1 raw_getpid_via_sud=4242 real=16567
```

Measured path virtualization, end to end, using `openat2(RESOLVE_IN_ROOT)`:

```
host has /etc/os-release: no
SUD-CHROOT traps=1 rewritten=1 fd=4 line=NAME="Alpine Linux"
ID=alpine
VERSION_ID=3.20.10
```

The payload issued a raw `openat("/etc/os-release")` from **outside** the trusted region; the handler
opened it in-root and returned the fd; the guest file came back although the host has none.

The contract, from the kernel source (`kernel/entry/syscall_user_dispatch.c`):

- `PR_SYS_DISPATCH_EXCLUSIVE_ON` (`1`): `offset..offset+len` is the **always-allowed** region. The
  trusted runtime (its libc) goes there.
- `selector == 0` (`SYSCALL_DISPATCH_FILTER_ALLOW`) lets a syscall through; `selector == 1`
  (`BLOCK`) dispatches it. Earlier tests that failed used `0` where `1` was needed.
- Syscalls from outside the region are **not executed**; the handler must emulate and write the
  result to `REG_RAX`. `rt_sigreturn` and the signal trampoline live in the trusted libc range, so
  they are not re-dispatched.
- Not preserved across `execve`, `fork`, or `clone` (man page, confirmed: a forked child did not
  trap). The handler intercepts `clone`/`fork` and re-arms in the child/thread. A kernel `execve`
  would drop it, so the payload must be loaded with a **userland execve** loader.

Why this is the right layer: the loaded static payload — including its embedded libc — is entirely
outside the trusted region, so **every** syscall it makes is dispatched, with no need to wrap libc
functions and no dependence on `LD_PRELOAD` reaching anything.

References: `rzrn/9aout` (SUD for native Plan 9 binaries), `anvilsecure/ulexecve`,
`VHSgunzo/ulexec`, `io12/userland-execve-rust`, `qaidvoid/onelf` (userland execve),
`tsuzu/go-chrootfs` (openat2 `RESOLVE_IN_ROOT`), `Azathothas/memfd-ng` / `novafacing/memfd-exec`
(memfd exec with fallbacks).

Filed as podbox issue **#109**.

---

## 2. The one architectural flaw, and its six symptoms

podbox virtualises the filesystem with an **`LD_PRELOAD` libc interposer**. That layer cannot, by
construction, cover a program that does not call that libc. Every "limit" in the backlog is a
symptom of this one choice:

1. **Static binaries are not covered at all.** `lib/shim.sh` in sandhome says it plainly: "LD_PRELOAD
   cannot reach a statically linked binary."
2. **`exec`'s `PT_INTERP` resolution happens in the kernel**, not through libc, so a dynamic
   grandchild re-enters through the host loader and escapes the image (#89, #95).
3. **Relative paths are resolved against the host cwd** by `map.rs::absolutize` (`AT_FDCWD` →
   real `getcwd()`), then prefixed again by `prepare`/`rewrite` → `<rootfs><rootfs>/...` (#86, #105).
4. **`dlopen`/`dlmopen` are not defined** by the interposer (#88) — T-0708.
5. **The libc ABI union is incomplete** (`ld-linux-x86-64.so.2`'s `__tls_get_addr`,
   `libdl.so.2`'s `dlsym`), so glibc payloads are refused (#87, #96, #97).
6. **`process_vm_readv` is required by the `supervise` rung** and is EPERM here, so the fallback
   rung is unreachable.

The correct layer on this host is a **syscall interpreter** — SUD for native static payloads (§1),
qemu-user for other architectures and for dynamic payloads whose exec/threads a userland loader
cannot keep up with.

---

## 3. Limit → mechanism → workaround → reference matrix

Each row: the measured symptom, the exact mechanism, the concrete fix, the project to copy, what
sandhome does, and the podbox issue it belongs to.

### 3.1 Paths

| # | Symptom (measured) | Mechanism | Fix | Reference | sandhome | podbox |
| --- | --- | --- | --- | --- | --- | --- |
| relative paths | `touch RELFILE` → ENOENT | `absolutize` (`map.rs:506`) uses the **real** cwd (host path) then `prepare` (`:613`) prefixes it again | reverse-map the real cwd with `map::unrewrite` in `absolutize` (the interposer's own `getcwd` at `lib.rs:1145` already does this) | — | runs everything from the view root | #86, #105 |
| double prefix | `<rootfs><rootfs>/...` | same as above when the cwd is already inside the rootfs | idempotent rewrite: if the path already begins with the rootfs prefix, leave it | — | — | #105 |
| host escape on miss | `qemu -L <rootfs> <rootfs>/bin/busybox cat /workspace/hostonly.txt` printed the host file; `ls /tmp` showed the host `/tmp` | `util/path.c::path()` falls back to the **unprefixed host path** on `access()` miss (`ret = name`) | fail closed: return `-ENOENT`, and resolve through the in-root resolver | `tsuzu/go-chrootfs` | `sh_view_copy_kb` never exposes host paths | #94, #109 |
| absolute symlink escape | `<rootfs>/bin/cat -> /bin/busybox` → `No such file or directory` | `path()` does not translate absolute symlink targets | `openat2(rootfd, p, {.resolve=RESOLVE_IN_ROOT})` | `go-chrootfs` | remaps an in-tree symlink onto the view, repoints an outside symlink | #95, #109 |
| `..` escape | — | naive prefixing does not clamp | `RESOLVE_IN_ROOT` clamps `..` (measured: `../../etc/os-release` resolved in-root) | `go-chrootfs` | — | #95, #109 |

### 3.2 Dynamic linking

| # | Symptom | Mechanism | Fix | Reference | sandhome | podbox |
| --- | --- | --- | --- | --- | --- | --- |
| glibc refused | every glibc payload refused, `__tls_get_addr` / `dlsym` | `abi::union` (T-0709) omits `ld-linux-x86-64.so.2` and `libdl.so.2`; the caller passes only `libgcc_s.so.1` as `rest` | pass the full `NEEDED` closure | `pkgforge-dev/cross-libc-dlopen` | does not emulate a foreign rootfs libc at all — it installs/adopts the toolchain on the host root | #87, #96, #97 |
| one symlink decides | `lib64/ld-linux` decides gcc:13 / node:20 / golang:1.22 | absolute `lib64` symlink resolves on the host | image-relative loader substitution | `cross-libc-dlopen` | same as above | #96 |
| `dlopen` missing | `python3 -c 'import subprocess'` ImportError | `dlopen`/`dlmopen` not defined (T-0708) | define them via `dlsym(RTLD_DEFAULT)`/the loader, or route through the in-root resolver | `fritzw/ld-preload-open`, `dex4er/fakechroot` | — | #88 |
| children use host ld.so | dynamic grandchildren fail 127 | kernel `PT_INTERP` resolution bypasses the interposer | the interposer rewrites `execve`/`execveat` to run `<rootfs>/lib/ld-*.so` with the payload, and rewrites the loader path | sandhome `lib/shim.sh` loader substitution | substitutes the loader for children | #89, #95 |
| static grandchild works, dynamic fails | — | same as above | same as above | — | — | #89 |

### 3.3 Size, staging, `#!`

| # | Symptom | Mechanism | Fix | Reference | sandhome | podbox |
| --- | --- | --- | --- | --- | --- | --- |
| 128 MiB ceiling | `node:alpine` (149,234,536 B) and zig (172,641,672 B) refused | `MAX_PAYLOAD_READ = 134217728` in the payload reader | stream into a `memfd` in 128 KiB chunks; no read ceiling | `Azathothas/memfd-ng`, `novafacing/memfd-exec` | `shims/memexec.c` streamed 172 MB zig into a memfd; `memexec …/zig version` → `0.16.0` in **0.126 s** | #90 |
| exe-relative tools | `memexec …/zig env` → `unable to find zig installation directory '/memfd:sandhome (deleted)'` | the tool reads `/proc/self/exe` or `argv[0]` and needs its install tree beside it | copy such tools to a real path (sandhome's `tc_zig_copy_bins`), or place them in a view dir | sandhome `tools/zig.sh` | zig must be a real copy; `tc_zig_exec_mb=200` | #90 |
| `#!` refused | `node:alpine` exits 126 on `docker-entrypoint.sh` | the loader rung refuses scripts | read the shebang, map the interpreter through the image, exec it (or run from a memfd with the fd left open across exec so the interpreter re-opens `/proc/self/fd/N`) | `termux/termux-exec`, sandhome `shims/memexec.c` | memexec on a `#!` script printed `HI` where direct exec gave `bad interpreter: Permission denied`; adopted scripts are left on `PATH`, never linked into a view (avoids a re-resolve loop) | #92 |
| `--entrypoint` dance | `node:alpine` needs undocumented `--entrypoint node` | as above | fix the shebang case and the dance disappears | — | — | #92 |

### 3.4 Toolchains (the errand's actual content)

| toolchain | what it needs | mechanism that delivers it | evidence |
| --- | --- | --- | --- |
| **Go** | `GOROOT`, build cache, `exec` of the compiler | static payload → SUD loader, or qemu; `GOROOT` must be the **host** path, caches on an exec root | proven today: `GO_BUILD_WORKAROUND_OK` (recipe §4) |
| **zig** | exe-relative install tree, 172 MB | real copy (not a view symlink) + `--zig-lib-dir`; static → SUD/qemu | proven: `ZIG_QEMU_OK`; memexec 0.126 s |
| **rust** | `cargo` spawns `rustc` (child exec), sysroot | dynamic interposer for cargo + loader substitution for the child, or SUD loader that re-arms on clone/exec | sandhome `tools/go.sh` pattern (GOROOT view, exec-only caches) |
| **cmake** | many child execs, `gcc`, sysroot | dynamic interposer + loader substitution; `/proc/self/exe` may be read | sandhome loader substitution |
| **qemu cross** | dynamic qemu, `-L` sysroot | qemu under the interposer, or a static qemu-user; patch `path()` and `do_execve` (§3.1) | qemu path leak proven |
| **node** | 149 MB, `#!`, exe-relative | memfd streaming + shebang fix + copy | sandhome `TC_node_BINS`, `tc_node_exec_mb=16`/`200` |

Go recipe that works today:

```
podbox run --name gb -d -e GOROOT=<store>/rootfs/<golang>/usr/local/go \
  -e GOCACHE=/tmp/gocache -e GOPATH=/tmp/gopath -e GO111MODULE=off -w /tmp \
  golang:1.24-alpine sleep 600
podbox cp main.go gb:/tmp/main.go
podbox exec -e GOROOT=<store>/rootfs/<golang>/usr/local/go -e GOCACHE=/tmp/gocache \
  -e GOPATH=/tmp/gopath gb /bin/busybox sh -c \
  'cd /tmp && /usr/local/go/bin/go build -o <store>/rootfs/<golang>/tmp/gohello main.go && /tmp/gohello'
```

Two facts it establishes: `GOROOT=/usr/local/go` (guest path) **fails** with "cannot find GOROOT
directory", and `GOROOT=<store>/...` (host path) works — the binary is not running under the
virtualization it thinks it is. And `go build -o /tmp/gohello` writes **host** `/tmp/gohello`, not
`<rootfs>/tmp`, so the interposer does not cover the child compiler.

### 3.5 UI / headless browser

| need | obstacle here | mechanism | reference |
| --- | --- | --- | --- |
| display | no X, no Wayland, no bind | software rendering, headless mode | `atlassian/docker-chromium-xvfb`, `cgoldberg/xvfbwrapper` |
| GPU | no `/dev/dri` | SwiftShader / llvmpipe | `google/swiftshader`, `utensils/docker-opengl` |
| pty | no pty in the sandbox? | a userspace pty wrapper | `dtolnay/faketty` |
| sockets | no bind to `/run` | put sockets under the view root | sandhome two-root view |
| the browser itself | chromium is dynamic, reads `/proc`, `/dev/shm` | dynamic interposer + `/dev` shims + `/proc` passthrough | — |

### 3.6 Names, networking, devices

| # | Symptom | Mechanism | Fix | Reference | sandhome |
| --- | --- | --- | --- | --- | --- |
| name lookups | `getent` returns 2, `nginx` `getpwnam` fails | `/etc/passwd` present in the image, absent on the host | synthetic NSS database via `LD_PRELOAD` | `samba-team/nss_wrapper` | `shims/fakepwd.c`; derives entries from `id -u`/`id -g`, adds caller-named logins |
| overlay/volumes | no `-v` (issue #104) | no `/dev/fuse`, no `mount` | two-root **view** model (copy regular executables, symlink data back) — not an overlay | `containers/fuse-overlayfs` (needs `/dev/fuse`, so unavailable), `mpartel/bindfs`, `trapexit/mergerfs` | `sh_promote_tree`: copies execs, symlinks `.so`/`.rlib`/`.a`/data, remaps in-tree symlinks, gated by `sh_view_copy_kb` |
| networking | no userns, no `/dev/net/tun` | user-mode networking is unavailable without userns | host networking / proxy env | `rootless-containers/slirp4netns` (needs userns), `rootless-containers/bypass4netns` (seccomp-notify ADDFD), `hmgle/graftcp` (proxy redirect) | — |
| terminal | — | pty wrapper | `dtolnay/faketty` | — |

### 3.7 The `--strict` contract (#93)

`--strict` does not refuse the grandchild limit while `--strict=all` refuses benign `/dev` shims.
The contract should name the tier that is actually in use and refuse the **unreachable** ones
(ptrace, supervise) with the measured reason (`process_vm_readv: EPERM`), and the new `native-static`
tier should appear in `probe` as `measured_by.user_dispatch=true`.

---

## 4. The interception layers, ranked for this host

| rank | layer | covers | survives exec | needs | status here |
| --- | --- | --- | --- | --- | --- |
| 1 | **SUD + userland-execve + openat2** | static, native | no (re-arm in handler) | kernel 5.11+ | **works, measured** (#109) |
| 2 | `LD_PRELOAD` interposer | dynamic libc callers | yes (kernel reloads it) | the payload calls that libc | current podbox |
| 3 | qemu-user (`-L`, patched) | any arch, static+dynamic | yes (emulator re-execs) | qemu binary | works with 3 patches |
| 4 | seccomp-notify | dynamic | yes | `process_vm_readv` | **dead here** |
| 5 | ptrace | anything | yes | `ptrace` | **dead here** |
| 6 | userns/mount/chroot | anything | yes | privileges | **dead here** |

The design implication: podbox should pick the layer from the payload (static → SUD, dynamic →
interposer, foreign → qemu), not from the CLI verb.

---

## 5. What sandhome does that maps onto each of these

sandhome already runs the errand on this host, so every "blocker" has a working precedent:

- **128 MiB ceiling → memexec streaming.** `shims/memexec.c` streams into `memfd_create` and execs
  `/proc/self/fd/N` (fallback `fexecve`), fd left open across exec. 172 MB zig ran `version` in
  0.126 s.
- **No `-v` → two roots.** `SANDHOME_HOME` (data, may be noexec) + `SANDHOME_EXEC` (exec-capable);
  `sh_promote_tree` copies regular executables and symlinks the rest.
- **`#!` refusal → run from an fd.** Measured `HI` where direct exec gave permission denied.
- **Static Go → exec-only caches on the exec root.** `tc_go_env`: `GOROOT` = view path,
  `GOCACHE`/`GOTMPDIR`/`GOBIN` on `SANDHOME_EXEC`.
- **Foreign glibc → don't emulate it.** sandhome installs/adopts the toolchain on the host root. That
  is the deepest of the five and the reason the ABI-union/loader/absolute-symlink class never arises.
- **exec detection → behaviour, not `/proc/mounts`.** `sh_exec_probe` writes a file and runs it.

Every one of these is a shim over a primitive that is available here. None requires a privilege the
host denies.

---

## 6. What podbox should do, concretely

1. **Add the `native-static` rung** (#109): userland-execve loader + SUD + `openat2(RESOLVE_IN_ROOT)`
   + memfd staging. Native speed, no qemu, no privileges.
2. **Replace `path()`'s host fallback with fail-closed in-root resolution** and route every path
   syscall through it (#94, #109).
3. **Make the loader substitution cover children** (#89, #95): rewrite `execve`/`execveat` so the
   child re-enters through the image loader, and re-arm SUD on `clone`/`fork`.
4. **Complete the libc ABI union** (#87, #96) or, better, adopt sandhome's stance and do not emulate
   a foreign rootfs libc.
5. **Fix `absolutize`/`unrewrite`** (#86, #105).
6. **Stream payloads into a memfd** with no read ceiling, and copy exe-relative tools (#90).
7. **Map the shebang interpreter** (#92).
8. **Make `--strict` name the tier and refuse only the unreachable ones** (#93).

Each of these has a comment on its issue with the mechanism, a patch, and a clause that fails against
the current code.

