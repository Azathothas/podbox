/* libfakepasswd.c — environment completion for a runtime with no
 * /etc/passwd and a read-only root: serve a virtual one.
 *
 * Subjects that resolve the invoking user at startup (Go's
 * user.Current via cgo, anything calling getpwuid_r, anything opening
 * /etc/passwd directly) die before doing any work. Two hooks cover
 * both consumption paths:
 *   - getpwuid_r / getpwnam_r: answered from the virtual files
 *   - open/open64/openat of /etc/passwd, /etc/group: redirected
 *
 * This is environment COMPLETION, not isolation: the kernel's own
 * checks are untouched, and every consumer that bypasses libc (static
 * binaries, Go built without cgo) is out of reach by construction.
 *
 * Build: gcc -shared -fPIC -o libfakepasswd.so libfakepasswd.c -ldl
 * Use:   FAKE_ETC=/path/to/virtual-etc LD_PRELOAD=$PWD/libfakepasswd.so <subject>
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <grp.h>
#include <pwd.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <unistd.h>

#ifndef FAKE_ETC
#define FAKE_ETC "/workspace/fake-etc"
#endif

static const char *fake_etc(void) {
	const char *e = getenv("FAKE_ETC");
	return e ? e : FAKE_ETC;
}

static const char *redirect(const char *path) {
	if (!path) return path;
	if (!strcmp(path, "/etc/passwd")) { static char b[4096]; snprintf(b, sizeof(b), "%s/passwd", fake_etc()); return b; }
	if (!strcmp(path, "/etc/group"))  { static char b[4096]; snprintf(b, sizeof(b), "%s/group", fake_etc()); return b; }
	return path;
}

/* ---- open-family redirection -------------------------------------- */
static int open_impl(const char *path, int flags, mode_t mode, int has_mode) {
	static int (*real_open)(const char *, int, ...);
	if (!real_open) real_open = dlsym(RTLD_NEXT, "open");
	const char *p = redirect(path);
	if (has_mode) return real_open(p, flags, mode);
	return real_open(p, flags);
}

int open(const char *path, int flags, ...) {
	va_list ap; va_start(ap, flags);
	mode_t m = (flags & O_CREAT) ? va_arg(ap, mode_t) : 0;
	va_end(ap);
	return open_impl(path, flags, m, flags & O_CREAT);
}

int open64(const char *path, int flags, ...) {
	va_list ap; va_start(ap, flags);
	mode_t m = (flags & O_CREAT) ? va_arg(ap, mode_t) : 0;
	va_end(ap);
	return open_impl(path, flags, m, flags & O_CREAT);
}

int openat(int dirfd, const char *path, int flags, ...) {
	static int (*real_openat)(int, const char *, int, ...);
	if (!real_openat) real_openat = dlsym(RTLD_NEXT, "openat");
	mode_t m = 0;
	if (flags & O_CREAT) { va_list ap; va_start(ap, flags); m = va_arg(ap, mode_t); va_end(ap); }
	return real_openat(dirfd, redirect(path), flags, m);
}

/* ---- passwd/group lookups ------------------------------------------ */
int getpwuid_r(uid_t uid, struct passwd *pwd, char *buf, size_t buflen, struct passwd **result) {
	static int (*real)(uid_t, struct passwd *, char *, size_t, struct passwd **);
	if (!real) real = dlsym(RTLD_NEXT, "getpwuid_r");
	int r = real(uid, pwd, buf, buflen, result);
	if (r != 0 || !*result) {
		/* serve root from the virtual file when the real lookup fails */
		char line[1024];
		char virt[4096]; snprintf(virt, sizeof(virt), "%s/passwd", fake_etc());
		FILE *f = fopen(virt, "r");
		if (f) {
			while (fgets(line, sizeof(line), f)) {
				/* parse: name:passwd:uid:gid:gecos:dir:shell */
				char *fields[7]; int nf = 0; char *s = line;
				fields[nf++] = s;
				for (char *c = line; *c && nf < 7; c++)
					if (*c == ':') { *c = 0; fields[nf++] = c + 1; }
				if (nf == 7 && (uid_t)strtoul(fields[2], NULL, 10) == uid) {
					pwd->pw_name = fields[0]; pwd->pw_passwd = fields[1];
					pwd->pw_uid = uid; pwd->pw_gid = (gid_t)strtoul(fields[3], NULL, 10);
					pwd->pw_gecos = fields[4]; pwd->pw_dir = fields[5]; pwd->pw_shell = fields[6];
					*result = pwd;
					fclose(f);
					return 0;
				}
			}
			fclose(f);
		}
	}
	return r;
}

int getpwnam_r(const char *name, struct passwd *pwd, char *buf, size_t buflen, struct passwd **result) {
	static int (*real)(const char *, struct passwd *, char *, size_t, struct passwd **);
	if (!real) real = dlsym(RTLD_NEXT, "getpwnam_r");
	int r = real(name, pwd, buf, buflen, result);
	if (r != 0 || !*result) {
		char virt[4096]; snprintf(virt, sizeof(virt), "%s/passwd", fake_etc());
		FILE *f = fopen(virt, "r");
		if (f) {
			char line[1024];
			while (fgets(line, sizeof(line), f)) {
				if (strncmp(line, name, strlen(name)) == 0 && line[strlen(name)] == ':') {
					char *fields[7]; int nf = 0; char *s = line;
					fields[nf++] = s;
					for (char *c = line; *c && nf < 7; c++)
						if (*c == ':') { *c = 0; fields[nf++] = c + 1; }
					if (nf == 7) {
						pwd->pw_name = fields[0]; pwd->pw_passwd = fields[1];
						pwd->pw_uid = (uid_t)strtoul(fields[2], NULL, 10);
						pwd->pw_gid = (gid_t)strtoul(fields[3], NULL, 10);
						pwd->pw_gecos = fields[4]; pwd->pw_dir = fields[5]; pwd->pw_shell = fields[6];
						*result = pwd;
						fclose(f);
						return 0;
					}
				}
			}
			fclose(f);
		}
	}
	return r;
}
