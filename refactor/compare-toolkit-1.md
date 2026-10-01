# compare-toolkit-1: the non-root capability of the wsl-toolkit build lane

Audited 2026-10-01 on Windows 11, wsl-toolkit 6.0.0, instance `podbox`
(distro `wsl-toolkit-podbox`, podman 6.1.2 / crun, rootless, cgroup v2 not
delegated). Image under test `docker.io/library/rust:1.98.1-bookworm`.

⭐ **VERDICT: the owner is right that non-root works, and right for a reason
that is not the one on the table. `wsl-toolkit run --user 1000:1000` builds
and tests `podbox-probe` and `podbox-complete` end to end, 159 tests green. But
it does NOT buy the thing it was proposed for: root and uid 1000 take the SAME
arm of `devices.rs` in this lane, because root here lacks `CAP_MKNOD` and the
kernel refuses `mknod` below the capability layer. The premise that root never
reaches the mknod-denied path is FALSE on this host.**

⭐ The one real blocker is the bootstrap: `bootstrap-env.sh` installs zig to
`/opt/zig` and symlinks `/usr/local/bin/zig`, and neither is writable by uid
1000, and the image has no `sudo`. Three components FAILED. It is a five-line
fix (a `ZIG_PREFIX` that honours `$HOME`), not a structural limit.

## The mechanism table

| mechanism | exact command | uid measured | workspace copied? | verdict |
| --- | --- | --- | --- | --- |
| `run --user 1000:1000`, no workspace | `wsl-toolkit --instance podbox run --image docker.io/library/rust:1.98.1-bookworm --user 1000:1000 -c 'id -u'` | 1000 (`uid=1000(toolkit) gid=1000(1000)`) | no, `/work` empty | works |
| `run --user toolkit` (the NAME) | same with `--user toolkit` | none: container never started | no | ⛔ REFUSED: `Error: unable to find user toolkit: no matching entries in passwd file`, exit 2 |
| `run --user 1000:1000 --workspace` | `wsl-toolkit --instance podbox run --image ... --user 1000:1000 --workspace <dir> -c '...'` | 1000 | yes, 521 entries, 31.4 MiB | works, `/work` is `1000 1000` |
| `run` as root (default, no `--user`) | `... run --image ... -c '...'` | 0 | yes | works, `/work` is `0 0` |
| `base exec` (default, no `--root`) | `wsl-toolkit --instance podbox base exec -c 'id -u; command -v cargo'` | 1000 (`toolkit`) | n/a, no workspace | ⚠ non-root confirmed, but ⛔ NO cargo, ⛔ NO rustc, ⛔ `CARGO_HOME` empty, ⛔ `/workspaces` absent |
| `base exec --dir /workspaces/podbox` | `wsl-toolkit --instance podbox base exec --dir /workspaces/podbox -c 'pwd'` | n/a | n/a | ⛔ `CreateProcessCommon:809: chdir(/workspaces/podbox) failed 2`, then listed `/` |
| `base grant --source <checkout> --mode rw` | `wsl-toolkit --instance podbox base grant --source 'C:\Users\AjamX\Downloads\podbox' --mode rw --json` | n/a | n/a | ⛔ REFUSED, exit 2: `base.mounts requires base.automount to be "off", so ungranted Windows drives are absent` |
| `matrix --user 1000:1000 --parallel 2` | `wsl-toolkit --instance podbox matrix --images alpine,debian12 --parallel 2 --user 1000:1000 -c 'id -u; ...'` | 1000 on both rows | no | works, `2 ran, 0 failed ... in 3s` |
| `wslc run -u 1000:1000` | `& 'C:\Program Files\WSL\wslc.exe' run -u 1000:1000 docker.io/library/alpine:latest id` | none | n/a | ⛔ WEDGED: `ERROR_SHARING_VIOLATION`, exit 1 |

## A. The bootstrap as a non-root user: it FAILS, and the reason is exact

`scripts/common/bootstrap-env.sh` pins `ZIG_PREFIX="/opt/zig"` at its line 71
and writes with `$SUDO mv ... "$ZIG_PREFIX"` and `$SUDO ln -sf ... /usr/local/bin/zig`
at lines 327-328. Its own guard is at lines 130-138: not root and no `sudo`
means installs are "reported as FAILED rather than attempted", and there is no
fallback prefix anywhere in the script.

Command, and its exit code read from the process:

```powershell
wsl-toolkit --instance podbox run --image docker.io/library/rust:1.98.1-bookworm `
  --user 1000:1000 --workspace 'C:\Users\AjamX\Downloads\wtk-staging' `
  --timeout 20m -c 'bash ./scripts/common/bootstrap-env.sh rust cc zig tools openssh'
```

