/* binary/dlopen.c -- let a static binary load its OWN plugins, with no loader.
 *
 * The problem, measured in experiments/50-host-plugin-feasibility.sh
 * -------------------------------------------------------------------------
 * A static glibc binary has no dynamic loader of its own, so dlopen() borrows
 * the HOST's ld.so and libc.so.6 -- and that pairing is what breaks. Across
 * the eleven pinned environments, dlopen of a shared object succeeds on two
 * and dies inside glibc's loader on the rest:
 *
 *   dl-call-libc-early-init.c:37  sym != NULL   Debian 11, Ubuntu 20.04
 *   dl-machine.h:487              assertion     Rocky 8
 *   signal 8                                    openSUSE Leap, Fedora 42
 *
 * ⛔ and success IS the worse outcome. Where it loads, a second libc and the
 * host loader are now in the process, which is the exact thing this project
 * exists to keep out. docs/limitations.md section 1.
 *
 * The split that makes this tractable
 * -------------------------------------------------------------------------
 * ⭐ Two different problems wear the same word. Loading a HOST plugin --
 * libnss_ldap, a distribution's GTK module -- means running code that was
 * built somewhere else against a libc that is not ours, and that is the hard
 * one. Loading the program's OWN plugins, shipped by the same build, is not
 * hard at all: the code is already available at link time. The only thing
 * dlopen is providing there is a name-to-function lookup, and a lookup does
 * not need a loader.
 *
 * So this file answers dlopen/dlsym/dlclose/dlerror out of a table the build
 * compiled in. Nothing is mapped, nothing is relocated, no ld.so is consulted
 * and no second libc can enter. The plugins are ordinary objects in the link.
 *
 * Prior art, and what was taken from it
 * -------------------------------------------------------------------------
 * references/allyourcodebase__pipewire/tree/src/wrap/dlfcn.zig does exactly
 * this for PipeWire's two plugin systems, by hand, in Zig. Taken from it:
 * the handle-is-a-pointer-to-the-table-entry representation, the no-op
 * dlclose, the insistence that dlerror is part of the interface and not an
 * extra, and its comptime assertion that no handle collides with RTLD_DEFAULT
 * or RTLD_NEXT -- reproduced here as a runtime check, since C cannot make it
 * at compile time.
 *
 * ⭐ what IS different: that table is hand-written for one program. `pgb`
 * GENERATES this one with `nm` from the objects the build produced, which is
 * what turns a PipeWire-specific patch into a mechanism. POC 50 links
 * CPython's 49 extension modules in by hand through CPython's own
 * Modules/Setup.local; this is that, for programs with no such mechanism.
 *
 * ⭐ the redirection IS a link-time one, so no source changes, exactly as in
 * binary/iconv.c: -Wl,--wrap=dlopen,... makes ld rewrite every UNDEFINED
 * reference to those names into __wrap_*, so calls from the application, from
 * any static library in the link, and from libraries compiled long before this
 * tool existed are all caught, and none of them has to have seen a pgb header.
 *
 * ⭐ why this FILE IS in an archive. An archive member is pulled in only when
 * something references a symbol it defines. A program that never calls dlopen
 * links none of this. Same reasoning as binary/iconv.c.
 *
 * ⚠ what this does not do, stated rather than discovered later:
 *
 *   - it does not load a HOST plugin, and it is not trying to. That is
 *     docs/AGENTS.md item 4, routes B and C.
 *   - dlopen(NULL), "a handle for the main program", cannot be answered from
 *     a generated table unless the build put a "@SELF" entry in it, because
 *     nothing here can enumerate the executable's own symbols. Unmatched, it
 *     reports so through dlerror() rather than returning a handle whose
 *     dlsym silently answers NULL for everything.
 *   - RTLD_DEFAULT and RTLD_NEXT are a search order over loaded objects.
 *     There is no such order here. They are reported through dlerror(),
 *     never faked.
 *   - dlvsym(), dlinfo() and dladdr() are not wrapped. A program that calls
 *     them keeps whatever glibc's static build does with them, which is the
 *     pre-existing behaviour rather than something this file changed.
 *
 * SPDX-License-Identifier: 0BSD
 */

#include "dlopen.h"
#include "elfload.h"

#include <string.h>

/* dlfcn.h is not included: this file must compile in an environment whose
 * dlfcn.h has already been seen by the wrapped program, and the only two
 * values needed are ABI constants that have never changed on Linux. */
