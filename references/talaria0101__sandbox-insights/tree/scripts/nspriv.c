/* nspriv.c — what a namespace created through the clone-family gap
 * grants, in-process, and where its boundary sits.
 *
 * The seccomp denylist of this runtime class refuses the legacy
 * unshare(2) and setns(2) but does not name clone(2)/clone3(2) with
 * namespace flags (measured: clone3(NULL) answers EINVAL — the kernel
 * rejected the argument, not the caller). A process that enters
 * CLONE_NEWUSER|CLONE_NEWNET through clone3 OWNS the new namespaces,
 * and capability checks for a namespace's owner userns pass for it.
 *
 * Demonstrated here, all in-process (no helper binary: exec drops the
 * granted set, see the boundary below):
 *   1. raw socket creation as netns owner (CAP_NET_RAW over the netns)
 *   2. CLONE_NEWPID private process tree (child reports PID 1)
 *   3. the boundary: uid_map stays EMPTY (procfs-provenance: nobody can
 *      write a map into /proc mounted from the initial userns), and an
 *      exec from an unmapped uid drops the granted capabilities — so
 *      these privileges are in-process only.
 *
 * Build: cc -O2 -o nspriv nspriv.c
 */
#define _GNU_SOURCE
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sched.h>
#include <linux/sched.h>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <netinet/in.h>
#include <unistd.h>

static struct clone_args ca_flags(unsigned long flags) {
	struct clone_args a;
	memset(&a, 0, sizeof(a));
	a.flags = flags;
	a.exit_signal = SIGCHLD;
	return a;
}

static int in_child_netraw(void) {
	/* the capability this child holds is netns-owner CAP_NET_RAW */
	int s = socket(AF_INET, SOCK_RAW, IPPROTO_ICMP);
	if (s < 0) { printf("raw-icmp-socket: %s\n", strerror(errno)); return 1; }
	printf("raw-icmp-socket: OK (netns-owner CAP_NET_RAW effective in-process)\n");
	close(s);
	/* the boundary, made visible: the uid map is empty because /proc is
	 * a mount from the initial userns and map_write() checks against the
	 * MOUNT's userns, not the target process's */
	FILE *f = fopen("/proc/self/uid_map", "r");
	char buf[64] = { 0 };
	size_t n = f ? fread(buf, 1, sizeof(buf) - 1, f) : 0;
	if (f) fclose(f);
	printf("uid_map in child: [%s]%s\n", n ? buf : "",
	       n ? "" : " (EMPTY — unwritable: procfs mount provenance)");
	if (n) fflush(stdout);
	return 0;
}

int main(void) {
	setvbuf(stdout, NULL, _IONBF, 0);
	/* control first: raw socket in the parent's netns */
	int s = socket(AF_INET, SOCK_RAW, IPPROTO_ICMP);
	printf("parent raw-icmp-socket: %s\n", s < 0 ? strerror(errno) : "OK");
	if (s >= 0) close(s);

	printf("\n## clone3(CLONE_NEWUSER|CLONE_NEWNET), child probes in-process\n");
	struct clone_args a = ca_flags(CLONE_NEWUSER | CLONE_NEWNET);
	errno = 0;
	pid_t p = syscall(SYS_clone3, &a, sizeof(a));
	if (p < 0) {
		/* a committed negative result: the gap may have been closed by
		 * the operator since the last measurement */
		printf("clone3 NEWUSER|NEWNET: %s (gap closed — in-process namespace privileges gone)\n", strerror(errno));
		return 1;
	}
	if (p == 0) _exit(in_child_netraw());
	int st; waitpid(p, &st, 0);
	printf("clone3 NEWUSER|NEWNET: child spawned rc=%d\n", WIFEXITED(st) ? WEXITSTATUS(st) : -1);

	printf("\n## clone3(CLONE_NEWPID), child reports its own pid\n");
	a = ca_flags(CLONE_NEWPID);
	errno = 0;
	p = syscall(SYS_clone3, &a, sizeof(a));
	if (p < 0) { printf("clone3 NEWPID: %s\n", strerror(errno)); return 1; }
	if (p == 0) {
		printf("child getpid(): %d %s\n", getpid(), getpid() == 1 ? "(private PID tree: PID 1)" : "");
		fflush(stdout);
		_exit(0);
	}
	waitpid(p, &st, 0);

	printf("\n## the exec boundary: capabilities granted in the namespace die at execve\n");
	a = ca_flags(CLONE_NEWUSER);
	errno = 0;
	p = syscall(SYS_clone3, &a, sizeof(a));
	if (p < 0) { printf("clone3 NEWUSER: %s\n", strerror(errno)); return 1; }
	if (p == 0) {
		/* re-exec self with a marker; the exec'ed image runs with uid
		 * unmapped (65534) and NO capabilities from this namespace */
		char *argv[] = { "/proc/self/exe", "--post-exec", NULL };
		execv("/proc/self/exe", argv);
		_exit(127);
	}
	waitpid(p, &st, 0);
	return 0;
}

/* executed after the self-exec above (see the exec boundary) */
__attribute__((constructor)) static void post_exec_check(void) {
	/* cheap tell: if our uid is nobody (65534) the exec came from an
	 * unmapped-uid namespace child */
	if (getuid() == 65534) {
		printf("post-exec: uid=%d (unmapped) — exec dropped the granted capability set\n", getuid());
		fflush(stdout);
		_exit(0);
	}
}
