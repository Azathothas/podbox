/* tcpfwd.c - one TCP forward, for experiment 280's driver side.
 *
 * The driver container must reach fixture registries at `localhost:PORT`,
 * but on a podman lane those registries publish on the host, reachable
 * only as host.containers.internal. The driver's /etc/hosts pins
 * localhost to 127.0.0.1 and cannot be rewritten unprivileged, so each
 * podbox call runs beside forwards that listen on driver loopback and
 * dial out: `tcpfwd LISTEN_PORT TARGET_HOST TARGET_PORT`.
 *
 * One process per forward, forked per connection, select(2) both ways
 * until EOF. No dependencies beyond libc; built static with zig so the
 * driver needs nothing installed:
 *   zig cc -target x86_64-linux-musl -static -O2 tcpfwd.c -o tcpfwd
 *
 * Exit: 0 on a clean kill (it runs until the container exits), 2 on usage
 * or a listen failure. A failed downstream connect drops that connection
 * with nothing printed: the caller reads podbox's error, not this one's.
 */
#include <arpa/inet.h>
#include <errno.h>
#include <netdb.h>
#include <netinet/in.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/select.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

static int send_all(int fd, const char *buf, size_t n) {
	while (n > 0) {
		ssize_t w = send(fd, buf, n, 0);
		if (w < 0) {
			if (errno == EINTR)
				continue;
			return -1;
		}
		if (w == 0)
			return -1;
		buf += w;
		n -= (size_t)w;
	}
	return 0;
}

static void copy_both(int a, int b) {
	char buf[65536];
	fd_set rfds;
	int maxfd = a > b ? a : b;
	int open_a = 1, open_b = 1;

	for (;;) {
		FD_ZERO(&rfds);
		if (open_a)
			FD_SET(a, &rfds);
		if (open_b)
			FD_SET(b, &rfds);
		if (select(maxfd + 1, &rfds, NULL, NULL, NULL) < 0) {
			if (errno == EINTR)
				continue;
			return;
		}
		if (open_a && FD_ISSET(a, &rfds)) {
			ssize_t n = recv(a, buf, sizeof buf, 0);
			if (n <= 0) {
				open_a = 0;
				shutdown(b, SHUT_WR);
			} else if (send_all(b, buf, (size_t)n) != 0) {
				return;
			}
		}
		if (open_b && FD_ISSET(b, &rfds)) {
			ssize_t n = recv(b, buf, sizeof buf, 0);
			if (n <= 0) {
				open_b = 0;
				shutdown(a, SHUT_WR);
			} else if (send_all(a, buf, (size_t)n) != 0) {
				return;
			}
		}
		if (!open_a && !open_b)
			return;
	}
}

int main(int argc, char **argv) {
	if (argc != 4) {
		fprintf(stderr, "usage: tcpfwd LISTEN_PORT TARGET_HOST TARGET_PORT\n");
		return 2;
	}
	int listen_port = atoi(argv[1]);

	struct addrinfo hints;
	memset(&hints, 0, sizeof hints);
	hints.ai_family = AF_UNSPEC;
	hints.ai_socktype = SOCK_STREAM;
	struct addrinfo *dst = NULL;
	if (getaddrinfo(argv[2], argv[3], &hints, &dst) != 0)
		return 2;

	int ls = socket(AF_INET, SOCK_STREAM, 0);
	if (ls < 0)
		return 2;
	int one = 1;
	setsockopt(ls, SOL_SOCKET, SO_REUSEADDR, &one, sizeof one);
	struct sockaddr_in lo;
	memset(&lo, 0, sizeof lo);
	lo.sin_family = AF_INET;
	lo.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
	lo.sin_port = htons((unsigned short)listen_port);
	if (bind(ls, (struct sockaddr *)&lo, sizeof lo) != 0)
		return 2;
	if (listen(ls, 16) != 0)
		return 2;

	/* Reap children without ever blocking the accept loop. */
	signal(SIGCHLD, SIG_IGN);

	for (;;) {
		int c = accept(ls, NULL, NULL);
		if (c < 0) {
			if (errno == EINTR)
				continue;
			return 2;
		}
		pid_t p = fork();
		if (p < 0) {
			close(c);
			continue;
		}
		if (p > 0) {
			close(c);
			continue;
		}
		int up = -1;
		for (struct addrinfo *ai = dst; ai != NULL; ai = ai->ai_next) {
			up = socket(ai->ai_family, ai->ai_socktype, ai->ai_protocol);
			if (up < 0)
				continue;
			if (connect(up, ai->ai_addr, ai->ai_addrlen) == 0)
				break;
			close(up);
			up = -1;
		}
		if (up < 0)
			_exit(0);
		copy_both(c, up);
		_exit(0);
	}
}
