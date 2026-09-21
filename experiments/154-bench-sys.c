/* Syscall loop: sched_yield is a real system call on every Linux
 * architecture (unlike the vDSO clocks), so this measures the
 * trap-and-return path rather than a userspace shortcut. Self-times and
 * prints a checksum of the iteration count. Written for
 * experiments/154-tcg-workload-spread.sh; the count is pinned here. */
#define _GNU_SOURCE
#include <sched.h>
#include <stdint.h>
#include <stdio.h>
#include <time.h>

static double now(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (double)t.tv_sec + (double)t.tv_nsec / 1e9;
}

int main(void) {
    const long n = 2000000L;
    uint64_t h = 0xcbf29ce484222325ull;
    double t0 = now();
    for (long i = 0; i < n; i++) {
        sched_yield();
        h ^= (uint64_t)(i + 1);
        h *= 0x100000001b3ull;
    }
    double dt = now() - t0;
    printf("workload=sys iters=%ld kops=%.1f checksum=%016llx elapsed=%.3f\n",
           n, (double)n / dt / 1e3, (unsigned long long)h, dt);
    return 0;
}
