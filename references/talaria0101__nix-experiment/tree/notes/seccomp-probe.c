#define _GNU_SOURCE
#include <stdio.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>
#include <sys/syscall.h>
#include <sched.h>
#include <sys/mount.h>
#include <sys/xattr.h>
#include <fcntl.h>
#include <signal.h>
#include <grp.h>
#include <sys/stat.h>
#include <sys/ptrace.h>

static void t(const char*name,int r){printf("%-28s %s (errno=%s)\n",name,r==0?"OK":"FAIL",strerror(errno));}
int main(){
  int r=(int)syscall(SYS_unshare,CLONE_NEWUSER); t("unshare(CLONE_NEWUSER)",r==-1?-1:0);
  r=(int)syscall(SYS_unshare,CLONE_NEWNS); t("unshare(CLONE_NEWNS)",r==-1?-1:0);
  r=mount("t","/tmp","tmpfs",0,NULL); t("mount(tmpfs /tmp)",r==-1?-1:0);
  r=chown("/tmp/seccomp-probe.c",100,100); t("chown(file,100,100)",r==-1?-1:0);
  r=(int)syscall(SYS_setuid,65534); t("setuid(65534)",r==-1?-1:0);
  r=(int)syscall(SYS_setgroups,0,NULL); t("setgroups(0)",r==-1?-1:0);
  r=mknod("/tmp/probe-node",S_IFCHR|0600,0); t("mknod(char dev)",r==-1?-1:0);
  r=setxattr("/tmp/seccomp-probe.c","user.probe","x",1,0); t("setxattr(user)",r==-1?-1:0);
  r=(int)syscall(SYS_keyctl,0,0,0,0,0); t("keyctl(0,...)",r==-1?-1:0);
  r=(int)syscall(SYS_bpf,0,0,0); t("bpf(...)",r==-1?-1:0);
  r=(int)syscall(SYS_perf_event_open,0,0,0,0,0); t("perf_event_open",r==-1?-1:0);
  r=ptrace(PTRACE_TRACEME,0,0,0); t("ptrace(TRACEME)",r==-1?-1:0);
  return 0;
}
