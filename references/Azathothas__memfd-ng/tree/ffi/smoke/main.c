/* Link the memfd-ng shared library from C and test the ABI.
 * scripts/ffi-smoke.sh builds and runs this program. */
#include <stdio.h>
#include <string.h>
#include <sys/wait.h>
#include <errno.h>

#include "memfd-ng.h"

int main(void) {
    if (memfd_ng_abi_version() != 1) {
        fprintf(stderr, "test: unexpected ABI version\n");
        return 1;
    }
    printf("test: linked against memfd-ng %s\n", memfd_ng_version());

    /* Read the shell executable as the test image. */
    FILE *f = fopen("/bin/sh", "rb");
    if (!f) { perror("fopen /bin/sh"); return 1; }
    fseek(f, 0, SEEK_END);
    long len = ftell(f);
    fseek(f, 0, SEEK_SET);
    static unsigned char code[8 << 20];
    if (len < 0 || (size_t)len > sizeof code || fread(code, 1, (size_t)len, f) != (size_t)len) {
        fprintf(stderr, "test: read failed\n");
        return 1;
    }
    fclose(f);

    const char *argv[] = {"sh", "-c", "echo in-memory-from-C; exit 5", NULL};
    int32_t err = 0;
    memfd_ng_child *child = memfd_ng_spawn(code, (size_t)len, "smoke-sh", argv, NULL, &err);
    if (!child) {
        fprintf(stderr, "test: spawn failed: %s (%d)\n", strerror(-err), -err);
        return 1;
    }
    printf("test: child pid %d\n", memfd_ng_pid(child));

    int32_t status = 0;
    int rc = memfd_ng_wait(child, &status);
    if (rc != 0) {
        fprintf(stderr, "test: wait failed: %s\n", strerror(-rc));
        return 1;
    }
    if (!WIFEXITED(status) || WEXITSTATUS(status) != 5) {
        fprintf(stderr, "test: unexpected wait status %d\n", status);
        return 1;
    }
    printf("test: child exited with code 5\n");

    /* An invalid image must return -ENOEXEC through err_out. */
    const unsigned char bogus[] = {0x7f, 'E', 'L', 'F', 'n', 'o', 'p', 'e'};
    memfd_ng_child *bad = memfd_ng_spawn(bogus, sizeof bogus, "smoke-bogus", argv, NULL, &err);
    if (bad != NULL || err != -ENOEXEC) {
        fprintf(stderr, "test: expected NULL/-ENOEXEC, got %p/%d\n", (void *)bad, err);
        return 1;
    }
    printf("test: invalid image returned ENOEXEC\n");

    printf("test: all checks passed\n");
    return 0;
}