Exact text, uid 1000:

```
== disk, before anything is written
  975174 MB and 67038882 inodes available under /work

== components
⚠ not root and no sudo: anything needing a package install will be
  reported as FAILED rather than attempted.

  ok       rust: 1.98.1, musl target present
  ok       cc: gcc 12
  FAILED   tools: apt-get install failed for jq
  FAILED   openssh: apt-get install failed for openssh-server
  FAILED   zig: unpack or link failed

== 2 already present, 0 installed, 3 failed
=== BOOTSTRAP EXIT=1 ===
```

and the reason, on stderr, verbatim:

```
mv: cannot move '/tmp/tmp.IJ8WUUeRl7/zig-x86_64-linux-0.16.0' to '/opt/zig': Permission denied
```

The same script as root, for contrast, exits 0 in 34 s:

```
== 2 already present, 3 installed, 0 failed
everything requested is present.
```

What the uid question actually costs, measured not assumed:

| path | writable by uid 1000 | who owns it |
| --- | --- | --- |
| `/work` | yes, and it is `1000 1000` because `--user` also sets the copy's owner | `1000 1000` |
| `/work/target` (created by the build) | yes | `1000 1000` |
| `$CARGO_HOME` = `/usr/local/cargo` | ⭐ YES, mode `drwxrwxrwx` | `0 0` |
| `~/.cargo` | yes (but `~` is `/work`, not `$CARGO_HOME`) | `1000 1000` |
| `/opt` | ⛔ no | root |
| `/usr/local/bin` | ⛔ no | root |
| `/run/sshd` | ⛔ `mkdir: cannot create directory '/run/sshd': Permission denied` | root |
| `sudo` | ⛔ absent from the image | - |

⭐ `CARGO_HOME` being mode 777 is the fact that makes a non-root lane cheap:
cargo's registry and git cache live there and need no fix. `zig` and `jq` are
the only things that need a root path.

### The bootstrap blocker has a short fix, and I proved it works

`ZIG_PREFIX` is a plain shell variable. A lane that sets it to a directory the
user owns needs no script change and no `sudo`. Verified end to end, uid 1000,
same pin and same checksum, prefix under `$HOME`:

```
== who am i
1000
toolkit
== installing zig into /work/.local/zig (user-owned, NOT /opt)
sha256 OK
unpacked to a user-writable prefix
== zig on PATH
/work/.local/zig/zig-x86_64-linux-0.16.0/zig
0.16.0
== CC wrapper sees zig?
clang version 21.1.0
Target: x86_64-unknown-linux5.10.0-musl
== cargo test -p podbox-complete as 1000
TEST_RC=0
SECONDS=36
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

⚠ The zig download and unpack is NOT free and NOT free to share. It is ~53 MB
and about 25 s per job, and `--workspace` copies a FRESH tree into every job,
so a prefix written under `/work` by one job is absent from the next. Measured:
the second attempt at the same script failed with `zig version` printing
nothing and the build dying at
`zig-cc.sh: zig is not on PATH. Install it with ./scripts/common/bootstrap-env.sh zig`.
Anything that depends on that prefix has to install it inside the same job.

## B. Per-crate non-root results

Staged workspace `C:\Users\AjamX\Downloads\wtk-staging` (32 MB: `.cargo`,
`crates`, `scripts`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`). The
checkout itself is 3476 MB, of which `target` is 2391 MB, which would exceed
`--max-bytes` (1 GiB) on a full copy. `du -sm` on the checkout, and
`wsl-toolkit run` reporting `workspace: 521 entries, 31.4 MiB copied` for the
staged tree.

| crate | ring in the graph? | bootstrap needed | uid 1000 result |
| --- | --- | --- | --- |
| `podbox-probe` | no | no | ⭐ 108 passed, 0 failed, exit 0, 16 s cold |
| `podbox-extract` | ⛔ YES, via `podbox-image` | zig | ⛔ build fails at `cc-rs`, `TEST_RC=101` |
| `podbox-image` | ⛔ YES, it is the crate that pulls `rustls` | zig | not run separately; reached through `podbox-complete` |
| `podbox-complete` | ⛔ YES, via `podbox-image` | zig | ⭐ 51 passed, 0 failed, exit 0, 36 s cold |
| `podbox-ssh` | ⛔ YES | zig + openssh-server | ⛔ 1 test fails on a MISSING BINARY |

