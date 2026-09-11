/* LD_PRELOAD shim for chroots without a real devtmpfs: this sandbox allows
   mknod only for char 0:0, so /dev/urandom inside the chroot is an empty
   regular file. nix 2.2.2 (libstdc++ std::random_device, openssl RAND_poll)
   reads /dev/urandom directly and dies ("random_device could not be read").
   We intercept opens of /dev/{u}random and serve reads from getrandom(2). */
#define _GNU_SOURCE
#include <errno.h>
#include <string.h>
#include <stddef.h>
#include <stdarg.h>
#include <fcntl.h>
#include <sys/syscall.h>
#include <unistd.h>

static int marked[4096];

static int is_urandom(const char *p) {
    return p && (strcmp(p, "/dev/urandom") == 0 || strcmp(p, "/dev/random") == 0);
}
static int getrand(void *buf, size_t len) {
    size_t off = 0;
    while (off < len) {
        ssize_t n = syscall(SYS_getrandom, (char*)buf + off, len - off, 0);
        if (n < 0) { if (errno == EINTR || errno == EAGAIN) continue; return -1; }
        off += (size_t)n;
    }
    return 0;
}

static int do_open(const char *path, int flags, mode_t mode) {
    int fd = (int)syscall(SYS_openat, AT_FDCWD, path, flags, mode);
    if (fd >= 0 && is_urandom(path) && fd < 4096) marked[fd] = 1;
    return fd;
}
int open(const char *path, int flags, ...) {
    va_list ap; va_start(ap, flags); mode_t m = va_arg(ap, int); va_end(ap);
    return do_open(path, flags, m);
}
int open64(const char *path, int flags, ...) {
    va_list ap; va_start(ap, flags); mode_t m = va_arg(ap, int); va_end(ap);
    return do_open(path, flags, m);
}
ssize_t read(int fd, void *buf, size_t n) {
    if (fd >= 0 && fd < 4096 && marked[fd]) {
        if (getrand(buf, n) < 0) return -1;
        return (ssize_t)n;
    }
    return (ssize_t)syscall(SYS_read, fd, buf, n);
}
int close(int fd) {
    if (fd >= 0 && fd < 4096) marked[fd] = 0;
    return (int)syscall(SYS_close, fd);
}
