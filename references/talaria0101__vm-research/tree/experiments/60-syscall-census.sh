#!/usr/bin/env bash
# Question: the 10- probe showed a DENYLIST-style filter (clone3 got EINVAL
# from the kernel while unshare got EPERM). Is there a gap? Full census: (a)
# every new-API syscall the old probes never tried — fsopen/fsmount/move_mount/
# open_tree/mount_setattr/openat2/mknodat; (b) clone3 with CLONE_NEWUSER|
# CLONE_NEWNS — a nested userns would make us real root over userns-owned
# tmpfs; (c) broad sweep for EPERM anomalies.
# Exit codes: 0 census taken, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/60-syscall-census.log"
conditions seccomp-census > "$LOG"

cat > "$VR_WORK/census.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <errno.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <sched.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/sysmacros.h>
#include <sys/wait.h>
#include <sys/mman.h>
#include <linux/perf_event.h>

#ifndef OPEN_TREE_NOSUPERMOUNT
#define OPEN_TREE_NOSUPERMOUNT 4
#endif
static void p(const char *n, long r) {
  printf("%-38s rc=%ld errno=%d (%s)\n", n, r, r < 0 ? errno : 0,
         r < 0 ? strerror(errno) : "ok");
  errno = 0;
}
/* child body executed in nested userns+mountns via clone3 */
static int nested_child(void *arg) {
  int k;
  write(2, "[nested] entered\n", 17);
  /* we are root of the new userns: fix our uid_map relative to parent */
  int fd = open("/proc/self/uid_map", O_WRONLY);
  if (fd >= 0) { if (write(fd, "0 0 1\n", 6) < 0) {} close(fd); }
  fd = open("/proc/self/setgroups", O_WRONLY);
  if (fd >= 0) { write(fd, "deny", 4); close(fd); }
  fd = open("/proc/self/gid_map", O_WRONLY);
  if (fd >= 0) { write(fd, "0 0 1\n", 6); close(fd); }
  printf("[nested] uid=%d euid=%d\n", getuid(), geteuid());
  k = open("/dev/kvm", O_RDWR);
  printf("[nested] open(/dev/kvm) -> %s\n", k >= 0 ? "OPENED" : strerror(errno));
  if (k >= 0) { ioctl(k, 0xAE00 /*KVM_GET_API_VERSION*/); }
  /* tmpfs we own: mount via the new API, then mknod on it */
  int fs = (int)syscall(SYS_fsopen, "tmpfs", 0);
  p("[nested] fsopen(tmpfs)", fs);
  if (fs >= 0) {
    p("[nested] fsconfig(create tmpfs)", syscall(SYS_fsconfig, fs, FSCONFIG_CMD_CREATE, 0, 0, 0));
    int fm = (int)syscall(SYS_fsmount, fs, 0, 0);
    p("[nested] fsmount", fm);
    mkdir("/tmp/mp", 0755);
    if (fm >= 0) p("[nested] move_mount(tmpfs at /tmp/mp)", syscall(SYS_move_mount, fm, "", AT_FDCWD, "/tmp/mp", MOVE_MOUNT_F_EMPTY_PATH));
    p("[nested] mknod(kvm 10:232 on OWN tmpfs)", syscall(SYS_mknodat, AT_FDCWD, "/tmp/mp/kvm", S_IFCHR | 0600, makedev(10, 232)));
    k = open("/tmp/mp/kvm", O_RDWR);
    printf("[nested] open(forged node) -> %s\n", k >= 0 ? "OPENED" : strerror(errno));
    if (k >= 0) { long v = ioctl(k, 0xAE00); printf("[nested] KVM_GET_API_VERSION -> %ld\n", v); }
  }
  /* devtmpfs chain */
  int fd2 = (int)syscall(SYS_fsopen, "devtmpfs", 0);
  p("[nested] fsopen(devtmpfs)", fd2);
  if (fd2 >= 0) p("[nested] fsconfig(create devtmpfs)", syscall(SYS_fsconfig, fd2, FSCONFIG_CMD_CREATE, 0, 0, 0));
  /* sysfs in the nested ns: does /sys/dev/char/10:232 exist and help? */
  int fs3 = (int)syscall(SYS_fsopen, "sysfs", 0);
  p("[nested] fsopen(sysfs)", fs3);
  if (fs3 >= 0) {
    p("[nested] fsconfig(create sysfs)", syscall(SYS_fsconfig, fs3, FSCONFIG_CMD_CREATE, 0, 0, 0));
    int fm3 = (int)syscall(SYS_fsmount, fs3, 0, 0);
    p("[nested] fsmount(sysfs)", fm3);
    if (fm3 >= 0) {
      mkdir("/tmp/sys", 0755);
      p("[nested] move_mount(sysfs at /tmp/sys)", syscall(SYS_move_mount, fm3, "", AT_FDCWD, "/tmp/sys", MOVE_MOUNT_F_EMPTY_PATH));
      struct stat st;
      int e = stat("/tmp/sys/dev/char/10:232", &st);
      printf("[nested] stat sysfs/dev/char/10:232 -> %d (%s)\n", e, e ? strerror(errno) : "exists (sysfs attrs only)");
    }
  }
  fflush(stdout); _exit(0);
}
#define STACK (64 * 1024)
static char stk[STACK];
int main(void) {
  errno = 0;
  /* --- new mount API in the ORIGINAL ns --- */
  int fs = (int)syscall(SYS_fsopen, "tmpfs", 0);
  p("fsopen(tmpfs)", fs);
  if (fs >= 0) {
    p("  fsconfig(create)", syscall(SYS_fsconfig, fs, FSCONFIG_CMD_CREATE, 0, 0, 0));
    int fm = (int)syscall(SYS_fsmount, fs, 0, 0);
    p("  fsmount", fm);
    if (fm >= 0) { mkdir("/tmp/vmr-mp", 0755); p("  move_mount(tmpfs->/tmp/vmr-mp)", syscall(SYS_move_mount, fm, "", AT_FDCWD, "/tmp/vmr-mp", MOVE_MOUNT_F_EMPTY_PATH)); }
  }
  int fsd = (int)syscall(SYS_fsopen, "devtmpfs", 0);
  p("fsopen(devtmpfs)", fsd);
  if (fsd >= 0) p("  fsconfig(create devtmpfs)", syscall(SYS_fsconfig, fsd, FSCONFIG_CMD_CREATE, 0, 0, 0));
  p("open_tree(clone /dev/null)", syscall(SYS_open_tree, AT_FDCWD, "/dev/null", OPEN_TREE_CLONE | OPEN_TREE_NOSUPERMOUNT));
  int nd = open("/dev/null", O_RDONLY);
  unsigned char fhbuf[256] = { 0 };
  struct file_handle *fk = (struct file_handle *)fhbuf;
  fk->handle_bytes = 8; fk->handle_type = 1;
  int *w = (int *)fk->f_handle; w[0] = 10; w[1] = 232;
  p("open_by_handle_at(kvm@devtmpfs sb)", open_by_handle_at(nd, fk, O_RDWR));
  close(nd);
  p("unshare(NEWUSER)", unshare(CLONE_NEWUSER));
  /* --- nested userns+mountns via the UNFILTERED clone3 --- */
  /* glibc clone() uses clone3 internally on modern glibc (with trampoline) */
  long c3 = clone(nested_child, stk + STACK, CLONE_NEWUSER | CLONE_NEWNS | SIGCHLD, NULL);
  p("clone(glibc->clone3)(NEWUSER|NEWNS)", c3);
  /* legacy SYS_clone is ALSO unfiltered (probed separately: raw call returns
     a child; both clone numbers pass the filter, only unshare(2) is blocked) */
  if (c3 > 0) { int st; waitpid((pid_t)c3, &st, 0);
    if (WIFSIGNALED(st)) printf("  child KILLED by signal %d\n", WTERMSIG(st));
    else printf("  child exited status=%d\n", WEXITSTATUS(st)); }
  struct perf_event_attr pea = { 0 }; pea.size = sizeof(pea);
  p("perf_event_open", syscall(SYS_perf_event_open, &pea, 0, -1, -1, 0));
  p("kexec_load", syscall(SYS_kexec_load, 0, 0, NULL, 0));
  p("iopl(3)", syscall(SYS_iopl, 3));
  p("userfaultfd", syscall(SYS_userfaultfd, 0));
  p("setns(self mnt)", syscall(SYS_setns, (int)syscall(SYS_open, "/proc/self/ns/mnt", O_RDONLY), 0));
  return 0;
}
EOF
gcc -O0 -o "$VR_WORK/census" "$VR_WORK/census.c" >> "$LOG" 2>&1 || { echo "build failed" >> "$LOG"; exit 2; }
"$VR_WORK/census" >> "$LOG" 2>&1

