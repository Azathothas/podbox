/* census.c — identity + mechanism census for one class of restricted
 * Linux runtime.
 *
 * Prints what the process can see of itself (identity block, maps,
 * seccomp state) and probes the operations the container/virtualization/
 * debugging stack is built on, one per freshly forked child, so a probe
 * that succeeds cannot leak its effect into the next one.
 *
 * The rule this implements: a probe must report the verdict of the
 * operation it names, in a disposable child, and distinguish "denied"
 * from "could not run".
 *
 * Build: cc -O2 -o census census.c
 * Exit:  0 census taken (denials are results, not failures)
 *        2 could not build/run
 */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <unistd.h>
#include <fcntl.h>
#include <sched.h>
#include <grp.h>
#include <signal.h>
#include <sys/mman.h>
#include <sys/mount.h>
#include <sys/prctl.h>
#include <sys/ptrace.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/sysmacros.h>
#include <sys/types.h>
#include <sys/wait.h>

static char *self;

/* run one probe in a forked child. The CHILD prints its own verdict
 * line, because errno lives per-process: a parent that reports the
 * errno for the child's syscall reports its own errno (0), which is
 * how a census ends up full of "DENIED (0 Success)" rows that measure
 * nothing. The child's exit status is still the verdict (0/1/2) so a
 * harness can gate on it without parsing. */
static void probe(const char *name, int (*fn)(void)) {
	fflush(stdout); fflush(stderr);
	pid_t p = fork();
	if (p < 0) { printf("%-38s FORK-FAILED %s\n", name, strerror(errno)); return; }
	if (p == 0) {
		errno = 0;
		int rc = fn();
		if (rc == 0)
			printf("%-38s OK\n", name);
		else if (errno == EPERM || errno == EACCES)
			printf("%-38s DENIED errno=%d (%s)\n", name, errno, strerror(errno));
		else
			printf("%-38s errno=%d (%s)%s\n", name, errno, strerror(errno),
			       (errno == EINVAL || errno == ENOENT || errno == EBADF || errno == EFAULT || errno == ESRCH)
				       ? "  <- executed, argument-shaped" : "");
		fflush(stdout);
		if (rc == 0) _exit(0);
		if (errno == EPERM || errno == EACCES) _exit(1);
		_exit(2);
	}
	int st = 0; waitpid(p, &st, 0);
	if (WIFSIGNALED(st))
		printf("%-38s KILLED-BY-SIGNAL %d\n", name, WTERMSIG(st));
	errno = 0;
}

