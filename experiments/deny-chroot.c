/* deny-chroot.c - the target shape's one denial, for operations the
 * drive must meet rather than assume.
 *
 * Installs a seccomp filter denying chroot(2) with EPERM and execs its
 * arguments. Everything else is allowed. The /dev/ptmx half of the
 * target shape comes from a directory bind-mounted over /dev/ptmx in a
 * mount namespace (see experiments/356-no-chroot-rung.sh): open answers
 * EISDIR there, which is the stat-Ok-open-denied arm T-0503 refuses.
 *
 * Build: cc -O2 -Wall -Werror -static -o deny-chroot deny-chroot.c
 * (static: the fixture must not need the loader it may itself deny
 * nothing of, and must run anywhere).
 *
 * Exit: 2 no command, otherwise the command's own code. A filter that
 * could not install refuses rather than running unfiltered: an
 * unfiltered run would prove the capable path and read as the denied
 * one.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <linux/audit.h>
#include <linux/filter.h>
#include <linux/seccomp.h>
#include <linux/unistd.h>
#include <stddef.h>
#include <stdio.h>
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <unistd.h>

int main(int argc, char **argv)
{
	if (argc < 2) {
		fprintf(stderr, "usage: deny-chroot cmd [arg...]\n");
		return 2;
	}
	struct sock_filter f[] = {
		BPF_STMT(BPF_LD + BPF_W + BPF_ABS,
			 offsetof(struct seccomp_data, arch)),
		BPF_JUMP(BPF_JMP + BPF_JEQ + BPF_K, AUDIT_ARCH_X86_64, 1, 0),
		BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_ALLOW),
		BPF_STMT(BPF_LD + BPF_W + BPF_ABS,
			 offsetof(struct seccomp_data, nr)),
		BPF_JUMP(BPF_JMP + BPF_JEQ + BPF_K, __NR_chroot, 1, 0),
		BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_ALLOW),
		BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_ERRNO | EPERM),
	};
	struct sock_fprog prog = {
		(unsigned short)(sizeof(f) / sizeof(f[0])), f,
	};
	if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0) {
		perror("deny-chroot: PR_SET_NO_NEW_PRIVS");
		return 2;
	}
	if (syscall(__NR_seccomp, SECCOMP_SET_MODE_FILTER, 0, &prog) != 0) {
		perror("deny-chroot: SECCOMP_SET_MODE_FILTER");
		return 2;
	}
	execvp(argv[1], &argv[1]);
	perror("deny-chroot: execvp");
	return 2;
}
