#!/usr/bin/env bash
# Question: cross-validation of Azathothas/container-research claims against
# THIS sandbox's measured behavior. Their model: N (userns, map 0 1000 1) +
# F (seccomp denylist: unshare setns mount umount2 pivot_root ptrace
# process_vm_readv/writev) + M (Landlock: write allowlist
# {/tmp,/dev/shm,/workspace,/state}, move_mount EPERM, /proc/pid/mem EACCES,
# detached-mount open EACCES). Claims tested here: F pivot_root,
# F process_vm_readv, M detached-mount open EACCES, M write allowlist
# membership per path. My 60- census already confirmed: unshare/setns/mount
# EPERM, /proc/self/mem EACCES, fsopen/fsmount OK, clone NEWUSER OK.
# Exit codes: 0 all probes ran, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/77-container-research-crossval.log"
{ echo "## conditions"; conditions container-research-crossval; } > "$LOG"

cat > "$VR_WORK/xval.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <errno.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/mount.h>
#include <sys/syscall.h>
#include <sys/uio.h>
int main(void){
  char err[128];
  /* F: pivot_root (theirs: EPERM) — needs a mount; expect seccomp EPERM first */
  errno=0;
  if (syscall(SYS_pivot_root, "/tmp", "/tmp/old") < 0) {
    snprintf(err,sizeof err,"%s",strerror(errno));
    printf("pivot_root: %s\n", err);
  } else printf("pivot_root: OK (UNEXPECTED)\n");
  /* F: process_vm_readv on self (theirs: EPERM) */
  errno=0;
  char local[16]; struct iovec l = { .iov_base=local, .iov_len=16 };
  if (syscall(SYS_process_vm_readv, getpid(), &l, 1, NULL, 0, 0) < 0) {
    snprintf(err,sizeof err,"%s",strerror(errno));
    printf("process_vm_readv(self): %s\n", err);
  } else printf("process_vm_readv(self): OK (UNEXPECTED)\n");
  /* M: fsmount object → openat (theirs: detached mounts creatable, openat → EACCES) */
  int fs = (int)syscall(SYS_fsopen, "tmpfs", 0);
  if (fs < 0){ printf("fsopen: %s\n", strerror(errno)); return 0; }
  if (syscall(SYS_fsconfig, fs, FSCONFIG_CMD_CREATE, 0, 0, 0) < 0){ printf("fsconfig: %s\n", strerror(errno)); return 0; }
  int fm = (int)syscall(SYS_fsmount, fs, 0, 0);
  printf("fsmount fd=%d\n", fm);
  if (fm >= 0){
    /* try to open the mount ROOT through the mount fd (AT_EMPTY_PATH-ish via openat) */
    int o = openat(fm, ".", O_RDONLY);
    printf("openat(fsmount fd, \".\"): %s\n", o<0?strerror(errno):"OK");
    close(o);
    /* attach attempt via move_mount from the fsmount fd */
    long r = syscall(SYS_move_mount, fm, "", AT_FDCWD, "/workspace", MOVE_MOUNT_F_EMPTY_PATH);
    printf("move_mount(fsmount -> /workspace): %s\n", r<0?strerror(errno):"OK");
  }
  /* M: write allowlist — test each candidate path (open O_WRONLY|O_CREAT on existing dirs) */
  const char *paths[] = {"/tmp/vmr75","/dev/shm/vmr75","/workspace/vm-research/experiments/work/vmr75","/state/vmr75","/var/tmp/vmr75","/etc/vmr75","/root/vmr75","/usr/vmr75"};
  for (unsigned i=0;i<sizeof paths/sizeof*paths;i++){
    int fd = open(paths[i], O_WRONLY|O_CREAT, 0644);
    printf("write-open %s: %s\n", paths[i], fd<0?strerror(errno):"OK");
    if (fd>=0){ close(fd); unlink(paths[i]); }
  }
  return 0;
}
EOF
gcc -O0 -o "$VR_WORK/xval" "$VR_WORK/xval.c" 2>> "$LOG" || { echo "build failed" >> "$LOG"; exit 2; }
"$VR_WORK/xval" >> "$LOG" 2>&1

# M write-allowlist via shell on file/dir targets that need dirs
for p in /tmp /dev/shm /workspace /state /var/tmp /etc /usr; do
  if [ -d "$p" ]; then
    if touch "$p/.vmr75w" 2>/dev/null; then echo "write $p: OK"; rm -f "$p/.vmr75w"; else echo "write $p: DENIED"; fi
  else
    echo "write $p: (absent)"
  fi
done >> "$LOG"
tail -24 "$LOG"
[ -f "$LOG" ] && exit 0 || exit 2