⚠ **`podbox-extract` is NOT bootstrap-free.** The assignment called it one of
the two crates to isolate the uid question from the bootstrap question, and
that is wrong. `crates/podbox-extract/Cargo.toml:18` declares
`podbox-image = { path = "../podbox-image" }`, and `podbox-image` is the crate
that pulls `rustls`, which `Cargo.toml:58` pins with `features = [..., "ring", ...]`.
So `ring` is in `podbox-extract`'s graph. Measured failure as uid 1000:

```
error occurred in cc-rs: command did not execute successfully (status code exit status: 2):
LC_ALL="C" "/work/scripts/zig-cc.sh" "-O0" ... "--target=x86_64-unknown-linux-musl" ...
"-c" ".../ring-0.17.14/crypto/curve25519/curve25519.c"
```

`podbox-probe` is the only crate in this set with neither `ring` nor a
bootstrap requirement, so it is the only one that isolates the uid question.

### Copy ownership: yes, non-root owns the copy, and it matters

Measured under `--user 1000:1000 --workspace`: `drwxr-xr-x 5 1000 1000 4096 /work`,
every file `1000 1000`, `touch /work/probe.txt` OK, `mkdir -p /work/target` OK,
`touch /work/target/x` OK, and after a real build `/work/target` is still
`1000 1000` and 583 MB.

Under root the same copy is `0 0`. That is not cosmetic, and the mixing fails:

```
error: Permission denied (os error 13) at path "/work/targetmIQ8rr"
```

which is a uid 1000 cargo refused to write into a `/work` that a root-run job
had created. ⛔ **A lane must not share one workspace directory between a root
job and a non-root job.** `--workspace` copying per job is what makes them
safe; a `base grant` mount would not be, and the grant is refused anyway.

### The mknod arm: root and non-root are the SAME here

This is the finding that reorders the whole question.

As **root**, in this lane:

```
$ mknod /tmp/t/dev/null c 1 3
mknod: /tmp/t/dev/null: Operation not permitted
MKNOD_RC=1
```

`/proc/self/status` under root: `CapEff: 00000000800405fb`, and decoding it:

```
CAP_EFF=800405fb
CAP_MKNOD(27): CLEAR
CAP_SYS_ADMIN(21): CLEAR
--- mknod under uid 0 here ---
mknod: /dev/shm/mt/n: Operation not permitted
SHM_MKNOD_FAIL
--- inside unshare -r ---
0
CapEff:	000001ffffffffff
mknod: /tmp/u/n: Operation not permitted
UNSHARE_MKNOD_FAIL
```

⭐ Even inside `unshare -r`, where uid 0 holds ALL capabilities
(`0x1ffffffffff`), `mknod` is still refused. The denial is therefore BELOW the
capability layer, in the kernel's device-node policy for this host and
filesystem, and no uid and no capability set reaches it.

So both lanes select the shim arm, and both pass. Measured, uid 0 and uid 1000,
the same script:

```
== kernel arm as uid 0
ARM=shim (mknod refused)
DEV_RC=0
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 40 filtered out
ALL_RC=0
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

and the 11 `devices::` tests by name, run as uid 1000, all `ok`:

```
absent_fd_spellings_are_staged_as_conventional_links, a_removable_link_is_displaced_and_named,
foreign_shapes_at_fd_spellings_refuse_by_name, an_unremovable_link_is_a_named_refusal,
a_grown_dev_null_is_truncated_and_the_growth_is_reported, a_second_pass_leaves_the_filled_shims_alone,
every_shim_is_reported_as_a_degradation_and_names_the_errno,
a_directory_at_a_device_path_is_a_named_failure, a_symlink_at_a_device_path_is_replaced_and_named,
a_fifo_at_a_device_path_is_replaced_and_named, what_each_row_claims_is_what_is_on_disk
```

⭐ Note what this means for the two candidate tests the audit pointed at.
`crates/podbox-complete/src/devices.rs` lines 603 and 629 are `assert!(f.degraded)`
and `assert_eq!(f.action, Action::Rewrote)` INSIDE `a_symlink_at_a_device_path_is_replaced_and_named`
(line 591) and `a_fifo_at_a_device_path_is_replaced_and_named` (line 615). Both
pass under root here. Neither is a root-fails test on this host. What makes
them interesting is `displace_link` running before `mknod`, and
`what_each_row_claims_is_what_is_on_disk` (line 517) which is written by this
project to be arm-independent on purpose.

### `podbox-ssh`: a missing-tool problem, as the audit said, and it is not fixable non-root

As uid 1000 with zig installed, so the crate does build:

```
test result: ok. 101 passed; 0 failed ... finished in 30.02s
test result: FAILED. 14 passed; 1 failed; 0 ignored ... finished in 2.19s

