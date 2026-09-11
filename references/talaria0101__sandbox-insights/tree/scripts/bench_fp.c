/* bench_fp.c — the FP-heavy counterpart to bench.c.
 *
 * 30M iterations of a dependent double multiply-add chain (an FP pipe
 * that software emulation cannot hide behind integer throughput).
 * Pairing bench.c (integer-dominated) with this one measures how the
 * TCG tax is shaped by the workload, not just its magnitude.
 * Same checksum discipline: identical across platforms or the run is
 * invalid.
 */
#include <stdint.h>
#include <stdio.h>
#include <time.h>

static double now(void) {
	struct timespec ts;
	clock_gettime(CLOCK_MONOTONIC, &ts);
	return (double)ts.tv_sec + (double)ts.tv_nsec / 1e9;
}

int main(void) {
	const uint32_t N = 30000000u;
	double a = 1.0000001, b = 0.9999999, acc = 1.0;
	double t0 = now();
	for (uint32_t i = 0; i < N; i++) {
		acc = acc * a + b;	/* dependent chain: no ILP to hide behind */
		/* keep acc bounded without branching */
		if (acc > 1e10) acc *= 1e-10;
	}
	double t1 = now();
	printf("elapsed=%.3fs mops=%.1f checksum=%08x acc=%.6e iters=%u\n",
	       t1 - t0, (double)N / (t1 - t0) / 1e6,
	       (uint32_t)(acc * 1000.0) & 0xffffffffu, acc, N);
	return 0;
}