#define PGT_BINARY_RTLD_DEFAULT ((void *)0)
#define PGT_BINARY_RTLD_NEXT    ((void *)-1L)

/* ⛔ dlerror() IS part of the interface, not a courtesy. dlopen is allowed to
 * return NULL for reasons a caller must be able to distinguish, and a program
 * that gets NULL with no message has been handed a silent failure. Every
 * failure path below sets this. */
static const char *pgb_dl_err;

/* Cleared by a read, as dlerror() is specified to do. */
char *__wrap_dlerror(void)
{
    const char *e = pgb_dl_err;
    pgb_dl_err = NULL;
    return (char *)e;
}

static const char *pgb_basename(const char *p)
{
    const char *s = strrchr(p, '/');
    return s ? s + 1 : p;
}

/* ⚠ the matching rule IS not plain equality, and it cannot be.
 *
 * A program reaches its plugins by a path it built at run time -- from a
 * configure-time PKGLIBDIR, from an environment variable, from a directory
 * scan. The generated table knows what the BUILD produced. Requiring the two
 * strings to be equal would mean the table only ever matched programs that
 * hard-code a bare soname, which is the minority.
 *
 * So: exact match first, because a table entry written as a full path should
 * beat a basename collision; then basename, which is what actually fires for
 * `dlopen("/usr/lib/myapp/plugins/foo.so")` against a table built from
 * `foo.so`.
 *
 * ⛔ Basename matching is deliberately NOT extended to stripping "lib" or a
 * version suffix. `foo.so` and `foo.so.2` are different objects to the
 * program that named one of them, and a lookup that quietly returned the
 * other would be a silent wrong answer -- the failure mode this whole project
 * is about.
 */
static const struct pgb_dl_lib *pgb_find(const char *path)
{
    const struct pgb_dl_lib *l;

    if (&pgb_dlopen_libs[0] == NULL)
        return NULL;

    for (l = pgb_dlopen_libs; l->name; l++)
        if (strcmp(l->name, path) == 0)
            return l;

    {
        const char *base = pgb_basename(path);
        for (l = pgb_dlopen_libs; l->name; l++)
            if (strcmp(pgb_basename(l->name), base) == 0)
                return l;
    }
    return NULL;
}

/* ⭐ handles from two sources have to be told apart, and there is no bit to
 * spare in either. A table handle points into pgb_dlopen_libs; a loader handle
 * points at the loader's own object record. dlsym must send each to the right
 * place, so the loader's handles are recorded here as they are issued. Small
 * and linear: a program that dlopens more than a few dozen HOST objects is not
 * a case this mechanism is claiming. */
#define PGT_BINARY_ELF_HANDLES 64
static void *pgb_elf_handles[PGT_BINARY_ELF_HANDLES];
static int   pgb_elf_nhandles;

/* ⛔ IS the loader in this link at all. `--wrap-dlopen` links this file without
 * elfload.o -- a program loading its OWN plugins needs no loader, and
 * linking one would drag in the provider table with it. The loader's entry
 * points are therefore WEAK (binary/elfload.h), so their addresses are 0 here and
 * the address must be tested BEFORE the call: calling through a weak-undefined
 * function is a jump to NULL, not a "no". */
static int pgb_elf_linked(void)
{
    return &pgb_elf_available != 0 && pgb_elf_available();
}

static void pgb_elf_remember(void *h)
{
    int i;
    for (i = 0; i < pgb_elf_nhandles; i++)
        if (pgb_elf_handles[i] == h)
            return;
    if (pgb_elf_nhandles < PGT_BINARY_ELF_HANDLES)
        pgb_elf_handles[pgb_elf_nhandles++] = h;
}

static int pgb_is_elf_handle(void *h)
{
    int i;
    for (i = 0; i < pgb_elf_nhandles; i++)
        if (pgb_elf_handles[i] == h)
            return 1;
    return 0;
}

/* Hand a path to the compiled-in loader and adopt its error into dlerror(). */
static void *pgb_elf_open(const char *path, int flags)
{
    void *h = pgb_elf_dlopen(path, flags);
    if (h == NULL) {
        const char *e = pgb_elf_dlerror();
        pgb_dl_err = e ? e : "pgb: the compiled-in loader refused the object";
        return NULL;
    }
    pgb_elf_remember(h);
    return h;
}

