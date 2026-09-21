/* File I/O: write 64 MiB of patterned chunks to io.bin in the working
 * directory, read it back, and mix every byte into an FNV-1a checksum.
 * Self-times and prints the checksum, so three platforms running one
 * binary must print one checksum. The backing store differs per platform
 * (named in the driver's conditions); the checksum must not. Written for
 * experiments/154-tcg-workload-spread.sh; sizes are pinned here. */
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

static double now(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (double)t.tv_sec + (double)t.tv_nsec / 1e9;
}

#define CHUNK (1024 * 1024)
#define NCHUNK 64

int main(void) {
    static unsigned char chunk[CHUNK];
    uint64_t h = 0xcbf29ce484222325ull;
    int fd = open("io.bin", O_WRONLY | O_CREAT | O_TRUNC, 0600);
    if (fd < 0) {
        printf("workload=io error=open-write\n");
        return 1;
    }
    double t0 = now();
    for (int c = 0; c < NCHUNK; c++) {
        for (int i = 0; i < CHUNK; i++) {
            chunk[i] = (unsigned char)(((size_t)c * CHUNK + i) * 31 & 0xff);
        }
        if (write(fd, chunk, CHUNK) != CHUNK) {
            printf("workload=io error=short-write\n");
            return 1;
        }
    }
    close(fd);
    fd = open("io.bin", O_RDONLY);
    if (fd < 0) {
        printf("workload=io error=open-read\n");
        return 1;
    }
    ssize_t r;
    while ((r = read(fd, chunk, CHUNK)) > 0) {
        for (ssize_t i = 0; i < r; i++) {
            h ^= chunk[i];
            h *= 0x100000001b3ull;
        }
    }
    close(fd);
    unlink("io.bin");
    double dt = now() - t0;
    printf("workload=io mib=%d mibs=%.1f checksum=%016llx elapsed=%.3f\n",
           NCHUNK, (double)(2 * NCHUNK) / dt, (unsigned long long)h, dt);
    return 0;
}
