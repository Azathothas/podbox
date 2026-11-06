/* Link stub for dlsym, TODO/interpose.md T-1312.
 *
 * It provides the SONAME binding, never an implementation: the link records
 * `dlsym` against `libdl.so.2`, and at runtime the loader serves the real
 * library, which defines the symbol on both sides of the 2.34 merge (before
 * it `dlsym` lived in libdl; after it glibc keeps libdl as a compatibility
 * object). The `2` is libdl's own SONAME version, unrelated to the
 * `libc.so.6` beside it.
 *
 * The version node comes from `dl-stub.map`, not from `.symver`: the
 * assembler directive left no node behind, which the link proved with
 * `version node not found for symbol dlsym@GLIBC_2.2.5`. The body is
 * `int3` so the impossible call traps instead of returning a garbage
 * address. This file is never shipped and never loaded.
 */
	.text
	.globl dlsym
	.type dlsym, @function
dlsym:
	int3
	.size dlsym, .-dlsym
