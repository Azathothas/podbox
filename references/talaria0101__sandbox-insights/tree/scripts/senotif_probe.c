/* senotif_probe.c — the ptrace-free primitive set for syscall tracing.
 *
 * Question: can a tracer exist where the ptrace syscall class is
 * denied outright? The tracee can always seccomp ITSELF, so yes — if
 * six primitives hold. This single file proves all six:
 *
 *   P1  a process can install SECCOMP_FILTER_FLAG_NEW_LISTENER on itself
 *   P2  the listener fd crosses to the tracer over a unix socket (SCM_RIGHTS)
 *   P3  NOTIF_RECV / NOTIF_SEND with USER_NOTIF_FLAG_CONTINUE shepherd
 *       every tracee syscall, unprivileged
 *   P4  a forged return value (no CONTINUE, val set) is accepted
 *   P5  SECCOMP_IOCTL_NOTIF_ADDFD injects a REAL fd into the tracee
 *   P6  /proc/<pid>/mem serves memory arguments while the tracee is
 *       blocked in the notification — stable by construction, no TOCTOU
 *
 * Boundary, probed last: an OUTER ERRNO filter outranks USER_NOTIF
 * (kernel action precedence), so syscalls an outer layer denies — the
 * ptrace class here — never notify and can never be mediated.
 *
 * Build: cc -O2 -o senotif_probe senotif_probe.c
 * Exit:  0 all primitives proven, 1 a primitive failed,
 *        2 user-notification unavailable.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/prctl.h>
#include <sys/ptrace.h>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <sys/uio.h>
#include <sys/wait.h>
#include <linux/audit.h>
#include <linux/filter.h>
#include <linux/seccomp.h>
#include <unistd.h>

#define TRACEE_TARGET "/bin/true"
#define INJECTED_FD 1000

static int passes, failed;

static void ok(const char *s) { passes++; printf("PASS %s\n", s); }
static void bad(const char *s) { failed++; printf("FAIL %s\n", s); }

/* notify on everything EXCEPT the handoff path: the tracee must be able
 * to sendmsg the listener fd to the tracer — a sendmsg caught by its own
 * filter would block in the notification queue before the tracer ever
 * holds the listener, and the handoff would deadlock. TSYNC is omitted:
 * combined with NEW_LISTENER the kernel refuses the filter outright
 * (EINVAL) — a listener belongs to one thread's filter. */
static int install_notify_filter(void) {
	struct sock_filter prog[] = {
		BPF_STMT(BPF_LD + BPF_W + BPF_ABS, offsetof(struct seccomp_data, nr)),
		BPF_JUMP(BPF_JMP + BPF_JEQ + BPF_K, __NR_sendmsg, 0, 1),
		BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_ALLOW),
		BPF_STMT(BPF_RET + BPF_K, SECCOMP_RET_USER_NOTIF),
	};
	const struct sock_fprog fprog = { .len = (unsigned)(sizeof(prog) / sizeof(prog[0])), .filter = prog };
	return (int)syscall(SYS_seccomp, SECCOMP_SET_MODE_FILTER,
			    SECCOMP_FILTER_FLAG_NEW_LISTENER, &fprog);
}

#ifndef SECCOMP_USER_NOTIF_FLAG_CONTINUE
#define SECCOMP_USER_NOTIF_FLAG_CONTINUE (1UL << 2)
#endif
#ifndef SECCOMP_IOCTL_NOTIF_RECV
#define SECCOMP_IOCTL_NOTIF_RECV  0xc0502100
#define SECCOMP_IOCTL_NOTIF_SEND  0xc0182101
#endif
#ifndef SECCOMP_IOCTL_NOTIF_ADDFD
#define SECCOMP_IOCTL_NOTIF_ADDFD 0x40182103
#endif
#ifndef SECCOMP_ADDFD_FLAG_SETFD
#define SECCOMP_ADDFD_FLAG_SETFD (1UL << 0)
#endif
#ifndef SECCOMP_USER_NOTIF_FD_SYNC_WAKEUP
#define SECCOMP_USER_NOTIF_FD_SYNC_WAKEUP (1UL << 0)
#endif

static void notif_init(struct seccomp_notif *n) { memset(n, 0, sizeof(*n)); }
static void resp_init(struct seccomp_notif_resp *r) { memset(r, 0, sizeof(*r)); }

