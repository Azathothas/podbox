// env completion: this sandbox has no /etc/passwd and the root fs is not
// writable, so any subject that resolves the invoking user dies at startup
// (observed first with lima v2.2.0: osutil.init() must.Must panic). We cannot
// create /etc/passwd (mounts owned outside the user namespace), so serve a
// virtual one: intercept getpwuid_r/getpwnam_r (cgo Go, glibc consumers) and
// open/open64/openat (everyone else, path-redirect).
//
// build: gcc -shared -fPIC -o libfakepasswd.so libfakepasswd.c -ldl
// use:   LD_PRELOAD=.../libfakepasswd.so <subject>
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdarg.h>
#include <string.h>
#include <pwd.h>
#include <grp.h>
#include <sys/types.h>
#include <errno.h>

#ifndef FAKE_ETC
#define FAKE_ETC "/workspace/vm-research/experiments/work/fake-etc"
#endif

static const char *redirect(const char *path) {
  if (!path) return path;
  if (!strcmp(path, "/etc/passwd")) return FAKE_ETC "/passwd";
  if (!strcmp(path, "/etc/group"))  return FAKE_ETC "/group";
  return path;
}

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
  const char *p = redirect(path);
  if (flags & O_CREAT) {
    va_list ap; va_start(ap, flags);
    mode_t m = va_arg(ap, mode_t);
    va_end(ap);
    return real_openat(dirfd, p, flags, m);
  }
  return real_openat(dirfd, p, flags);
}

/* cgo Go / glibc consumers */
int getpwuid_r(uid_t uid, struct passwd *pwd, char *buf, size_t buflen, struct passwd **result) {
  static int (*real_fn)(uid_t, struct passwd *, char *, size_t, struct passwd **);
  if (uid != 0) {
    if (!real_fn) real_fn = dlsym(RTLD_NEXT, "getpwuid_r");
    return real_fn(uid, pwd, buf, buflen, result);
  }
  const char *name = "root", *dir = "/state/home", *shell = "/bin/sh", *gecos = "root";
  size_t need = strlen(name) + 1 + strlen(dir) + 1 + strlen(shell) + 1 + strlen(gecos) + 1;
  if (buflen < need) { errno = ERANGE; *result = NULL; return ERANGE; }
  pwd->pw_uid = 0; pwd->pw_gid = 0;
  char *w = buf;
#define CPY(field, src) do { field = w; size_t l = strlen(src) + 1; memcpy(w, src, l); w += l; } while (0)
  CPY(pwd->pw_name, name); CPY(pwd->pw_passwd, "x"); CPY(pwd->pw_gecos, gecos);
  CPY(pwd->pw_dir, dir);   CPY(pwd->pw_shell, shell);
#undef CPY
  *result = pwd;
  return 0;
}

int getpwnam_r(const char *name, struct passwd *pwd, char *buf, size_t buflen, struct passwd **result) {
  if (name && !strcmp(name, "root")) return getpwuid_r(0, pwd, buf, buflen, result);
  static int (*real_fn)(const char *, struct passwd *, char *, size_t, struct passwd **);
  if (!real_fn) real_fn = dlsym(RTLD_NEXT, "getpwnam_r");
  return real_fn(name, pwd, buf, buflen, result);
}

/* plain (non-_r) API — used by openssh tools, most C consumers */
static struct passwd pw_entry;
struct passwd *getpwuid(uid_t uid) {
  static char buf[512];
  struct passwd *res;
  if (getpwuid_r(uid, &pw_entry, buf, sizeof(buf), &res) != 0 || !res) return NULL;
  return &pw_entry;
}
static struct passwd pwn_entry;
struct passwd *getpwnam(const char *name) {
  static char buf[512];
  struct passwd *res;
  if (getpwnam_r(name, &pwn_entry, buf, sizeof(buf), &res) != 0 || !res) return NULL;
  return &pwn_entry;
}
static struct group gr_entry;
static char *gr_members[1] = { NULL };
struct group *getgrgid(gid_t gid) {
  static char buf[512];
  struct group *res;
  if (getgrgid_r(gid, &gr_entry, buf, sizeof(buf), &res) != 0 || !res) return NULL;
  gr_entry.gr_mem = gr_members;
  return &gr_entry;
}

int getgrgid_r(gid_t gid, struct group *grp, char *buf, size_t buflen, struct group **result) {
  if (gid != 0) {
    static int (*real_fn)(gid_t, struct group *, char *, size_t, struct group **);
    if (!real_fn) real_fn = dlsym(RTLD_NEXT, "getgrgid_r");
    return real_fn(gid, grp, buf, buflen, result);
  }
  const char *name = "root";
  if (buflen < strlen(name) + 2) { errno = ERANGE; *result = NULL; return ERANGE; }
  grp->gr_gid = 0;
  grp->gr_name = buf;
  strcpy(buf, name);
  char *mem = buf + strlen(name) + 1;
  mem[0] = '\0';
  grp->gr_passwd = mem;      /* empty passwd */
  grp->gr_mem = NULL;        /* no member list */
  *result = grp;
  return 0;
}
