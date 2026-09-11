/* attribute.c — the bogus-argument discriminator.
 *
 * A seccomp filter sees the syscall number and six argument registers.
 * It cannot dereference a pointer, and it runs before the syscall body.
 * So ONE call separates "a filter refused this" from "the kernel refused
 * this": call the syscall with an argument the kernel would reject
 * inside its own body —
 *
 *   a path- or id-shaped errno (ENOENT, EBADF, ESRCH)  -> executed
 *   EPERM for the same argument                        -> refused pre-entry
 *
 * Controls are carried so a probe that has stopped discriminating says
 * so: pidfd_getfd(-1,-1) must answer EBADF, kcmp(-1,-1,..) ESRCH from
 * any unfiltered kernel.
 *
 * Each probe runs in its own child; the child prints the raw errno.
 * Build: cc -O2 -o attribute attribute.c
 */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <unistd.h>
#include <sys/mount.h>
#include <sys/ptrace.h>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <sys/wait.h>

#define BOGUS "/proc/self/no-such-attribute-path"
#define BOGUS_PID 999999

static const char *ename(int e) { return strerror(e); }

static void run(const char *name, int (*fn)(int *)) {
	fflush(stdout);
	pid_t p = fork();
	if (p == 0) {
		int e = 0;
		long r = fn(&e);
		if (r >= 0) printf("%-40s OK (executed)\n", name);
		else printf("%-40s errno=%d (%s)%s\n", name, e, ename(e),
		       (e == EPERM) ? "  <- refused pre-entry" : "  <- executed, kernel-internal denial");
		fflush(stdout);	/* _exit discards stdio buffers: without this the verdict is lost */
		_exit(0);
	}
	int st; waitpid(p, &st, 0);
	if (!WIFEXITED(st) || WEXITSTATUS(st) != 0)
		printf("%-40s probe-child abnormal exit %d\n", name,
		       WIFSIGNALED(st) ? 128 + WTERMSIG(st) : WEXITSTATUS(st));
}

static long raw6(long n, long a, long b, long c, long d, long e, long f, int *err) {
	long r = syscall(n, a, b, c, d, e, f);
	if (r < 0) *err = errno;
	return r;
}

static int mount_bogus(int *e)      { return (int)raw6(SYS_mount, (long)"none", (long)BOGUS, (long)"tmpfs", 0, 0, 0, e); }
static int umount2_bogus(int *e)    { return (int)raw6(SYS_umount2, (long)BOGUS, 0, 0, 0, 0, 0, e); }
static int pivot_bogus(int *e)      { return (int)raw6(SYS_pivot_root, (long)BOGUS, (long)BOGUS, 0, 0, 0, 0, e); }
static int probe_fsopen(int *e)      { return (int)raw6(430, (long)"tmpfs", 0, 0, 0, 0, 0, e); }
static int move_mount_bogus(int *e) {
	int fs = syscall(430, "tmpfs", 0);
	if (fs < 0) { *e = errno; return -1; }
	/* fsconfig(CREATE) so a fsmount fd exists to move */
	long r = raw6(429, fs, (long)"", -100 /* AT_FDCWD */, (long)BOGUS, 0x4 /* FMPATH empty-source */, 0, e);
	close(fs);
	return (int)r;
}
static int process_vm_bogus(int *e) {
	struct { void *base; size_t len; } l = { (void *)&e, 1 }, r = { (void *)0x1000, 1 };
	return (int)raw6(310, BOGUS_PID, (long)&l, 1, (long)&r, 1, 0, e);
}
static int pidfd_getfd_ctrl(int *e) { return (int)raw6(438, -1, -1, 0, 0, 0, 0, e); }
static int kcmp_ctrl(int *e)        { return (int)raw6(312, -1, -1, 0, 0, 0, 0, e); }
static int open_tree_bogus(int *e)  { return (int)raw6(428, -100, (long)BOGUS, 0, 0, 0, 0, e); }
static int setns_bogus(int *e)      { return (int)raw6(308, -1, 0, 0, 0, 0, 0, e); }
/* entry-layer controls: syscalls with no plausible denylist entry. If
 * these answered pre-entry EPERM, the denial layer would be uniform
 * rather than a number list — they must reach the kernel. */
static int membarrier_bogus(int *e) { return (int)raw6(324, -1, 0, -1, 0, 0, 0, e); }
static int mprotect_null(int *e)    { return (int)raw6(10, 0, 0, 0, 0, 0, 0, e); }

int main(void) {
	printf("## bogus-argument attribution (one child per probe)\n");
	printf("## convention: EPERM for a bogus argument = refused pre-entry (filter);\n");
	printf("##            path/pid-shaped errno = executed (kernel-internal denial)\n\n");
	run("mount(2, bogus target)", mount_bogus);
	run("umount2(2, bogus target)", umount2_bogus);
	run("pivot_root(2, bogus paths)", pivot_bogus);
	run("fsopen(tmpfs) [new mount API]", probe_fsopen);
	run("open_tree(bogus)", open_tree_bogus);
	run("move_mount(-> bogus dest)", move_mount_bogus);
	run("setns(bogus fd)", setns_bogus);
	run("process_vm_readv(bogus pid)", process_vm_bogus);
	printf("\n## controls (must be argument-shaped, or the prober is broken)\n");
	run("pidfd_getfd(-1,-1)", pidfd_getfd_ctrl);
	run("kcmp(-1,-1,...)", kcmp_ctrl);
	run("membarrier(bogus) [entry ctrl]", membarrier_bogus);
	run("mprotect(0,0,0) [entry ctrl]", mprotect_null);
	return 0;
}
