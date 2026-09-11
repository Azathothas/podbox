/* nix-pty-shim.c — LD_PRELOAD shim that emulates the pseudoterminal Nix
 * insists on for builder output, plus a /proc/self/exe escape hatch.
 *
 * Why: since Nix 2.3.0, startBuilder() unconditionally does
 *     builderOut = posix_openpt(O_RDWR|O_NOCTTY);
 *     slaveName = ptsname_r(...); ... open(slaveName) ... tcsetattr(raw)
 * On hosts without a usable /dev/ptmx+devpts (this sandbox: no ptmx node,
 * mount(2) blocked -> devpts unmountable, mknod of char 5:2 blocked) every
 * Nix >= 2.3 fails at "opening pseudoterminal master". Emulating the pty
 * with a socketpair removes that wall without any kernel cooperation:
 *
 *   posix_openpt()  -> socketpair(); master = sv[0], stash sv[1]
 *   grantpt/unlockpt-> success (nothing to unlock on a socket)
 *   ptsname{,_r}()  -> fake path, e.g. /dev/.nix-pty-emu/0
 *   open(fake path) -> dup(stashed slave fd)   [same process image]
 *   tcgetattr/tcsetattr on marked fds -> fake success (sockets give ENOTTY)
 *   readlink("/proc/self/exe") -> $NIX_SHIM_EXE if set (no procfs in chroot)
 *
 * Scope/limits (documented in docs/latest-nix-shim.md):
 *  - only works for DYNAMIC Nix binaries (static ones ignore LD_PRELOAD);
 *  - one emulated pty at a time (Nix runs at most one builder per process
 *    with max-jobs=1; the shim serialises);
 *  - the fake slave path only resolves through the shim, i.e. in the Nix
 *    process image — which is sufficient because Nix opens the slave itself
 *    and dup2()s it onto the builder's stdio before exec.
 * Build: gcc -shared -fPIC -O2 -o nix-pty-shim.so nix-pty-shim.c -lpthread
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <stdarg.h>
#include <stddef.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/syscall.h>
#include <termios.h>
#include <unistd.h>
#include <stdlib.h>
#include <stdio.h>

#define FAKE_PTY_PATH "/dev/.nix-pty-emu/0"
#define FD_TABLE_SIZE 4096

static pthread_mutex_t g_lock = PTHREAD_MUTEX_INITIALIZER;
static int g_master = -1;   /* sv[0], returned by posix_openpt */
static int g_slave = -1;    /* live slave fd, handed out for open(FAKE_PTY_PATH) */
static int g_in_use = 0;    /* is an emulated pty currently allocated? */
static int g_is_child = 0;  /* set in the forked builder child */
static char g_marked[FD_TABLE_SIZE]; /* fds that are our master or slave */

static int dbg_on(void) {
    static int v = -1;
    if (v < 0) { const char *e = getenv("NIX_SHIM_DEBUG"); v = e && *e ? 1 : 0; }
    return v;
}
static void dbg(const char *fmt, ...) {
    if (!dbg_on()) return;
    FILE *f = fopen("/tmp/nix-shim.log", "a");
    if (!f) return;
    char cmdline[256] = {0};
    int cfd = open("/proc/self/cmdline", O_RDONLY);
    if (cfd >= 0) {
        ssize_t n = read(cfd, cmdline, sizeof(cmdline) - 1);
        if (n > 0) cmdline[n] = 0;
        close(cfd);
        for (int i = 0; i < (int)n; i++) if (cmdline[i] == 0) cmdline[i] = ' ';
    }
    va_list ap; va_start(ap, fmt);
    fprintf(f, "[pid %d ppid %d cmd %s] ", (int)getpid(), (int)getppid(), cmdline);
    vfprintf(f, fmt, ap);
    fprintf(f, "\n");
    va_end(ap);
    fclose(f);
}

static void mark(int fd) { if (fd >= 0 && fd < FD_TABLE_SIZE) g_marked[fd] = 1; }
static void unmark(int fd) { if (fd >= 0 && fd < FD_TABLE_SIZE) g_marked[fd] = 0; }

static int is_fake_pty(const char *path) {
    return path && strcmp(path, FAKE_PTY_PATH) == 0;
}

/* ------------------------------------------------------------------ pty */

int posix_openpt(int flags) {
    int sv[2];
    (void)flags;
    pthread_mutex_lock(&g_lock);
    if (g_in_use) { pthread_mutex_unlock(&g_lock); errno = EAGAIN; return -1; }
    if (socketpair(AF_UNIX, SOCK_STREAM, 0, sv) < 0) {
        pthread_mutex_unlock(&g_lock);
        return -1;
    }
    g_master = sv[0];
    g_slave = sv[1];
    g_in_use = 1;
    mark(sv[0]);
    mark(sv[1]);
    dbg("posix_openpt -> master=%d slave=%d", sv[0], sv[1]);
    pthread_mutex_unlock(&g_lock);
    return sv[0];
}

int grantpt(int fd) { (void)fd; return 0; }
int unlockpt(int fd) { (void)fd; return 0; }

char *ptsname(int fd) {
    static char buf[sizeof(FAKE_PTY_PATH)];
    pthread_mutex_lock(&g_lock);
    if (fd != g_master) { pthread_mutex_unlock(&g_lock); errno = EBADF; return NULL; }
    strcpy(buf, FAKE_PTY_PATH);
    pthread_mutex_unlock(&g_lock);
    return buf;
}