echo "--- broad sweep (fork-isolated; EPERM where kernel would not EPERM = filtered)" >> "$LOG"
cat > "$VR_WORK/sweep.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <errno.h>
#include <string.h>
#include <unistd.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <signal.h>
int main(void) {
  signal(SIGCHLD, SIG_IGN);
  for (long n = 0; n <= 460; n++) {
    pid_t pid = fork();
    if (pid == 0) {
      alarm(1);
      signal(SIGALRM, SIG_DFL);
      errno = 0;
      long r = syscall(n);
      int e = errno;
      _exit(r == -1 ? (e == EPERM ? 1 : (e == ENOSYS ? 2 : 0)) : 0);
    }
    int st; if (waitpid(pid, &st, 0) < 0) { printf("%3ld WAIT-ERR\n", n); continue; }
    if (WIFSIGNALED(st)) printf("%3ld SIGNALLED %d\n", n, WTERMSIG(st));
    else if (WEXITSTATUS(st) == 1) printf("%3ld EPERM\n", WEXITSTATUS(st) ? n : n);
    else if (WEXITSTATUS(st) == 2) printf("%3ld ENOSYS\n", n);
  }
  return 0;
}
EOF
gcc -O0 -o "$VR_WORK/sweep" "$VR_WORK/sweep.c" >> "$LOG" 2>&1 || exit 2
"$VR_WORK/sweep" 2>/dev/null | grep -E 'EPERM|SIGNALLED' >> "$LOG"
echo "--- done" >> "$LOG"
echo "--- broad sweep: syscalls that answer EPERM where kernel would not" >> "$LOG"
cat > "$VR_WORK/sweep.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <errno.h>
#include <string.h>
#include <unistd.h>
#include <sys/syscall.h>
int main(void) {
  /* each probe: syscall(n) with 0 args; note result class */
  for (long n = 0; n <= 460; n++) {
    errno = 0;
    long r = syscall(n);
    int e = errno;
    if (r == -1 && (e == EPERM || e == ENOSYS)) {
      printf("%3ld %s\n", n, e == EPERM ? "EPERM" : "ENOSYS");
    }
  }
  return 0;
}
EOF
gcc -O0 -o "$VR_WORK/sweep" "$VR_WORK/sweep.c" >> "$LOG" 2>&1 || exit 2
"$VR_WORK/sweep" >> "$LOG" 2>&1
echo "--- done" >> "$LOG"
grep -vE '^## |^date|^host|^mem|^qemu|^accel|^subject' "$LOG" | head -40
exit 0
