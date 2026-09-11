/* bench.c — the cross-platform checksum benchmark.
 *
 * 30M iterations of xorshift32 + double accumulation + FNV mixing,
 * compiled -O2 from the same source on every platform (host, chroot,
 * TCG guest). Prints elapsed seconds, Mops/s and a CHECKSUM.
 *
 * The checksum is the load-bearing output: it must be IDENTICAL on
 * every platform, which cross-validates the runs — a platform that
 * computed something different says so instead of looking faster.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static double now(void) {
	struct timespec ts;
	clock_gettime(CLOCK_MONOTONIC, &ts);
	return (double)ts.tv_sec + (double)ts.tv_nsec / 1e9;
}

int main(void) {
	const uint32_t N = 30000000u;
	uint32_t x = 2463534242u;
	uint64_t h = 2166136261u;
	double acc = 0.0;
	double t0 = now();
	for (uint32_t i = 0; i < N; i++) {
		x ^= x << 13; x ^= x >> 17; x ^= x << 5;	/* xorshift32 */
		acc += (double)(x & 0xFFFF) / 65535.0;
		h ^= x; h *= 16777619u;				/* FNV round */
	}
	double t1 = now();
	/* fold h to 32 bits for a stable printed checksum */
	uint32_t out = (uint32_t)(h ^ (h >> 32));
	printf("elapsed=%.3fs mops=%.1f checksum=%08x iters=%u\n",
	       t1 - t0, (double)N / (t1 - t0) / 1e6, out, N);
	return 0;
}
