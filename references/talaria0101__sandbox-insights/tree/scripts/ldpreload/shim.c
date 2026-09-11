/* shim.c — an LD_PRELOAD interposer on lchown/open, used to measure
 * which payload classes an interposer actually reaches.
 *
 * Build: cc -O2 -shared -fPIC -o shim.so shim.c -ldl
 * Use:   LD_PRELOAD=$PWD/shim.so ./victim
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>
#include <stdarg.h>

int lchown(const char *path, uid_t owner, gid_t group) {
	static int (*real)(const char *, uid_t, gid_t);
	if (!real) real = dlsym(RTLD_NEXT, "lchown");
	fprintf(stderr, "SHIM: lchown(\"%s\", %u, %u) intercepted\n", path, owner, group);
	return real(path, owner, group);
}

int chown(const char *path, uid_t owner, gid_t group) {
	static int (*real)(const char *, uid_t, gid_t);
	if (!real) real = dlsym(RTLD_NEXT, "chown");
	fprintf(stderr, "SHIM: chown(\"%s\", %u, %u) intercepted\n", path, owner, group);
	return real(path, owner, group);
}

int open(const char *path, int flags, ...) {
	static int (*real_open)(const char *, int, ...);
	if (!real_open) real_open = dlsym(RTLD_NEXT, "open");
	if (flags & O_CREAT) {
		va_list ap; va_start(ap, flags); mode_t m = va_arg(ap, mode_t); va_end(ap);
		fprintf(stderr, "SHIM: open(\"%s\") intercepted\n", path);
		return real_open(path, flags, m);
	}
	fprintf(stderr, "SHIM: open(\"%s\") intercepted\n", path);
	return real_open(path, flags);
}
