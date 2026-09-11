/* victim_static.c — statically linked victim: no dynamic loader runs,
 * so no preload can ever see it. Same syscalls as victim_dyn.
 * Build: cc -O2 -static -o victim_static victim_static.c */
#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
	int fd = open("/tmp/.ldpreload-victim", O_CREAT | O_WRONLY, 0644);
	if (fd >= 0) close(fd);
	int r = lchown("/tmp/.ldpreload-victim", 0, 0);
	printf("victim_static: lchown -> %d\n", r);
	unlink("/tmp/.ldpreload-victim");
	return 0;
}