thread 'three_authenticated_ssh_sessions_share_one_node_connection' (2930) panicked at crates/podbox-ssh/tests/common.rs:41:14:
e2e needs "sshd" on PATH and it is absent; cannot prove the SSH path
```

`sshd` is absent and cannot be installed: the image has no `sudo`, and the
harness also needs `mkdir -p /run/sshd` which is
`mkdir: cannot create directory '/run/sshd': Permission denied`. 113 of 114
tests pass non-root. The one failure names the binary and nothing about uid.

## C. Is there a better non-root form than `--user 1000:1000`?

**No. `--user 1000:1000` is the only form that works, and it is the right one.**

- ⛔ `--user toolkit` does not work on this image. `toolkit` is the account in
  the BASE distro, not in the job image: `Error: unable to find user toolkit:
  no matching entries in passwd file`, exit 2, container never started. The
  manual says "a name, a uid, or uid:gid", which is true of the mechanism and
  misleading about which images carry the name. Use the uid.
- ⚠ `base exec` is non-root by default, confirmed (`id -u` -> 1000,
  `id -un` -> toolkit), and it is the wrong tool here. Its environment is
  cleared and the base carries no toolchain: `CARGO_HOME` is empty,
  `command -v cargo` -> NO_CARGO, `command -v rustc` -> NO_RUSTC. `/root` is
  `drwxr-x--- root root` and `~` holds only dotfiles plus `.wsl-toolkit`. So a
  `base exec` lane needs its own rustup install before cargo runs. `distro
  new/run/enter --user` has the same problem plus `--user` defaulting to root.
- ⛔ `base grant` is refused outright and I did not widen it:
  `wsl-toolkit: base.mounts requires base.automount to be "off", so ungranted
  Windows drives are absent`, exit 2. `wsl-toolkit --instance podbox config
  --effective --json` has NO `mounts` key at all, and `base exec` reports
  `ls: cannot access '/workspaces': No such file or directory`. So the
  podbox checkout is not granted, and per the skill's rule I am reporting the
  refusal rather than widening `automount` to make a step pass. Flipping
  `base.automount` to `off` is a config change to a shared instance and is the
  operator's call, not mine.
- ⭐ `matrix --user` works and is the form to use for a fleet. Two rows at
  `--parallel 2` on alpine and debian12, both uid 1000, both `SHIM`, both exit
  0, `2 ran, 0 failed, 0 unreached, 0 timed out, in 3s`.

## D. What the non-root lane costs: nothing measurable

Same script (`timed.sh`), same image, same staged workspace, cold cargo cache,
`podbox-probe` + `podbox-complete` end to end. Wall time from a host-side
`Stopwatch` around the whole `wsl-toolkit run`, exit code read from the process.

| lane | uid | cold build+test | warm re-run | wall | exit |
| --- | --- | --- | --- | --- | --- |
| non-root | 1000 | 33 s | 0 s | 48.10 s | 0 |
| root | 0 | 34 s | 0 s | 52.95 s | 0 |

Non-root is 4.85 s FASTER on wall here, which is inside the noise of two
samples; the honest reading is ⭐ **no measurable cost either way.** Both build
583 MB of `/work/target` and both pass 159 tests.

The costs that are real and are not time:

- ⚠ ~53 MB and ~25 s per job for zig, and it cannot be shared across jobs
  because `--workspace` copies a fresh tree each time (proven in section A).
- ⚠ `--max-bytes` defaults to 1 GiB. The podbox checkout is 3476 MB, 2391 MB of
  it `target`. A full copy of the checkout is refused. A lane must exclude
  `target`, `references` and `experiments`, or stage a build subset.
- ⭐ `--parallel` defaults to 4 and `matrix` needs no flag to use it. Six to
  nineteen agents at the default is four waves, which at ~50 s per agent is a
  few minutes. ⛔ Do NOT raise it past the CPU count: `wsl-toolkit inspect`
  reports `cgroups v2, cgroupfs, NOT delegated: no cgroup per container, so no
  limits and no attribution`, so parallel rows contend for the same cores with
  nothing to account for them.

## E. `wslc` for symmetry: still wedged

`wslc 3.0.1.0` at `C:\Program Files\WSL\wslc.exe`. `--version` and
`run --help` answer, so the binary is present and its CLI parses. Every actual
container operation fails:

```
$ wslc run -u 1000:1000 docker.io/library/alpine:latest id
wslc.exe : The process cannot access the file because it is being used by another process.
Error code: ERROR_SHARING_VIOLATION
PS_EXIT=1

