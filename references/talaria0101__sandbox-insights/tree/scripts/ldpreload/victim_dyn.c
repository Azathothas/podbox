/* victim_dyn.c — dynamically linked C victim: its lchown/open calls go
 * through libc and are visible to an LD_PRELOAD shim. The shim sees
 * the unmapped-gid lchown; the kernel still refuses it (EINVAL from
 * the mapping check) — seeing a call is not clearing it.
 * Build: cc -O2 -o victim_dyn victim_dyn.c */
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

int main(void) {
	int fd = open("/tmp/.ldpreload-victim", O_CREAT | O_WRONLY, 0644);
	if (fd >= 0) close(fd);
	int r = lchown("/tmp/.ldpreload-victim", 0, 42);
	printf("victim_dyn: lchown -> %d errno=%d (%s)\n", r, r < 0 ? errno : 0,
	       r < 0 ? strerror(errno) : "ok");
	unlink("/tmp/.ldpreload-victim");
	return 0;
}