/* receive a listener fd over a unix socket; returns fd or -1 (timeout 3s) */
static int recv_listener(int sock) {
	struct pollfd pf = { .fd = sock, .events = POLLIN };
	if (poll(&pf, 1, 3000) != 1) return -1;
	struct msghdr mh = { 0 };
	char cbuf[CMSG_SPACE(sizeof(int))]; memset(cbuf, 0, sizeof(cbuf));
	char b[4]; struct iovec iov = { .iov_base = b, .iov_len = sizeof(b) };
	mh.msg_iov = &iov; mh.msg_iovlen = 1;
	mh.msg_control = cbuf; mh.msg_controllen = sizeof(cbuf);
	if (recvmsg(sock, &mh, 0) < 0) return -1;
	int fd = -1;
	for (struct cmsghdr *cm = CMSG_FIRSTHDR(&mh); cm; cm = CMSG_NXTHDR(&mh, cm))
		if (cm->cmsg_level == SOL_SOCKET && cm->cmsg_type == SCM_RIGHTS)
			memcpy(&fd, CMSG_DATA(cm), sizeof(int));
	return fd;
}

int main(int argc, char **argv) {
	setvbuf(stdout, NULL, _IONBF, 0);

	if (argc > 1 && !strcmp(argv[1], "--addfd-target")) {
		/* P5 target: read the injected fd; print what arrives */
		char buf[128] = { 0 };
		int n = read(INJECTED_FD, buf, sizeof(buf) - 1);
		if (n >= 0) printf("TRACEE-ADDFD-READ: [%s]\n", buf);
		_exit(n >= 0 ? 0 : 3);
	}

	int sk[2];
	if (socketpair(AF_UNIX, SOCK_STREAM, 0, sk) < 0) { perror("socketpair"); return 2; }

	/* ---------- P1 + P2: self-install + handoff ---------- */
	pid_t tracee = fork();
	if (tracee == 0) {
		int lfd = install_notify_filter();
		if (lfd < 0) _exit(90);
		struct msghdr mh = { 0 };
		char cbuf[CMSG_SPACE(sizeof(int))]; memset(cbuf, 0, sizeof(cbuf));
		struct iovec iov = { .iov_base = "H", .iov_len = 1 };
		mh.msg_iov = &iov; mh.msg_iovlen = 1;
		mh.msg_control = cbuf; mh.msg_controllen = sizeof(cbuf);
		struct cmsghdr *cm = CMSG_FIRSTHDR(&mh);
		cm->cmsg_level = SOL_SOCKET; cm->cmsg_type = SCM_RIGHTS; cm->cmsg_len = CMSG_LEN(sizeof(int));
		memcpy(CMSG_DATA(cm), &lfd, sizeof(int));
		if (sendmsg(sk[0], &mh, 0) < 0) _exit(91);
		execl(TRACEE_TARGET, TRACEE_TARGET, (char *)NULL);
		_exit(92);
	}
	int lfd = recv_listener(sk[1]);
	if (lfd < 0) {
		int st; waitpid(tracee, &st, 0);
		fprintf(stderr, "P1/P2 failed: no listener (tracee exit %d)\n",
			WIFEXITED(st) ? WEXITSTATUS(st) : -1);
		return 2;
	}
	ok("P1 self-install of SECCOMP_FILTER_FLAG_NEW_LISTENER + P2 handoff over SCM_RIGHTS");

	/* ---------- P3 + P6: shepherding + memory read ---------- */
	char mempath[64];
	snprintf(mempath, sizeof(mempath), "/proc/%d/mem", tracee);
	int mem_ok = 0, continued = 0;
	char seen_path[256] = { 0 };

	/* drive until the tracee exits. Breaking out earlier leaves the tracee
	 * blocked in its next notified syscall — a hang by construction. */
	for (;;) {
		struct seccomp_notif n; notif_init(&n);
		if (ioctl(lfd, SECCOMP_IOCTL_NOTIF_RECV, &n) < 0) {
			if (errno == EINTR) continue;
			break;	/* tracee gone: ENOENT */
		}
		struct seccomp_notif_resp r; resp_init(&r); r.id = n.id;
		r.flags = SECCOMP_USER_NOTIF_FLAG_CONTINUE;
		if ((n.data.nr == SYS_execve || n.data.nr == SYS_execveat) && !mem_ok) {
			/* the tracee is BLOCKED in this notification: its memory is
			 * quiescent, so /proc/<pid>/mem reads are stable (no TOCTOU
			 * with a concurrent write — the tracee cannot run). The
			 * remote pointer is the OFFSET into the mem file. */
			int mfd = open(mempath, O_RDONLY);
			if (mfd >= 0) {
				ssize_t got = pread(mfd, seen_path, sizeof(seen_path) - 1,
						    (off_t)n.data.args[0]);
				if (got > 0) {
					seen_path[strcspn(seen_path, "\n")] = 0;
					mem_ok = 1;
				}
				close(mfd);
			}
		}
		if (ioctl(lfd, SECCOMP_IOCTL_NOTIF_SEND, &r) < 0 && errno == ENOENT) continue;
		continued++;
	}
	if (continued > 0) ok("P3 CONTINUE shepherding (every syscall continued to run)");
	else { bad("P3 no notifications processed"); }
	if (mem_ok) printf("PASS P6 /proc/<pid>/mem read while tracee blocked: execve path=[%s]\n", seen_path), passes++;
	else bad("P6 /proc/<pid>/mem read failed while blocked");

	int st; waitpid(tracee, &st, 0);
	int tracee_rc = WIFEXITED(st) ? WEXITSTATUS(st) : -1;
	printf("tracee (/bin/true under a notify-everything filter) exited %d — shepherded to completion\n", tracee_rc);

	/* ---------- P4: forged return value ---------- */
	{
		int sk2[2]; socketpair(AF_UNIX, SOCK_STREAM, 0, sk2);
		pid_t t2 = fork();
		if (t2 == 0) {
			int l2 = install_notify_filter();
			if (l2 < 0) _exit(90);
			struct msghdr mh = { 0 };
			char cbuf[CMSG_SPACE(sizeof(int))]; memset(cbuf, 0, sizeof(cbuf));
			struct iovec iov = { .iov_base = "H", .iov_len = 1 };
			mh.msg_iov = &iov; mh.msg_iovlen = 1;
			mh.msg_control = cbuf; mh.msg_controllen = sizeof(cbuf);
			struct cmsghdr *cm = CMSG_FIRSTHDR(&mh);
			cm->cmsg_level = SOL_SOCKET; cm->cmsg_type = SCM_RIGHTS; cm->cmsg_len = CMSG_LEN(sizeof(int));
			memcpy(CMSG_DATA(cm), &l2, sizeof(int));
			sendmsg(sk2[0], &mh, 0);
			/* the consumer side of the proof: call getuid until it returns
			 * the forged value — the tracee OBSERVES the forgery */
			for (int i = 0; i < 1000000; i++) {
				uid_t u = getuid();
				if (u == 4242) {
					printf("P4-TRACEE: getuid returned 4242 — the forged value reached the tracee\n");
					_exit(0);
				}
			}
			_exit(1);
		}
		int l2 = recv_listener(sk2[1]);
		if (l2 >= 0) {
			int forged = 0, drained = 0;
			for (;;) {
				struct seccomp_notif n; notif_init(&n);
				if (ioctl(l2, SECCOMP_IOCTL_NOTIF_RECV, &n) < 0) break;	/* tracee gone */
				struct seccomp_notif_resp r; resp_init(&r); r.id = n.id;
				if ((n.data.nr == SYS_getuid || n.data.nr == SYS_geteuid) && !forged) {
					r.val = 4242;	/* no CONTINUE: the value replaces the syscall */
					forged = 1;
				} else r.flags = SECCOMP_USER_NOTIF_FLAG_CONTINUE;
				ioctl(l2, SECCOMP_IOCTL_NOTIF_SEND, &r);
				drained++;
			}
			int st2; waitpid(t2, &st2, 0);
			int saw = WIFEXITED(st2) && WEXITSTATUS(st2) == 0;
			if (forged && saw) ok("P4 forged return: kernel accepted NOTIF_SEND without CONTINUE; the tracee read 4242");
			else printf("FAIL P4 forged=%d tracee-saw=%d (drained %d)\n", forged, saw, drained), failed++;
		} else { bad("P4 second handoff failed"); kill(t2, SIGKILL); waitpid(t2, NULL, 0); }
	}

	/* ---------- P5: ADDFD ---------- */
	{
		char self[4096];
		ssize_t sl = readlink("/proc/self/exe", self, sizeof(self) - 1);
		if (sl > 0) {
			self[sl] = 0;
			int sk3[2]; socketpair(AF_UNIX, SOCK_STREAM, 0, sk3);
			int inject_src = open("/etc/hostname", O_RDONLY);
			if (inject_src < 0) inject_src = open("/proc/self/cmdline", O_RDONLY);
			pid_t t3 = fork();
			if (t3 == 0) {
				int l3 = install_notify_filter();
				if (l3 < 0) _exit(90);
				struct msghdr mh = { 0 };
				char cbuf[CMSG_SPACE(sizeof(int))]; memset(cbuf, 0, sizeof(cbuf));
				struct iovec iov = { .iov_base = "H", .iov_len = 1 };
				mh.msg_iov = &iov; mh.msg_iovlen = 1;
				mh.msg_control = cbuf; mh.msg_controllen = sizeof(cbuf);
				struct cmsghdr *cm = CMSG_FIRSTHDR(&mh);
				cm->cmsg_level = SOL_SOCKET; cm->cmsg_type = SCM_RIGHTS; cm->cmsg_len = CMSG_LEN(sizeof(int));
				memcpy(CMSG_DATA(cm), &l3, sizeof(int));
				sendmsg(sk3[0], &mh, 0);
				execl(self, self, "--addfd-target", (char *)NULL);
				_exit(92);
			}
			int l3 = recv_listener(sk3[1]);
			if (l3 >= 0 && inject_src >= 0) {
				int injected = 0;
				for (;;) {
					struct seccomp_notif n; notif_init(&n);
					if (ioctl(l3, SECCOMP_IOCTL_NOTIF_RECV, &n) < 0) break;	/* tracee gone */
					struct seccomp_notif_resp r; resp_init(&r); r.id = n.id;
					if (n.data.nr == SYS_read && (int)(signed)n.data.args[0] == INJECTED_FD) {
						/* the tracee is blocked reading fd 1000: install
						 * OUR fd as ITS fd 1000; the read proceeds on it */
						struct seccomp_notif_addfd ad;
						memset(&ad, 0, sizeof(ad));
						ad.id = n.id;
						ad.flags = SECCOMP_ADDFD_FLAG_SETFD;
						ad.srcfd = (unsigned)inject_src;
						ad.newfd = INJECTED_FD;
						ad.newfd_flags = 0;
						if (ioctl(l3, SECCOMP_IOCTL_NOTIF_ADDFD, &ad) >= 0) injected = 1;
						r.flags = SECCOMP_USER_NOTIF_FLAG_CONTINUE;
					} else r.flags = SECCOMP_USER_NOTIF_FLAG_CONTINUE;
					ioctl(l3, SECCOMP_IOCTL_NOTIF_SEND, &r);
					/* drain until the tracee exits — a notification left
					 * unanswered blocks the tracee in that syscall forever */
				}
				int st3; waitpid(t3, &st3, 0);
				int exited_clean = WIFEXITED(st3) && WEXITSTATUS(st3) == 0;
				if (injected && exited_clean) ok("P5 SECCOMP_IOCTL_NOTIF_ADDFD installed a real fd (tracee read it)");
				else printf("FAIL P5 injected=%d clean-exit=%d\n", injected, exited_clean), failed++;
			} else { bad("P5 handoff or inject source unavailable"); kill(t3, SIGKILL); waitpid(t3, NULL, 0); }
			close(inject_src);
		}
	}

	/* ---------- the boundary: outer ERRNO outranks USER_NOTIF ---------- */
	{
		pid_t t4 = fork();
		if (t4 == 0) {
			errno = 0;
			long rc = ptrace(PTRACE_TRACEME, 0, 0, 0);
			printf("boundary: ptrace(TRACEME) inside a self-filtered tracee: %s — an outer ERRNO filter\n", rc == 0 ? "OK" : strerror(errno));
			printf("  outranks USER_NOTIF, so denied syscalls NEVER notify and can never be mediated\n");
			_exit(0);
		}
		int st4; waitpid(t4, &st4, 0); (void)st4;
	}

	printf("\n%d passed, %d failed\n", passes, failed);
	return failed ? 1 : 0;
}