void *__wrap_dlopen(const char *path, int flags)
{
    const struct pgb_dl_lib *l;

    (void)flags;   /* RTLD_LAZY/NOW/GLOBAL/LOCAL describe work that is already
                    * done: the code is in the executable and was bound by the
                    * static link. There is nothing left to defer or scope. */

    pgb_dl_err = NULL;

    /* ⭐ the order IS compiled-in table first, loader second, and it matters.
     * A program's own plugin is already in the link, so answering it from the
     * table maps nothing at all. Reaching for the loader first would map a
     * copy of code the executable already contains and give the process two of
     * every symbol in it. The loader is for objects that are NOT in the link,
     * which is exactly what a HOST plugin is. */
    if (&pgb_dlopen_libs[0] == NULL) {
        if (pgb_elf_linked() && path != NULL)
            return pgb_elf_open(path, flags);
        pgb_dl_err = "pgb: no plugin table was compiled in "
                     "(build with --wrap-dlopen or --host-dlopen)";
        return NULL;
    }

    if (path == NULL) {
        /* Only answerable if the build named an entry "@SELF". */
        l = pgb_find("@SELF");
        if (l == NULL) {
            pgb_dl_err = "pgb: dlopen(NULL) needs a \"@SELF\" entry in the "
                         "compiled-in table; this build has none";
            return NULL;
        }
    } else {
        l = pgb_find(path);
        if (l == NULL) {
            /* Not one of this build's own plugins. With --host-dlopen the
             * compiled-in loader gets it; without, the message is unchanged. */
            if (pgb_elf_linked())
                return pgb_elf_open(path, flags);
            pgb_dl_err = "pgb: no such plugin in the compiled-in table";
            return NULL;
        }
    }

    /* ⛔ a handle must not collide with a sentinel. pipewire's dlfcn.zig
     * asserts this at comptime; C cannot, so it is checked here. In practice
     * a table entry never lands at address 0 or -1, but "in practice" is how
     * a silent wrong answer gets shipped: dlsym would take the collision for
     * RTLD_DEFAULT and search the wrong thing. */
    if ((const void *)l == PGT_BINARY_RTLD_DEFAULT || (const void *)l == PGT_BINARY_RTLD_NEXT) {
        pgb_dl_err = "pgb: plugin table entry collides with an RTLD sentinel";
        return NULL;
    }

    return (void *)(size_t)l;
}

void *__wrap_dlsym(void *handle, const char *name)
{
    const struct pgb_dl_lib *l;
    const struct pgb_dl_sym *s;

    pgb_dl_err = NULL;

    if (handle == PGT_BINARY_RTLD_DEFAULT || handle == PGT_BINARY_RTLD_NEXT) {
        /* ⚠ Reported, never faked. Both are a search ORDER over loaded
         * objects and this binary has no such order -- everything is already
         * one image. Answering from the table would be a guess dressed as a
         * result. */
        pgb_dl_err = "pgb: RTLD_DEFAULT/RTLD_NEXT are not available in a "
                     "statically linked image";
        return NULL;
    }

    if (handle == NULL) {
        pgb_dl_err = "pgb: dlsym on a NULL handle";
        return NULL;
    }

    if (pgb_is_elf_handle(handle)) {
        void *a = pgb_elf_dlsym(handle, name);
        if (a == NULL) {
            const char *e = pgb_elf_dlerror();
            pgb_dl_err = e ? e : "pgb: symbol not found in the loaded object";
        }
        return a;
    }

    l = (const struct pgb_dl_lib *)handle;
    if (l->syms == NULL) {
        pgb_dl_err = "pgb: plugin has no symbol table";
        return NULL;
    }

    for (s = l->syms; s->name; s++)
        if (strcmp(s->name, name) == 0)
            return s->addr;

    pgb_dl_err = "pgb: symbol not found in the compiled-in table";
    return NULL;
}

int __wrap_dlclose(void *handle)
{
    pgb_dl_err = NULL;
    if (pgb_is_elf_handle(handle))
        return pgb_elf_dlclose(handle);
    /* Nothing was mapped, so nothing is unmapped. Returning 0 is not a lie:
     * dlclose's contract is that the caller may no longer use the handle,
     * and it may not. */
    (void)handle;
    return 0;
}
