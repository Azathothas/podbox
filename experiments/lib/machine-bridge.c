/* machine-bridge: a socket where the guest has only a serial line.
 *
 * Usage: machine-bridge TTY_PATH PROG [ARGS ...]
 *
 * The podbox machine arm boots a Linux guest whose only channel is its
 * first serial port, and speaks real SSH over it. The SSH server is
 * dropbear in inetd mode, but dropbear's inetd path calls getpeername on
 * its standard input and exits where that is not a socket (measured in
 * experiments 390 through the diag5/diag6 series: exit 1 with no banner
 * on pipes and ptys, a banner on a socket pair). A serial port is never
 * a socket, so the server cannot take the serial line directly.
 *
 * Upstream's answer to that shape is a wrapper, and this is it: one
 * socket pair, the server forked onto one end, the other end spliced
 * against the tty in both directions until the server's end reaches EOF,
 * then the exit waits out and its status is the bridge's own. The tty is
 * put in raw mode first where it has a line discipline to clear; a path
 * with none splices as it is. The bridge
 * moves opaque bytes and makes no decision: there is no parse here whose
 * mistake could admit, so a corruption fails closed rather than open.
 * Patching the server instead would add a divergent fork to carry across
 * pins, and the cage page records that maintenance cost paid once
 * already (docs/decisions/ssh-server-in-a-cage.md), so the server stays
 * pristine and this file carries the adaptation, owned and static.
 *
 * Exit: the server's status on a waited exit, 128 plus the signal where
 * a signal took it, 125 where the bridge itself could not start. A
 * write past the server's exit reads as EPIPE (SIGPIPE ignored) and
 * ends the server direction rather than the bridge, so a client that
 * pipelined past the server's death still gets the server's status;
 * a dead tty half-closes the server instead of wedging the splice.
 * Signals are not forwarded: the server owns its session, and the
 * machine arm on the host (not this file) stops the whole guest past
 * its deadline.
 */
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <termios.h>
#include <unistd.h>

#define BRIDGE_BUF 65536

static void die(const char *what)
{
	fprintf(stderr, "machine-bridge: %s: %s\n", what, strerror(errno));
	_exit(125);
}

/* Write every byte or fail: a partial stream is a corrupt session, and
 * a corrupt session must end here rather than arrive short. */
static void write_all(int fd, const char *buf, ssize_t n)
{
	while (n > 0) {
		ssize_t w = write(fd, buf, (size_t)n);
		if (w < 0) {
			if (errno == EINTR)
				continue;
			die("write");
		}
		buf += w;
		n -= w;
	}
}

/* Best-effort sibling for the server direction only. The server may
 * exit between the poll and the write, and its closed end then reads
 * as EPIPE with SIGPIPE ignored. That is the server's EOF arriving
 * as an error rather than a bridge failure: the reap below still
 * reports the server's own status. Any other error is a dead
 * transport and fails loud like write_all. */
static int write_sv(int fd, const char *buf, ssize_t n)
{
	while (n > 0) {
		ssize_t w = write(fd, buf, (size_t)n);
		if (w < 0) {
			if (errno == EINTR)
				continue;
			return -1;
		}
		buf += w;
		n -= w;
	}
	return 0;
}

