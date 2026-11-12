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
 * With --deny-dispatch first, a second filter also denies
 * prctl(PR_SET_SYSCALL_USER_DISPATCH) with EPERM: the host where the
 * native-static rung cannot arm (TODO/enter.md T-1652).
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
#include <string.h>
#include <sys/prctl.h>
#include <sys/syscall.h>

/* The architecture of the filter is the one this file is built for: the
 * numbers are the native headers' (TODO/interpose.md T-1721). */
#if defined(__x86_64__)
#define FIXTURE_AUDIT_ARCH AUDIT_ARCH_X86_64
#elif defined(__aarch64__)
#define FIXTURE_AUDIT_ARCH AUDIT_ARCH_AARCH64
#else
#error "deny-chroot.c: add the AUDIT_ARCH of this machine"
#endif
#include <unistd.h>

int main(int argc, char **argv)
{
	int deny_dispatch = argc > 1 && strcmp(argv[1], "--deny-dispatch") == 0;
	if (deny_dispatch) {
		argv++;
		argc--;
	}
	if (argc < 2) {
		fprintf(stderr, "usage: deny-chroot [--deny-dispatch] cmd [arg...]\n");
		return 2;
	}
	struct sock_filter f[] = {
		BPF_STMT(BPF_LD + BPF_W + BPF_ABS,
			 offsetof(struct seccomp_data, arch)),
		BPF_JUMP(BPF_JMP + BPF_JEQ + BPF_K, FIXTURE_AUDIT_ARCH, 1, 0),
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
	struct sock_filter d[] = {
		BPF_STMT(BPF_LD + BPF_W + BPF_ABS,
			 offsetof(struct seccomp_data, nr)),
		BPF_JUMP(BPF_JMP + BPF_JEQ + BPF_K, __NR_prctl, 1, 0),
		BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_ALLOW),
		BPF_STMT(BPF_LD + BPF_W + BPF_ABS,
			 offsetof(struct seccomp_data, args[0])),
		BPF_JUMP(BPF_JMP + BPF_JEQ + BPF_K, 59, 1, 0),
		BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_ALLOW),
		BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_ERRNO | EPERM),
	};
	struct sock_fprog dprog = {
		(unsigned short)(sizeof(d) / sizeof(d[0])), d,
	};
	if (deny_dispatch &&
	    syscall(__NR_seccomp, SECCOMP_SET_MODE_FILTER, 0, &dprog) != 0) {
		perror("deny-chroot: the --deny-dispatch filter");
		return 2;
	}
	execvp(argv[1], &argv[1]);
	perror("deny-chroot: execvp");
	return 2;
}
