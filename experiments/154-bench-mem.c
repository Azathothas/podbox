/* Memory bandwidth: fill 16 MiB with a strided pattern and mix it into
 * an FNV-1a checksum, 64 passes. Self-times and prints the checksum, so
 * three platforms running one binary must print one checksum. Written for
 * experiments/154-tcg-workload-spread.sh; sizes are pinned here. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static double now(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (double)t.tv_sec + (double)t.tv_nsec / 1e9;
}

int main(void) {
    static unsigned char buf[16 * 1024 * 1024];
    const int passes = 64;
    uint64_t h = 0xcbf29ce484222325ull;
    double t0 = now();
    for (int p = 0; p < passes; p++) {
        for (size_t i = 0; i < sizeof(buf); i += 64) {
            buf[i] = (unsigned char)((i * 31 + p) & 0xff);
        }
        for (size_t i = 0; i < sizeof(buf); i++) {
            h ^= buf[i];
            h *= 0x100000001b3ull;
        }
    }
    double dt = now() - t0;
    double mib = (double)sizeof(buf) * passes / (1024.0 * 1024.0);
    printf("workload=mem mib=%.0f mibs=%.1f checksum=%016llx elapsed=%.3f\n",
           mib, mib / dt, (unsigned long long)h, dt);
    return 0;
}
