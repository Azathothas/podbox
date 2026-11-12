/* Link stub for dlsym on aarch64, TODO/interpose.md T-1312 and T-1721.
 *
 * As dl-stub.S: it provides the SONAME binding, never an implementation.
 * The version node comes from dl-stub-aarch64.map (GLIBC_2.17, the first
 * version of glibc on aarch64). The body is `brk #0` so the impossible call
 * traps. This file is never shipped and never loaded.
 */
	.text
	.globl dlsym
	.type dlsym, %function
dlsym:
	brk #0
	.size dlsym, .-dlsym
