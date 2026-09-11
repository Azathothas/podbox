#!/usr/bin/env bash
# Question: what virtualization-relevant kernel facilities does this sandbox
# actually expose (KVM, TUN, mount/unshare/ptrace), and by which mechanism is
# each denied — seccomp filter, user-namespace scope, or absent device node?
#
# Every claim in the verdict matrix depends on this census; it is experiment
# number 10 because everything after it interprets its output.
#
# Inputs: none (pure runtime probe). Tools: gcc, coreutils.
# Exit codes: 0 probe ran (always, denials are the *result*),
#             2 could not run (no compiler).
set -u
cd "$(dirname "$0")"
LOG="logs/10-probe-environment.log"
mkdir -p logs
{
  echo "## conditions"
  echo "date: $(date -u +%FT%TZ)"
  echo "host: $(uname -srmo)"
  echo "uid/gid: $(id -u)/$(id -g)   caps_eff: $(grep CapEff /proc/self/status | awk '{print $2}')"
  echo "seccomp: $(grep Seccomp /proc/self/status | awk '{print $2}') (2=filter mode)"
  echo "uid_map: $(tr '\n' ' ' < /proc/self/uid_map)"
  echo "namespaces: $(ls -l /proc/self/ns/ | awk '{print $9" -> "$11}' | tr '\n' ' ')"
  echo
  echo "## static census"
  echo "devkvm-node: $(ls -l /dev/kvm 2>&1 | head -1)"
  echo "devtun-node: $(ls -l /dev/net/tun 2>&1 | head -1)"
  echo "dev-dir:     $(ls /dev 2>/dev/null | sort | tr '\n' ' ')"
  echo "proc-misc-kvm: $(grep -w kvm /proc/misc 2>/dev/null || echo ABSENT)"
  echo "proc-misc-tun: $(grep -w tun /proc/misc 2>/dev/null || echo ABSENT)"
  echo "proc-modules: $(grep -E '^(kvm|kvm_amd|kvm_intel|tun) ' /proc/modules 2>/dev/null | awk '{print $1,$2}' | tr '\n' '; ')"
  echo "cpu-flags-virt: $(grep -m1 flags /proc/cpuinfo | tr ' ' '\n' | grep -cE '^(vmx|svm)$') matched"
  echo "hypervisor-flag: $(grep -c hypervisor /proc/cpuinfo) (0 = we are on bare metal, inside a container on it)"
  echo
  echo "## dynamic census (syscall probe; EPERM with full caps => seccomp filter)"
} > "$LOG"

cat > /tmp/vmr_probe.c <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>
#include <fcntl.h>
#include <sched.h>
#include <sys/stat.h>
#include <sys/mount.h>
#include <sys/mman.h>
#include <sys/ioctl.h>
#include <sys/syscall.h>
#include <sys/sysmacros.h>
#include <sys/ptrace.h>
static void t(const char*name,int rc){printf("%-24s rc=%-4d errno=%d (%s)\n",name,rc,errno,rc<0?strerror(errno):"ok");errno=0;}
int main(void){
  t("mknod(chr 10:232)",mknod("/tmp/vmr_kvm",S_IFCHR|0600,makedev(10,232)));unlink("/tmp/vmr_kvm");
  t("mount(tmpfs)",mount("tmpfs","/tmp","tmpfs",0,NULL));
  t("umount2",umount2("/tmp",MNT_DETACH));
  t("unshare(CLONE_NEWNS)",unshare(CLONE_NEWNS));
  t("unshare(CLONE_NEWUSER)",unshare(CLONE_NEWUSER));
  t("ptrace(TRACEME)",ptrace(PTRACE_TRACEME,0,NULL,NULL));
  t("open(/proc/self/mem)",open("/proc/self/mem",O_RDWR));
  t("open(/dev/kvm)",open("/dev/kvm",O_RDWR));
  t("open(/dev/net/tun)",open("/dev/net/tun",O_RDWR));
  t("mmap+RWX mprotect",mprotect(mmap(NULL,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0),4096,PROT_READ|PROT_WRITE|PROT_EXEC));
  t("mmap(512MiB anon)",(int)(long)mmap(NULL,512L<<20,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0));
  errno=0;t("bpf",syscall(SYS_bpf,0,NULL,0));
  errno=0;t("keyctl",syscall(SYS_keyctl,0,0,0,0,0));
  errno=0;t("open_by_handle_at",syscall(SYS_open_by_handle_at,0,NULL,0));
  errno=0;t("clone3(NULL)",syscall(SYS_clone3,NULL,0));
  return 0;
}
EOF
gcc -O0 -o /tmp/vmr_probe /tmp/vmr_probe.c >> "$LOG" 2>&1 || { echo "cannot build probe" >> "$LOG"; exit 2; }
/tmp/vmr_probe 2>&1 | tee -a "$LOG"
echo
echo "log: $LOG"
exit 0