$ wslc ps
wslc.exe : The process cannot access the file because it is being used by another process.
Error code: ERROR_SHARING_VIOLATION
PS_EXIT=1
```

`ps` failing too says the wedge is in the shared state `wslc` opens, not in
the run path specifically. ⛔ I did not attempt any repair. `wslc -u` is
therefore untested and the comparison against `--user 1000:1000` stands
unmeasured.

## What I could not settle

- ⛔ **Whether `wslc run -u 1000:1000` behaves like `--user`.** Blocked on the
  wedge. What would settle it: the wedge clearing on its own after the
  offending job's handle is released, or a host reboot. Neither is mine to do,
  and ⛔ `wsl --shutdown` is forbidden because it breaks every WSL instance.
- ⚠ **Whether a lane with `base.automount: off` plus `base grant --mode rw`
  would be better than `--workspace`.** The grant is refused under the current
  config. What would settle it: an operator decision to change
  `base.automount`, then measuring copy time against `--workspace` copy time.
  My expectation is `--workspace` wins on isolation (each job gets its own
  `/work`) and `grant` wins on repeat runs (no 31.4 MB copy), but that is a
  prediction and I did not measure it.
- ⚠ **The `podbox-ssh` e2e under non-root.** Blocked on `sshd`, which needs
  root to install. What would settle it: a job image that ships `sshd`, or a
  root bootstrap followed by a non-root run of the suite, which I did not
  attempt because it mixes the two uid models in one job.
- ⚠ **The whole-workspace suite non-root**, `cargo test --workspace`. I ran
  `podbox-probe`, `podbox-extract`, `podbox-complete` and `podbox-ssh`. The
  remaining crates (`podbox-enter`, `podbox-cli`, `podbox-windows`,
  `podbox-supervise`, `podbox-interpose`, `podbox-image`) were not run under a
  non-root uid. `podbox-interpose` is the one I would expect trouble from: CI
  builds it with `RUSTFLAGS: -C target-feature=-crt-static` on the gnu target
  (`.github/workflows/gate.yml`), and a C interposer build has its own
  requirements. What would settle it: one more `cargo test --workspace` under
  `--user 1000:1000` with zig installed, which is about five minutes.
- ⚠ **A lane wider than two rows.** Every `matrix` measurement here was two
  rows at `--parallel 2`. The 19-agent fleet is an extrapolation from that.
- ⚠ **Whether the `unshare -r` result generalises.** `mknod` refused inside a
  user namespace with all capabilities shows the refusal is not a capability
  question on THIS host and filesystem. I did not test another filesystem or
  another kernel, and `inspect` records the engine storage as
  `overlay on extfs` inside WSL, which is not a configuration a stock Linux CI
  runner has.

## Scope of this audit

Read: `scripts/common/bootstrap-env.sh` in full (391 lines),
`crates/podbox-complete/src/devices.rs` lines 440-734, `crates/podbox-ssh/tests/common.rs`
in full (205 lines), `scripts/zig-cc.sh` in full, `.cargo/config.toml` in full,
`.github/workflows/gate.yml` lines 180-215, `Cargo.lock` for `ring`,
`crates/podbox-{probe,extract,image,complete}/Cargo.toml`, plus
`wsl-toolkit man --no-pager` in full (822 lines, saved to `%TEMP%/wtk-man.txt`)
and `wsl-toolkit config --effective --json`.

Measured: 20 `wsl-toolkit` jobs, every exit code read from the process, never
through a pipe. Every `wsl-toolkit` invocation was launched through
`powershell -NoProfile -Command` so MSYS never rewrote a guest path, and no
`wsl.exe` was called with a payload.

Not run: `cargo test --workspace` under either uid, `cargo build --release`,
any `--target` other than the default musl, and every crate named in the
"could not settle" list above.

Cleanup: `wsl-toolkit --instance podbox gc --apply` exits 0 with
`ledger compacted to 0 open record(s)`, and `py scripts/check-todo.py` exits 0
with `check-todo: ok` and no `lane job still kept`. No job was left running.
⛔ No file in this repository was modified except this report, which is new.
The only other thing written was the throwaway staging tree
`C:\Users\AjamX\Downloads\wtk-staging` OUTSIDE the repository, which holds
copies of `.cargo`, `crates`, `scripts`, `Cargo.toml`, `Cargo.lock`,
`rust-toolchain.toml` and five probe scripts.
