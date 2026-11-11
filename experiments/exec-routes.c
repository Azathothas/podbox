/* The child routes of T-1703 and the socket paths of T-1710, run inside
 * the image by experiment 432. argv[1] names the route: sys, popen,
 * fexecve, memfd, `listen PATH` or `connect PATH`. Each prints the lines
 * that the experiment matches. */
#define _GNU_SOURCE
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/wait.h>
#include <unistd.h>

extern char **environ;

int main(int argc, char **argv)
{
	char line[128];
	char *echo[] = {"echo", NULL, NULL};

	if (argc < 2)
		return 2;
	if (strcmp(argv[1], "sys") == 0) {
		int s = system("[ -e /etc/alpine-release ] && echo sys-$((6*7)); exit 3");
		printf("system-status %d\n", WIFEXITED(s) ? WEXITSTATUS(s) : -1);
		return 0;
	}
	if (strcmp(argv[1], "popen") == 0) {
		FILE *f = popen("echo popen-$((6*7))", "r");
		if (!f) {
			perror("popen");
			return 1;
		}
		if (fgets(line, sizeof line, f))
			fputs(line, stdout);
		printf("pclose-status %d\n", WEXITSTATUS(pclose(f)));
		return 0;
	}
	if (strcmp(argv[1], "fexecve") == 0) {
		int fd = open("/bin/busybox", O_RDONLY | O_CLOEXEC);
		if (fd < 0) {
			perror("open");
			return 1;
		}
		echo[1] = "fexecve-ok";
		fflush(stdout);
		fexecve(fd, echo, environ);
		perror("fexecve");
		return 1;
	}
	if (strcmp(argv[1], "memfd") == 0) {
		int in = open("/bin/busybox", O_RDONLY), m = memfd_create("t1703", MFD_CLOEXEC);
		ssize_t n;
		char buf[65536];
		if (in < 0 || m < 0) {
			perror("open");
			return 1;
		}
		while ((n = read(in, buf, sizeof buf)) > 0)
			if (write(m, buf, (size_t)n) != n)
				return 1;
		echo[1] = "memfd-ok";
		fexecve(m, echo, environ);
		perror("fexecve memfd");
		return 1;
	}
	if (argc > 2 && (strcmp(argv[1], "listen") == 0 || strcmp(argv[1], "connect") == 0)) {
		struct sockaddr_un a = {.sun_family = AF_UNIX};
		socklen_t n = sizeof a;
		int s = socket(AF_UNIX, SOCK_STREAM, 0);
		strncpy(a.sun_path, argv[2], sizeof a.sun_path - 1);
		if (argv[1][0] == 'l') {
			int c;
			if (bind(s, (struct sockaddr *)&a, sizeof a) || listen(s, 1)) {
				perror("bind");
				return 1;
			}
			memset(&a, 0, sizeof a);
			getsockname(s, (struct sockaddr *)&a, &n);
			printf("bound %s\n", a.sun_path);
			fflush(stdout);
			c = accept(s, NULL, NULL);
			if (c < 0 || write(c, "sock-hi\n", 8) != 8)
				return 1;
			close(c);
			return 0;
		}
		if (connect(s, (struct sockaddr *)&a, sizeof a)) {
			perror("connect");
			return 1;
		}
		memset(line, 0, sizeof line);
		if (read(s, line, sizeof line - 1) <= 0)
			return 1;
		printf("got %s", line);
		return 0;
	}
	return 2;
}