int main(int argc, char **argv)
{
	int tty, sv[2], child_dead = 0, sv_eof = 0, tty_done = 0;
	int status = 0;
	pid_t pid;
	static char buf[BRIDGE_BUF];

	if (argc < 3) {
		fprintf(stderr, "usage: machine-bridge TTY_PATH PROG [ARGS ...]\n");
		return 125;
	}
	/* SIGPIPE ignored, like the server itself does: a write past the
	 * server's exit must return EPIPE where the splice can read it as
	 * the server's EOF, not kill the bridge by signal with the
	 * server's status still unreaped. */
	if (signal(SIGPIPE, SIG_IGN) == SIG_ERR)
		die("signal");
	/* O_NOCTTY: PID 1 has no controlling terminal to acquire, and must
	 * not gain one here; the server arranges its own session. */
	tty = open(argv[1], O_RDWR | O_NOCTTY);
	if (tty < 0)
		die("open tty");
	/* Raw mode where there is a discipline to clear, spelled out rather
	 * than cfmakeraw: the serial line is an SSH transport, and the
	 * default discipline mangles binary framing both ways (output
	 * post-processing expands newline bytes, canonical input buffers
	 * and edits them). Measured: a clean banner then `Bad packet
	 * length` on every key exchange. A path with no discipline
	 * (a pipe, a null device, a file) has nothing to clear and splices
	 * as it is; only the open above refuses. A set mode the device
	 * already has is kept; only the mangling bits are cleared. */
	{
		struct termios t;
		if (tcgetattr(tty, &t) == 0) {
			t.c_iflag &= (tcflag_t) ~(IGNBRK | BRKINT | PARMRK | ISTRIP |
			    INLCR | IGNCR | ICRNL | IXON);
			t.c_oflag &= (tcflag_t) ~OPOST;
			t.c_lflag &= (tcflag_t) ~(ECHO | ECHONL | ICANON | ISIG | IEXTEN);
			t.c_cflag &= (tcflag_t) ~(CSIZE | PARENB);
			t.c_cflag |= CS8;
			/* cfmakeraw pins these two as well: the inherited
			 * canonical VMIN waits for four bytes per read and
			 * stalls a message whose length is not a multiple
			 * of four, one direction wedging the other behind
			 * the single-threaded splice. */
			t.c_cc[VMIN] = 1;
			t.c_cc[VTIME] = 0;
			if (tcsetattr(tty, TCSANOW, &t) != 0)
				die("tcsetattr");
		}
	}
	if (socketpair(AF_UNIX, SOCK_STREAM, 0, sv) != 0)
		die("socketpair");
	pid = fork();
	if (pid < 0)
		die("fork");
	if (pid == 0) {
		if (dup2(sv[1], 0) < 0 || dup2(sv[1], 1) < 0 || dup2(sv[1], 2) < 0)
			_exit(127);
		close(sv[0]);
		close(sv[1]);
		close(tty);
		execvp(argv[2], &argv[2]);
		_exit(127);
	}
	close(sv[1]);
	for (;;) {
		struct pollfd fds[2];
		int n = 0, pr;
		pid_t r;
		if (!sv_eof) {
			fds[n].fd = sv[0];
			fds[n].events = POLLIN;
			n++;
		}
		/* Past the server's exit only the drain runs: bytes the tty
		 * still offers have nobody to read them. */
		if (!tty_done && !child_dead) {
			fds[n].fd = tty;
			fds[n].events = POLLIN;
			n++;
		}
		if (n == 0)
			break;
		pr = poll(fds, (nfds_t)n, 200);
		if (pr < 0) {
			if (errno == EINTR)
				continue;
			die("poll");
		}
		for (int i = 0; i < n; i++) {
			short rev = fds[i].revents;
			ssize_t nr;
			int from_tty;
			if (!(rev & (POLLIN | POLLERR | POLLHUP)))
				continue;
			from_tty = (fds[i].fd != sv[0]);
			do {
				nr = read(fds[i].fd, buf, sizeof(buf));
			} while (nr < 0 && errno == EINTR);
			if (nr <= 0) {
				/* EOF or a hard error ends the direction: a half
				 * stream is a corrupt session, and a corrupt
				 * session ends here rather than arriving short. */
				if (from_tty) {
					tty_done = 1;
					/* Half-close: a live server learns input
					 * ended and exits itself, which turns a
					 * tty hangup into the server's status
					 * instead of a wedged splice. */
					shutdown(sv[0], SHUT_WR);
				} else
					sv_eof = 1;
				continue;
			}
			if (from_tty) {
				if (write_sv(sv[0], buf, nr) != 0) {
					if (errno == EPIPE)
						sv_eof = 1;
					else
						die("write");
				}
			} else
				write_all(tty, buf, nr);
		}
		/* The wait is polled, never blocking: the drain above must keep
		 * moving while the server is still writing. */
		r = waitpid(pid, &status, WNOHANG);
		if (r == pid)
			child_dead = 1;
		/* The server's exit closes its end, so its EOF is already on
		 * the way or here: leaving needs only the drain to finish. */
		if (child_dead && sv_eof)
			break;
	}
	/* The loop leaves on the drain's end; the status is reaped blocking
	 * where the poll never saw the exit. */
	if (!child_dead) {
		while (waitpid(pid, &status, 0) < 0 && errno == EINTR)
			;
	}
	close(sv[0]);
	close(tty);
	if (WIFEXITED(status))
		return WEXITSTATUS(status);
	if (WIFSIGNALED(status))
		return 128 + WTERMSIG(status);
	return 125;
}