int ptsname_r(int fd, char *buf, size_t buflen) {
    if ((size_t)sizeof(FAKE_PTY_PATH) > buflen) { errno = ERANGE; return ERANGE; }
    pthread_mutex_lock(&g_lock);
    if (fd != g_master) { pthread_mutex_unlock(&g_lock); errno = EBADF; return -1; }
    strcpy(buf, FAKE_PTY_PATH);
    pthread_mutex_unlock(&g_lock);
    return 0;
}

/* ------------------------------------------------------------- open family */

static int maybe_fake_open(const char *path) {
    if (!is_fake_pty(path) || g_is_child)
        return -1;
    pthread_mutex_lock(&g_lock);
    int fd = -1;
    if (g_in_use && g_slave >= 0) {
        fd = dup(g_slave); /* a fresh fd connected to the same socket */
        if (fd >= 0) {
            mark(fd);
            dbg("open(fake pty) -> dup slave %d as %d", g_slave, fd);
            /* Transfer ownership: close the original slave fd so the ONLY
               slave-side holders are nix's writeSide and (transiently) the
               builder. Otherwise the parent holds sv[1] forever and the
               master never sees EOF after the builder exits. */
            syscall(SYS_close, g_slave);
            if (g_slave >= 0 && g_slave < FD_TABLE_SIZE) g_marked[g_slave] = 0;
            g_slave = fd;
        }
    } else {
        errno = ENOENT;
    }
    pthread_mutex_unlock(&g_lock);
    return fd;
}

int open(const char *path, int flags, ...) {
    int fd = maybe_fake_open(path);
    if (fd >= 0) return fd;
    va_list ap; va_start(ap, flags); mode_t m = va_arg(ap, int); va_end(ap);
    return (int)syscall(SYS_openat, AT_FDCWD, path, flags, m);
}
int open64(const char *path, int flags, ...) {
    int fd = maybe_fake_open(path);
    if (fd >= 0) return fd;
    va_list ap; va_start(ap, flags); mode_t m = va_arg(ap, int); va_end(ap);
    return (int)syscall(SYS_openat, AT_FDCWD, path, flags, m);
}
int openat(int dirfd, const char *path, int flags, ...) {
    int fd = maybe_fake_open(path);
    if (fd >= 0) return fd;
    va_list ap; va_start(ap, flags); mode_t m = va_arg(ap, int); va_end(ap);
    return (int)syscall(SYS_openat, dirfd, path, flags, m);
}

/* ------------------------------------------------------- terminal ioctls */

int tcgetattr(int fd, struct termios *t) {
    if (fd >= 0 && fd < FD_TABLE_SIZE && g_marked[fd]) {
        /* Nix only wants to switch the slave into raw mode; report a
           plausible terminal and let tcsetattr succeed. */
        memset(t, 0, sizeof(*t));
        t->c_iflag = ICRNL | IXON;
        t->c_oflag = OPOST | ONLCR;
        t->c_cflag = CS8 | CREAD;
        t->c_lflag = ICANON | ECHO | ISIG;
        return 0;
    }
    errno = ENOTTY;
    return -1;
}

int tcsetattr(int fd, int act, const struct termios *t) {
    (void)act; (void)t;
    if (fd >= 0 && fd < FD_TABLE_SIZE && g_marked[fd]) return 0;
    errno = ENOTTY;
    return -1;
}

int isatty(int fd) {
    if (fd >= 0 && fd < FD_TABLE_SIZE && g_marked[fd]) return 1;
    /* delegate: real isatty does an ioctl; emulate via TIOC? keep syscall */
    return (int)syscall(SYS_ioctl, fd, 0x5401 /* TCGETS */, (size_t)0);
}

/* ------------------------------------------------------------- vfork ---- */

/* Nix starts builders with vfork() when possible. A vforked child shares the
   parent's memory, so our bookkeeping (g_master/g_slave/g_marked) and the
   pthread mutex would be mutated by the child in the parent's live state.
   A plain fork() is semantically identical for nix's purposes here. */
pid_t fork(void); /* glibc fork: runs our pthread_atfork handlers */

int vfork(void) {
    return fork();
}

/* ------------------------------------------------------- /proc/self/exe */

ssize_t readlink(const char *path, char *buf, size_t bufsz) {
    const char *over = getenv("NIX_SHIM_EXE");
    if (over && strcmp(path, "/proc/self/exe") == 0) {
        size_t n = strlen(over);
        if (n > bufsz) n = bufsz;
        memcpy(buf, over, n);
        return (ssize_t)n;
    }
    return (ssize_t)syscall(SYS_readlinkat, AT_FDCWD, path, buf, bufsz);
}

/* ------------------------------------------------------------- cleanup */

int close(int fd) {
    if (fd >= 0 && fd < FD_TABLE_SIZE) {
        pthread_mutex_lock(&g_lock);
        if (g_marked[fd]) {
            dbg("close(marked fd=%d) master=%d slave=%d is_child=%d", fd, g_master, g_slave, g_is_child);
            g_marked[fd] = 0;
            if (g_is_child) {
                /* child: never touch the shared bookkeeping or fds */
            } else {
                if (fd == g_master) {
                    /* nix closed the master: build output done; tear down */
                    if (g_slave >= 0 && g_slave < FD_TABLE_SIZE && g_marked[g_slave]) {
                        g_marked[g_slave] = 0;
                        syscall(SYS_close, g_slave);
                    }
                    g_master = -1; g_slave = -1; g_in_use = 0;
                } else if (fd == g_slave) {
                    /* writeSide closed early (right after childStarted) */
                    g_slave = -1;
                }
            }
        }
        pthread_mutex_unlock(&g_lock);
    }
    return (int)syscall(SYS_close, fd);
}
