# Failure field guide

Use the discriminator before applying the action. Similar messages arise from
different planes.

| Symptom | Likely mechanism | Discriminator | Action |
|---|---|---|---|
| uid 0 + all capability bits, operation denied | capability scoped to a user namespace | read uid/gid maps and namespace owners | reason about the namespace named by the kernel check |
| `chown ... Invalid argument` | target uid/gid unmapped | try 0:0 and inspect `gid_map` | ownership-neutral extraction + sidecar |
| Go `fork/exec ... operation not permitted` | child `setgroups`, not missing executable | remove `Credential` or set `NoSetGroups` in a controlled matrix | stop changing clone flags blindly |
| `unshare(CLONE_NEWUSER)` gives `EINVAL` in Go | multithreaded-prober artifact | single-threaded C/Rust helper or `clone` | replace the witness |
| char `mknod` 0:0 succeeds, real device fails | whiteout special case | pair 0:0 with 1:3 at same path | real device is not creatable |
| `mount` EPERM but `fsopen` succeeds | legacy syscall filtered; creation path open | bogus-path call + `fsmount`/`move_mount` stages | do not infer attachment support |
| `fsmount` succeeds, `move_mount` EPERM | mount object creatable, attachment denied later | bogus destination returns path-shaped errno | stop searching for another create path |
| `openat` on detached mount fd gives EACCES | path-resolving LSM cannot resolve detached tree | compare DAC-permitted metadata | detached mount is unusable here |
| TCP bind EPERM, UDP bind OK | protocol-aware network policy | matrix by family/address/port | Unix/FIFO control; UDP data |
| loopback connect EPERM, LAN closed port ECONNREFUSED | address-aware network policy | same syscall and port, different address | do not blame seccomp alone |
| external `:80` changes from EPERM to OK | live policy reconfiguration | two stacks, repeated controls, timestamped logs | expire cached network verdicts |
| `/dev/kvm` ENOENT though host driver is listed | device node withheld | close create/mount/receive-fd paths | use TCG or fail a KVM requirement |
| VM start exits 153 / `SIGXFSZ` | per-file limit, often memory backing | inspect `RLIMIT_FSIZE`, actual created files | keep memory/disk files below ceiling |
| `short write: -20 != N` | libarchive status, underlying ENOSPC | inspect library errno and destination capacity | move staging off small `/tmp` |
| output disappears or tools appear hung | full tmpfs or blocked notification | check blocks/inodes; use timeouts | preserve logs outside tmpfs |
| tracer artifacts exist but command status is 139 | tracer/tracee fault after semantic intervention | assert tracer exit separately from artifacts | preserve intended exit status; keep fault visible |
| seccomp notification listener works but dry-run changes files | argument-read leg failed, handler continued | preflight `/proc/pid/mem` or equivalent | refuse enforcement tier; explicit passthrough only |
| file report misses most GUI/static workload | relative paths or inherited fds omitted | run dirfd-relative and exec-inheritance fixtures | resolve in tracee context; scan post-exec fds |
| seccomp-emulated `openat` uses wrong directory | tracee dirfd mistaken for tracer fd | resolve `/proc/<pid>/fd/<n>` and tracee cwd | continue rather than guess if unresolved |
| tracee hangs at high process count | notification dropped on state-pool exhaustion | stress beyond pool and assert progress | answer CONTINUE/error; never drop a notification |
| `TracerPid` hidden but `wchan` says `ptrace_stop` | partial anti-debug coverage | test status, wchan, scalar and vector reads | feature-specific, length-preserving rewrites |
| vector read evades rewrite | marker split across iovecs | crafted split fixture | carry a seam window across buffers |
| chrooted command says `/bin/sh` missing though present | executable resolved before root switch or wrong root derived | resolve after `chroot`; print root identity | preserve needed environment and order operations |
| shell creates a growing `/dev/null` | node absent, redirection used `O_CREAT` | inspect file type before workload | deliberate shim or refuse device-dependent payload |
| compiler finds driver but not `cc1`/`crtbegin` | tool prefix derived from `argv[0]` | compare `gcc` with `/usr/bin/gcc` | use absolute compiler path |
| Alpine network device absent in guest | `virtio_net` is a module with dependencies | `modprobe` with full module tree | do not use bare `insmod` |
| Firecracker initramfs reaches kernel but PID 1 is silent | empty `/dev`, no console node | inspect archive; append console cpio | add `/dev/console` without host `mknod` |
| `-M microvm` shows printk but no shell output | userspace serial/machine artifact mismatch | boot same artifacts on `-M pc` | use pc profile for interactive control |
| snapshot seems slow or fast from host wall time | boundary includes process/log overhead | guest uptime at snapshot and first resume tick | report bounded guest-time delta and tick precision |
| lifecycle occasionally fails after a fixed sleep | readiness race | nonce/readiness fd and repeated loop | replace sleeps with events |
| test suite says pass although feature flag is unusable | option/path not reached by tests | invoke documented surface from CLI | test reachability, not only internal handler |
| review says defect fixed but comments/manual disagree | documentation drift | compare code, current manual, and tests | record OPEN until reconciled |
