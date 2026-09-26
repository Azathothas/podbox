// The T-0501 opener: open a path with exact flags and report.
//
// Usage: opener PATH FLAGS8
// FLAGS8 is the octal open(2) flags (O_RDONLY=0, O_WRONLY=1, O_RDWR=2,
// O_CREAT=0100, O_EXCL=0200, O_TRUNC=01000). Prints `rc=N` with the
// descriptor number, or `rc=-1 errno=E (name)` and exits 1, so the drive
// asserts the kernel-exact answer per flag shape rather than whatever a
// shell redirection happens to pass.
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: opener PATH FLAGS8\n");
        return 2;
    }
    unsigned long flags = strtoul(argv[2], 0, 8);
    int fd = open(argv[1], (int)flags, 0644);
    if (fd < 0) {
        printf("rc=-1 errno=%d (%s)\n", errno, strerror(errno));
        return 1;
    }
    printf("rc=%d\n", fd);
    return 0;
}
