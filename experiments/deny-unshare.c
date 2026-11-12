/* deny-unshare.c - a host that denies user namespaces, for the refusal
 * of the runsc tier (TODO/enter.md T-1729, experiments/436-runsc.sh).
 *
 * Installs a seccomp filter that denies unshare(2) with EPERM and execs
 * its arguments. Everything else is allowed.
 *
 * Build: cc -O2 -Wall -Werror -static -o deny-unshare deny-unshare.c
 *
 * Exit: 2 no command or no filter, otherwise the command's own code.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <linux/audit.h>
#include <linux/filter.h>
#include <linux/seccomp.h>
#include <stddef.h>
#include <stdio.h>
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <unistd.h>

#if defined(__x86_64__)
#define FIXTURE_AUDIT_ARCH AUDIT_ARCH_X86_64
#elif defined(__aarch64__)
#define FIXTURE_AUDIT_ARCH AUDIT_ARCH_AARCH64
#else
#error "deny-unshare.c: add the AUDIT_ARCH of this machine"
#endif

int main(int argc, char **argv)
{
	if (argc < 2) {
		fprintf(stderr, "usage: deny-unshare cmd [arg...]\n");
		return 2;
	}
	struct sock_filter f[] = {
		BPF_STMT(BPF_LD + BPF_W + BPF_ABS,
			 offsetof(struct seccomp_data, arch)),
		BPF_JUMP(BPF_JMP + BPF_JEQ + BPF_K, FIXTURE_AUDIT_ARCH, 1, 0),
		BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_ALLOW),
		BPF_STMT(BPF_LD + BPF_W + BPF_ABS,
			 offsetof(struct seccomp_data, nr)),
		BPF_JUMP(BPF_JMP + BPF_JEQ + BPF_K, __NR_unshare, 1, 0),
		BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_ALLOW),
		BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_ERRNO | EPERM),
	};
	struct sock_fprog prog = {
		(unsigned short)(sizeof(f) / sizeof(f[0])), f,
	};
	if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 ||
	    syscall(__NR_seccomp, SECCOMP_SET_MODE_FILTER, 0, &prog) != 0) {
		perror("deny-unshare: the filter");
		return 2;
	}
	execvp(argv[1], &argv[1]);
	perror("deny-unshare: execvp");
	return 2;
}