static int p_unshare_ns(void)	 { return unshare(CLONE_NEWNS); }
static int p_unshare_user(void)	 { return unshare(CLONE_NEWUSER); }
static int p_unshare_net(void)	 { return unshare(CLONE_NEWNET); }
static int p_mount_tmpfs(void)	 { return mount("tmpfs", "/tmp", "tmpfs", 0, NULL); }
static int p_pivot_root(void)	 { return syscall(SYS_pivot_root, "/proc/self/no-such", "/proc/self/no-such"); }
static int p_ptrace_traceme(void){ return (long)ptrace(PTRACE_TRACEME, 0, 0, 0) < 0 ? 1 : 0; }
static int p_mknod_kvm(void) {
	/* a REAL device number. mknod(S_IFCHR, 0) is a whiteout and is
	 * exempt from the capability check — it measures nothing. */
	unlink("/tmp/.census-kvm");
	return mknod("/tmp/.census-kvm", S_IFCHR | 0600, makedev(10, 232));
}
static int p_mknod_whiteout(void) {
	int u = unlink("/tmp/.census-wh");
	if (u != 0 && errno != ENOENT)
		printf("  [whiteout fixture unlink: %s — measuring anyway]\n", strerror(errno));
	int r = mknod("/tmp/.census-wh", S_IFCHR | 0600, makedev(0, 0));
	if (r == 0) unlink("/tmp/.census-wh");	/* a fixture left behind poisons the next run with EEXIST */
	return r;
}
static int p_chown_unmapped(void) {
	int fd = open("/tmp/.census-chown", O_CREAT | O_WRONLY, 0644);
	if (fd < 0) return 1; close(fd);
	/* gid 42 = the shadow group an Alpine/Debian /etc/shadow ships with */
	int r = chown("/tmp/.census-chown", 0, 42);
	unlink("/tmp/.census-chown");
	return r;
}
static int p_setuid_nonzero(void){ return setuid(1000); }
static int p_setgroups(void)	 { return setgroups(0, NULL); }
static int p_proc_mem_rd(void)	 { return open("/proc/self/mem", O_RDONLY) < 0 ? 1 : 0; }
static int p_proc_mem_wr(void)	 { return open("/proc/self/mem", O_RDWR) < 0 ? 1 : 0; }
static int p_proc_other_mem(void){ return open("/proc/1/mem", O_RDONLY) < 0 ? 1 : 0; }
static int p_dev_dir(void)	 { return open("/dev", O_RDONLY | O_DIRECTORY) < 0 ? 1 : 0; }
static int p_dev_kvm(void)	 { return open("/dev/kvm", O_RDWR) < 0 ? 1 : 0; }
static int p_landlock(void)	 { return syscall(444 /* landlock_create_ruleset */, NULL, 0, 1 /* LANDLOCK_CREATE_RULESET_VERSION */) < 0 ? 1 : 0; }
static int p_fsopen(void)	 { return syscall(430 /* fsopen */, "tmpfs", 0) < 0 ? 1 : 0; }
static int p_openat2(void)	 { return syscall(437 /* openat2 */, -100 /* AT_FDCWD */, "/proc/self/no-such-census", NULL, 0) < 0 ? (errno == ENOENT ? 0 : 1) : 0; }
static int p_io_uring(void) {
	int fd = (int)syscall(425 /* io_uring_setup */, 4, NULL);
	if (fd >= 0) { close(fd); return 0; }
	return (errno == EFAULT || errno == EINVAL) ? 0 : 1; /* executed at all */
}
static int p_clone3_null(void)	 { return syscall(435 /* clone3 */, NULL, 0) < 0 ? (errno == EINVAL ? 0 : 1) : 0; }
static int p_mprotect_rwx(void) {
	void *m = mmap(NULL, 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
	if (m == MAP_FAILED) return 1;
	return mprotect(m, 4096, PROT_READ | PROT_WRITE | PROT_EXEC);
}
static int p_memfd(void)	 { return syscall(319 /* memfd_create */, "census", 0) < 0 ? 1 : 0; }
static int p_bpf(void)		 { return syscall(321, 0, NULL, 0) < 0 ? 1 : 0; }
static int p_keyctl(void)	 { return syscall(250, 0, 0, 0, 0, 0) < 0 ? 1 : 0; }
static int p_open_by_handle(void){ return syscall(304, -1, NULL, 0) < 0 ? 1 : 0; }
static int p_setns(void)	 { return syscall(308, -1, 0) < 0 ? (errno == EPERM ? 1 : 2) : 0; }
static int p_kcmp(void)		 { return syscall(312, -1, -1, 0, 0, 0) < 0 ? (errno == ESRCH ? 0 : (errno == ENOSYS ? 2 : 1)) : 0; }

static void cat(const char *path) {
	char buf[4096]; ssize_t n;
	int fd = open(path, O_RDONLY);
	if (fd < 0) { printf("%s: (%s)\n", path, strerror(errno)); return; }
	n = read(fd, buf, sizeof(buf) - 1);
	if (n > 0) { buf[n] = 0; printf("%s:\n%s", path, buf); }
	close(fd);
}

int main(int argc, char **argv) {
	self = argv[0];
	printf("## identity\n");
	printf("uid=%d gid=%d euid=%d\n", getuid(), getgid(), geteuid());
	cat("/proc/self/uid_map");
	cat("/proc/self/gid_map");
	cat("/proc/self/setgroups");
	{
		char line[256]; FILE *f = fopen("/proc/self/status", "r");
		while (f && fgets(line, sizeof(line), f))
			if (!strncmp(line, "CapEff:", 7) || !strncmp(line, "CapBnd:", 7) ||
			    !strncmp(line, "Seccomp:", 8) || !strncmp(line, "NoNewPrivs:", 11))
				fputs(line, stdout);
		if (f) fclose(f);
	}
	{
		char buf[256]; ssize_t n;
		int fd = open("/proc/self/cmdline", O_RDONLY); (void)fd;
		if ((n = readlink("/proc/self/ns/user", buf, sizeof(buf) - 1)) > 0) { buf[n] = 0; printf("user-ns: %s\n", buf); }
		if ((n = readlink("/proc/self/ns/mnt", buf, sizeof(buf) - 1)) > 0) { buf[n] = 0; printf("mnt-ns: %s\n", buf); }
	}

	printf("\n## mechanism census (each probe in its own child)\n");
	/* F candidates: syscall-number denials */
	probe("unshare(CLONE_NEWNS)", p_unshare_ns);
	probe("unshare(CLONE_NEWUSER)", p_unshare_user);
	probe("unshare(CLONE_NEWNET)", p_unshare_net);
	probe("mount(tmpfs, /tmp)", p_mount_tmpfs);
	probe("pivot_root(bogus)", p_pivot_root);
	probe("ptrace(PTRACE_TRACEME)", p_ptrace_traceme);
	probe("bpf(0)", p_bpf);
	probe("keyctl(0)", p_keyctl);
	probe("open_by_handle_at(bogus)", p_open_by_handle);
	probe("setns(bogus fd)", p_setns);
	/* N candidates: capability/mapping effects */
	probe("mknod(chr 10:232 kvm)", p_mknod_kvm);
	probe("mknod(chr 0:0 whiteout)", p_mknod_whiteout);
	probe("chown(f, 0, 42 unmapped)", p_chown_unmapped);
	probe("setuid(1000)", p_setuid_nonzero);
	probe("setgroups(0)", p_setgroups);
	/* M candidates: path-scoped / provenance effects */
	probe("open /proc/self/mem RD", p_proc_mem_rd);
	probe("open /proc/self/mem WR", p_proc_mem_wr);
	probe("open /proc/1/mem RD", p_proc_other_mem);
	probe("openat /dev (dir)", p_dev_dir);
	probe("open /dev/kvm", p_dev_kvm);
	/* gap candidates: modern interfaces vs the legacy calls they replace */
	probe("landlock_create_ruleset", p_landlock);
	probe("fsopen(tmpfs)", p_fsopen);
	probe("openat2(bogus path)", p_openat2);
	probe("io_uring_setup(0)", p_io_uring);
	probe("clone3(NULL)", p_clone3_null);
	probe("kcmp(-1,-1) [control]", p_kcmp);
	/* what is plainly available */
	probe("mprotect RWX", p_mprotect_rwx);
	probe("memfd_create", p_memfd);

	printf("\n## device and filesystem shape\n");
	{
		char buf[4096]; ssize_t n;
		int fd = open("/proc/misc", O_RDONLY);
		if (fd >= 0 && (n = read(fd, buf, sizeof(buf) - 1)) > 0) {
			buf[n] = 0;
			char *k = strstr(buf, " kvm"), *t = strstr(buf, " tun");
			printf("host-drivers: kvm=%s tun=%s (from /proc/misc; nodes are a separate question)\n",
			       k ? "present" : "absent", t ? "present" : "absent");
		}
		close(fd);
	}
	return 0;
}
