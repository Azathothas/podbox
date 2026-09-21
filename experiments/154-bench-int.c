/* Integer loop: xorshift32 with double accumulation and FNV-1a mixing.
 * Self-times with CLOCK_MONOTONIC and prints its checksum, so three
 * platforms running one binary must print one checksum. Written for
 * experiments/154-tcg-workload-spread.sh; the iteration count is pinned
 * here, not taken from argv, so a citation of the script keeps meaning
 * what it meant. */
#include <stdint.h>
#include <stdio.h>
#include <time.h>

static double now(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (double)t.tv_sec + (double)t.tv_nsec / 1e9;
}

int main(void) {
    uint32_t x = 0x12345678u;
    uint64_t h = 0xcbf29ce484222325ull;
    double acc = 0.0;
    const long n = 100000000L;
    double t0 = now();
    for (long i = 0; i < n; i++) {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        acc += (double)x * 1.6180339887;
        h ^= (uint64_t)x;
        h *= 0x100000001b3ull;
    }
    double dt = now() - t0;
    printf("workload=int iters=%ld mops=%.1f checksum=%016llx acc=%.6e elapsed=%.3f\n",
           n, (double)n / dt / 1e6, (unsigned long long)h, acc, dt);
    return 0;
}
