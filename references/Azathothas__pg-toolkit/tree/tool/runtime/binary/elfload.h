/* binary/elfload.h -- the shape of the compiled-in ELF loader and its providers.
 *
 * Shared by binary/elfload.c, which is the loader; by binary/dlopen.c, which falls
 * through to it; and by pgb-provider-table.c, which `pgb` GENERATES per build.
 * Three files that have to agree on a layout get one header, or they agree
 * until somebody edits one of them.
 *
 * SPDX-License-Identifier: 0BSD
 */
#ifndef PGT_BINARY_ELFLOAD_H
#define PGT_BINARY_ELFLOAD_H

#include <stddef.h>

/* One symbol this executable can define for a loaded object. `addr` is NULL
 * when the name was in the generated list but the link did not pull the
 * archive member that defines it -- see the weak-reference note below. */
struct pgb_provider_sym {
    const char *name;
    void       *addr;
};

/* ⭐ WEAK, and the weakness IS the mechanism, not a convenience.
 *
 * The generated table takes the address of every name it lists. A strong
 * reference to each would force the archive member defining it out of libc.a
 * and into the link -- all of libc.a, for every program, whether or not any
 * plugin ever needed it. A weak undefined reference does not pull an archive
 * member: it resolves to 0 if nothing else in the link needed that symbol,
 * and to the real address if something did.
 *
 * So the table costs its own strings and pointers and nothing else, and the
 * decision about how much of libc.a to link is made SEPARATELY, by the -u
 * list `pgb` passes. That separation is what makes the size cost a dial
 * rather than a constant. Measured in experiments/76-.
 */
extern const struct pgb_provider_sym pgb_provider_syms[] __attribute__((weak));

/* Sonames this executable satisfies internally. A DT_NEEDED naming one of
 * these is answered out of the provider table instead of being mapped, which
 * is what keeps a second libc from entering the process. NULL-terminated. */
extern const char *const pgb_provider_sonames[] __attribute__((weak));

/* ⛔ WEAK, and without it `--wrap-dlopen` alone does not link.
 *
 * binary/dlopen.c is linked by BOTH opt-ins: `--wrap-dlopen`, for a program's own
 * plugins, and `--host-dlopen`, which adds the loader. pgb_elf_available()
 * below exists precisely so the first can be built WITHOUT the second -- but
 * a strong undefined reference fails the link before that check ever runs.
 *
 * ⚠ it looked like it worked for as long as the two were built in that order.
 * The runtime objects are cached in a directory keyed on the COMPILER, so a
 * previous --host-dlopen build left elfload.o there and a later
 * --wrap-dlopen build linked it by name. Change compiler -- which is what
 * moving the pin does -- and the same POC fails with five undefined
 * references. Caught by poc/70-sqlite-extensions and poc/80-mlt against
 * pgb-env-debian-trixie, on the first build in a fresh runtime directory.
 *
 * ⚠ PGT_BINARY_ELFLOAD_IMPL keeps the DEFINITIONS strong: a weak definition would
 * let anything else in the link silently replace the loader.
 */
#ifdef PGT_BINARY_ELFLOAD_IMPL
#define PGT_BINARY_ELF_WEAK
#else
#define PGT_BINARY_ELF_WEAK __attribute__((weak))
#endif

/* The loader. binary/dlopen.c calls these after its own compiled-in plugin table
 * misses; nothing else should. Returns NULL / sets the error string. */
void       *pgb_elf_dlopen(const char *path, int flags) PGT_BINARY_ELF_WEAK;
void       *pgb_elf_dlsym(void *handle, const char *name) PGT_BINARY_ELF_WEAK;
int         pgb_elf_dlclose(void *handle) PGT_BINARY_ELF_WEAK;
const char *pgb_elf_dlerror(void) PGT_BINARY_ELF_WEAK;

/* ⭐ solo's mechanism 5: make "what did this binary satisfy internally"
 * observable from INSIDE, beside the syscall trace pg-toolkit binary verify takes from
 * outside. Two independent instruments on the same question. Writes an
 * ldd-format listing to fd, including names served WITHOUT a mapping. */
void pgb_elf_trace_loaded(int fd) PGT_BINARY_ELF_WEAK;

/* Non-zero once a provider table is compiled in and non-empty. binary/dlopen.c
 * uses it to decide whether falling through is even possible, so that a build
 * without the loader keeps its existing honest error instead of a new one.
 *
 * ⛔ Call it through pgb_elf_linked() in binary/dlopen.c, never directly: when
 * the loader is not linked this symbol's address is 0, and calling through it
 * is a jump to NULL rather than a "no". */
int pgb_elf_available(void) PGT_BINARY_ELF_WEAK;

#endif /* PGT_BINARY_ELFLOAD_H */
