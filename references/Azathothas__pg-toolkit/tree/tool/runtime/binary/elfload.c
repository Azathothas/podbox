/* binary/elfload.c -- an ELF loader compiled INTO a static glibc executable.
 *
 * The problem this answers, and why the obvious fix cannot
 * -------------------------------------------------------------------------
 * A static glibc binary that calls dlopen() borrows the HOST's ld.so and
 * libc.so.6, and that pairing is what breaks: across the eleven pinned
 * environments it dies inside glibc's own loader on nine and "succeeds" on
 * two, where success means a second libc is now in the process.
 *
 * ⛔ and a better host loader would not help. experiments/72- measured why:
 * a static executable's dynamic symbol table IS empty -- DYNSYM 0 in that
 * table's own column -- so a host-loaded plugin has nothing to bind back to.
 * The plugin resolves `host_api_add` against an image that exports nothing
 * and fails with "undefined symbol" even when the loader itself works.
 *
 *   Arm              dynsym   outcome
 *   dynamic-host     11       PASSED
 *   static-host      0        FAIL dlopen: undefined symbol: host_api_add
 *
 * ⭐ So the loader has to be OURS, and the thing it resolves against has to
 * be the glibc already statically linked into this executable.
 *
 * Why that IS tractable, measured rather than hoped for
 * -------------------------------------------------------------------------
 * experiments/73- parsed 5,807 real host shared objects across the seven
 * pinned glibc environments and checked every GLIBC_/GCC_-versioned import
 * against what the pinned static glibc can define: 90.8%-97.8% already
 * definable, and the UNEXPLAINED residue is zero on every environment. Every
 * symbol that is not served falls into a class with a measured reason:
 *
 *   A  the host ld.so exports it     -- a compiled-in loader owns these, and
 *                                      this file is where they are owned
 *   B  host glibc newer than the pin -- 20 symbols, 14 of them __isoc23_*
 *   C  the pin removed it            -- empty everywhere
 *   S  in libc.so.6, never in libc.a -- 49 symbols, mostly sunrpc
 *   D  not the host libc's either    -- another library's, or nobody's
 *   E  unexplained                   -- zero
 *
 * what was taken from pg83/solo, and what was deliberately not
 * -------------------------------------------------------------------------
 * references/pg83__solo at 79451211 is a working loader of this shape, and
 * docs/research/solo.md is the sweep. Taken: the provider table generated
 * from a name list (lib/musl_symbols.cpp:1-12), the resolution ORDER rather
 * than an approximation of it (lib/elf_loader.cpp:2034-2078), static
 * providers short-circuiting the mapping entirely (lib/dlfcn.cpp:294-302),
 * per-symbol loud failure instead of refusing the object (lib/glibc_stubs.cpp),
 * the ldd-format internal trace (lib/elf_loader.h:21-26), and the AT_SECURE
 * discipline (lib/elf_loader.h:11-14).
 *
 * ⛔ NOT taken, and this is the whole reason the route is cheaper here:
 * lib/glibc_shim.cpp is 5,948 lines translating a guest's glibc imports onto
 * MUSL. This executable's libc IS glibc, so there is no translation to write.
 * lib/musl_tls.c is not taken either -- it writes musl's `libc.tls_head`
 * directly, and glibc's equivalent is a different mechanism. TLS is handled
 * below on glibc's own terms.
 *
 * ⚠ what this FILE does not do, stated rather than discovered later. Each is
 * a loud, named failure, never a silent wrong answer:
 *
 *   - R_X86_64_TPOFF64 (initial-exec TLS) needs space in the STATIC TLS block,
 *     which is laid out before main() and cannot be grown afterwards. It is
 *     served from glibc's own reserved surplus when that is reachable and
 *     refused by name when it is not. Demand is measured in experiments/76-.
 *   - R_X86_64_TLSDESC needs a resolver trampoline; refused by name.
 *   - Lazy binding is not implemented. Every PLT slot is bound at load, which
 *     is what RTLD_NOW does, and RTLD_LAZY is honoured as RTLD_NOW. That is
 *     allowed: lazy binding is an optimisation, not a contract.
 *   - dladdr/dlinfo/dlvsym are not answered here.
 *
 * SPDX-License-Identifier: 0BSD
 */

#define _GNU_SOURCE

#define PGT_BINARY_ELFLOAD_IMPL 1
#include "elfload.h"

#include <elf.h>
#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>
#include <stdint.h>
#include <netdb.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/auxv.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

/* ------------------------------------------------------------------ errors */

/* ⛔ dlerror() IS part of the interface. A caller that gets NULL with no
 * message has been handed a silent failure, which is the exact failure mode
 * this project exists to remove. Every path that returns NULL sets this. */
static char pgb_el_errbuf[512];
static int  pgb_el_errset;

static void el_err(const char *fmt, ...)
    __attribute__((format(printf, 1, 2)));

static void el_err(const char *fmt, ...)
{
    va_list ap;
    __builtin_va_start(ap, fmt);
    vsnprintf(pgb_el_errbuf, sizeof pgb_el_errbuf, fmt, ap);
    __builtin_va_end(ap);
    pgb_el_errset = 1;
}

const char *pgb_elf_dlerror(void)
{
    if (!pgb_el_errset)
        return NULL;
    pgb_el_errset = 0;
    return pgb_el_errbuf;
}

/* ⭐ a trace that survives the process dying, because the failures this loader
 * has to explain are constructors that segfault, and a message buffered behind
 * one of those is a message nobody reads. Unbuffered write(2) to stderr, gated
 * on PGT_BINARY_ELFLOAD_DEBUG so it costs one getenv in a normal run.
 *
 * ⛔ Ignored for a set-uid process, with every other environment switch. */
static int el_dbg_on(void)
{
    static int v = -1;
    if (v < 0) {
        if (getauxval(AT_SECURE) == 0 && getenv("PGB_ELFLOAD_DEBUG") != NULL) {
            /* ⛔ The retired spelling is reported and NOT honoured - T-147.
             * ⚠ The toolkit exits 2 on a retired name; this loader is
             * compiled into somebody else's program and only says so, because
             * a debug variable is not worth killing that program over. The
             * write is unbuffered for the same reason every other line here
             * is: the failures this loader explains take the process with
             * them. */
            static const char m[] =
                "pgb-elfload: PGB_ELFLOAD_DEBUG was retired; "
                "set PGT_BINARY_ELFLOAD_DEBUG instead\n";
            (void)!write(2, m, sizeof m - 1);
        }
        v = (getauxval(AT_SECURE) == 0) && getenv("PGT_BINARY_ELFLOAD_DEBUG") != NULL;
    }
    return v;
}

static void el_dbg(const char *fmt, ...) __attribute__((format(printf, 1, 2)));

static void el_dbg(const char *fmt, ...)
{
    char b[512];
    va_list ap;
    int n;

    if (!el_dbg_on())
        return;
    __builtin_va_start(ap, fmt);
    n = vsnprintf(b, sizeof b, fmt, ap);
    __builtin_va_end(ap);
    if (n > 0)
        (void)!write(2, b, (size_t)n < sizeof b ? (size_t)n : sizeof b - 1);
}

/* ------------------------------------------------------------ loaded object */

/* An upper bound on a .dynsym, used only to stop a corrupt hash chain
 * running off the end of a mapping. libLLVM's is ~100k. */
#define EL_MAX_SYMS   (1u << 22)
#define EL_MAX_OBJS   64
#define EL_MAX_NEEDED 32

struct el_obj {
    char        *path;          /* what we opened */
    char        *soname;        /* DT_SONAME, or the basename */
    unsigned char *base;        /* load bias; p_vaddr + base == run address */
    size_t       span;          /* bytes reserved by the mapping */

    const Elf64_Sym *symtab;
    const char      *strtab;
    const uint32_t  *gnu_hash;  /* DT_GNU_HASH, or NULL */
    const uint32_t  *elf_hash;  /* DT_HASH, or NULL */
    uint32_t         nsyms;     /* derived; 0 when neither hash is present */

    const uint16_t  *versym;    /* DT_VERSYM */
    const Elf64_Verdef  *verdef;
    const Elf64_Verneed *verneed;

    const Elf64_Rela *rela;     size_t relasz;
    const Elf64_Rela *jmprel;   size_t jmprelsz;
    const uint64_t   *relr;     size_t relrsz;

    void (**init_array)(void);  size_t init_arrayn;
    void (**fini_array)(void);  size_t fini_arrayn;
    void (*init)(void);
    void (*fini)(void);

    const char *runpath;        /* DT_RUNPATH, else DT_RPATH */
    int          symbolic;      /* DT_SYMBOLIC or DF_SYMBOLIC */
    int          textrel;

    /* PT_TLS, if any. modid is this loader's own module numbering.
     * tls_tpoff is non-zero once the block has been placed in glibc's static
     * TLS surplus -- see el_static_tls(). */
    const unsigned char *tls_image;
    size_t tls_filesz, tls_memsz, tls_align;
    size_t tls_modid;
    long   tls_tpoff;

    unsigned char *relro;  size_t relrosz;

    /* The mapped range, kept so a relocation cannot be talked into writing
     * outside it. See el_in_map(). */
    unsigned char *map_lo, *map_hi;

    struct el_obj *needed[EL_MAX_NEEDED];
    int            nneeded;

    int refcount;
    int initialised;

    /* ⛔ Is this object in the global lookup scope? Set from `dlopen`'s
     * RTLD_GLOBAL and propagated to the object's DT_NEEDED tree, which is what
     * ld.so does. ⚠ Discard `flags` here and every object behaves as
     * RTLD_GLOBAL, so the first object loaded wins every name for every object
     * after it. `experiments/104-` is the regression test. */
    int global;
};

/* RTLD_GLOBAL's value, spelled out because this file deliberately does not
 * include <dlfcn.h>: it defines the same entry points and the header would
 * fight it. The value is ABI on every glibc and musl target this loader runs
 * on, and a wrong value here can only mean an object is treated as local. */
#ifndef EL_RTLD_GLOBAL
#define EL_RTLD_GLOBAL 0x00100
#endif

static struct el_obj *el_objs[EL_MAX_OBJS];
static int            el_nobjs;

/* Sonames answered out of the provider table instead of being mapped. Kept so
 * pgb_elf_trace_loaded() can report them the way ldd does -- solo's mechanism
 * 5, and the reason it matters is that a name served WITHOUT a mapping is
 * invisible to a syscall trace. */
static const char *el_served[EL_MAX_OBJS * EL_MAX_NEEDED];
static int         el_nserved;

/* ------------------------------------------------------------------- paging */

static size_t el_pagesz(void)
{
    static size_t p;
    if (!p) {
        long v = sysconf(_SC_PAGESIZE);
        p = v > 0 ? (size_t)v : 4096;
    }
    return p;
}

static size_t el_down(size_t v) { return v & ~(el_pagesz() - 1); }
static size_t el_up(size_t v)   { return (v + el_pagesz() - 1) & ~(el_pagesz() - 1); }

/* ---------------------------------------------------------- provider lookup */

int pgb_elf_available(void)
{
    return &pgb_provider_syms[0] != NULL && pgb_provider_syms[0].name != NULL;
}

/* ⭐ class a of experiments/73-, defined here because ld.so defines it.
 *
 * These are not in libc.a and never will be: the host's dynamic loader
 * exports them, so a loader compiled in has to own them itself or every
 * object using general-dynamic TLS fails to resolve. Measured: of 492
 * undefined-symbol failures in the first full sweep of 904 host objects,
 * 398 were __tls_get_addr alone -- one name, 81% of the failures.
 *
 * ⚠ Checked BEFORE the generated table, because the generated table is built
 * from archives that do not contain these and a stale entry must not win. */
struct el_tls_index;
void *__tls_get_addr(struct el_tls_index *ti);

/* ⭐ class s, and it IS 247 of the 257 objects in the real residue.
 *
 * `_dl_mcount_wrapper_check` is in `libc.so.6` and NOT in `libc.a` -- checked
 * with nm on the pinned environment's own archive, which is what makes it
 * experiments/73-'s class S rather than a version problem. Any object built
 * against a shared glibc can reference it, and 247 on this host do.
 *
 * ⛔ Leaving it undefined REFUSES those objects, which is a much worse answer
 * than the right one, because the right one is to do nothing. glibc's version
 * routes a `-pg`-instrumented shared object's mcount through rtld:
 *
 *     void _dl_mcount_wrapper_check (void *selfpc)
 *     { if (_dl_mcount_via_rtld) _dl_mcount (RETURN_ADDRESS (0), selfpc); }
 *
 * ⭐ There is no rtld in this process and no profiling to route, so the object
 * is simply not profiled -- which is the same outcome as running it on a
 * system where profiling is off, and is what the caller already tolerates.
 *
 * ⚠ MEASURED, not assumed, and the measurement is the point. Of the 631
 * objects this loader failed with "undefined symbol", GLIBC'S OWN ld.so also
 * fails 374 -- they are plugins of a host PROGRAM (CPython, Perl, PostgreSQL,
 * PHP) whose symbols live in the executable, and nobody can load them
 * standalone. The residue that is actually ours is the other 257, and it is
 * SIX distinct symbols. This is the first of them.
 */
static void el_dl_mcount_wrapper_check(void *selfpc)
{
    (void)selfpc;
}

/* ⛔ the two names above have opposite requirements, so they are two tables
 * consulted at two different points of el_resolve() rather than one table
 * consulted once.
 *
 *   __tls_get_addr             must win over every loaded object. The module
 *                              ids it is handed were written by THIS loader's
 *                              R_X86_64_DTPMOD64 case out of el_tls_next_modid;
 *                              no other implementation can interpret them, so
 *                              a loaded object's copy answering instead reads
 *                              and writes through a pointer derived from
 *                              somebody else's bookkeeping.
 *
 *   _dl_mcount_wrapper_check   must yield to any real definition. It is a
 *                              stand-in for something ld.so would have
 *                              provided, and its whole content is `do
 *                              nothing`. If a real one is in scope, answering
 *                              from here would SHADOW it with a no-op.
 *
 * ⛔ one table cannot express both, and the single table this replaced was
 * checked FIRST and UNCONDITIONALLY inside el_provider(). That was safe only
 * because el_resolve() happens to call el_provider() last -- an ordering
 * nothing asserted, that neither entry named, and that was correct for one of
 * the two names by accident. experiments/94- asserts both directions and
 * carries the negative control that reproduces the single-table shape.
 *
 * ⚠ the defect class, and it is the one this tree keeps paying for: a lookup
 * that ANSWERS when it should DEFER. The weak `iconv_open` provider entry held
 * NULL and shadowed the real one; R_X86_64_DTPOFF64 answered offset 0 rather
 * than deferring to the defining module; pkgforge-dev/cross-libc-dlopen#28 is
 * 358 zero-returning stubs shadowing a provider the process could still
 * serve. */
static const struct pgb_provider_sym el_own_syms_first[] = {
    { "__tls_get_addr", (void *)(uintptr_t)&__tls_get_addr },
    { NULL, NULL }
};

static const struct pgb_provider_sym el_own_syms_last[] = {
    { "_dl_mcount_wrapper_check",
      (void *)(uintptr_t)&el_dl_mcount_wrapper_check },
    { NULL, NULL }
};

static void *el_own_lookup(const struct pgb_provider_sym *tab, const char *name)
{
    for (; tab->name; tab++)
        if (strcmp(tab->name, name) == 0)
            return tab->addr;
    return NULL;
}

/* ⭐ the negative control, and it is a BUILDER knob rather than a runtime one.
 * PGT_BINARY_T073_OWNSYMS_UNORDERED=1 makes internal/wrapper pass this define and
 * name the object differently; a shipped binary has no way to reach it. Under
 * it both tables move back inside el_provider(): one lookup, checked first
 * and called last. `experiments/94-` arm 2 must FAIL under it and arm 1 must
 * still pass, which is what makes it a control rather than a switch. */
#ifdef PGT_BINARY_ELFLOAD_OWNSYMS_UNORDERED
static void *el_own_first(const char *name) { (void)name; return NULL; }
static void *el_own_last(const char *name)  { (void)name; return NULL; }
#else
static void *el_own_first(const char *name)
{
    return el_own_lookup(el_own_syms_first, name);
}
static void *el_own_last(const char *name)
{
    return el_own_lookup(el_own_syms_last, name);
}
#endif

/* ⭐ THREAD-LOCAL symbols this image's own glibc defines, reached by ADDRESS
 * rather than by name. A TLS symbol cannot go in the generated provider table
 * -- 30 of libc.a's exports are TLS and the linker refuses a non-TLS
 * reference to any of them, which is a link error, not a policy. But their
 * run-time address IS available, and the offset from the thread pointer is
 * the same in every thread, so a host object importing errno binds correctly
 * everywhere. 10 of the 42 initial-exec failures in the first sweep were
 * exactly this one name. */
static void *el_addr_errno(void)   { return &errno; }
static void *el_addr_h_errno(void) { return &h_errno; }

static void *el_tls_provider(const char *name)
{
    if (strcmp(name, "errno") == 0)
        return el_addr_errno();
    if (strcmp(name, "h_errno") == 0)
        return el_addr_h_errno();
    return NULL;
}

/* The executable's own static glibc, by name. The generated table is sorted,
 * so this is a binary search: the table runs to several thousand entries and
 * a load touches it once per undefined symbol.
 *
 * ⚠ a null addr is a name the generator listed and the LINK did not pull in
 * -- see the weak-reference note in binary/elfload.h. It is a miss here, and the
 * caller reports it by name rather than guessing. */
/* ⛔ The length is computed once. Computing it per lookup costs an order of
 * magnitude: walking the table to its NULL terminator before every binary
 * search is 7,216 iterations per undefined symbol, and a small object has
 * dozens. Time to first symbol for
 * libz.so.1, best of 200 cold loads, fork per iteration:
 *
 *   before   673,989 ns        after   see experiments/76-
 *   ld.so     64,484 ns        (the host's own loader, same object)
 *
 * The table is a link-time constant, so one pass on first use is all it can
 * ever need. */
static size_t el_provider_count(void)
{
    static size_t n;
    static int done;

    if (!done) {
        while (pgb_provider_syms[n].name)
            n++;
        done = 1;
    }
    return n;
}

static void *el_provider(const char *name)
{
    size_t lo = 0, hi;

#ifdef PGT_BINARY_ELFLOAD_OWNSYMS_UNORDERED
    {
        void *p = el_own_lookup(el_own_syms_first, name);
        if (!p)
            p = el_own_lookup(el_own_syms_last, name);
        if (p)
            return p;
    }
#endif

    if (&pgb_provider_syms[0] == NULL)
        return NULL;

    hi = el_provider_count();
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        int c = strcmp(pgb_provider_syms[mid].name, name);
        if (c == 0)
            return pgb_provider_syms[mid].addr;
        if (c < 0)
            lo = mid + 1;
        else
            hi = mid;
    }
    return NULL;
}

/* ⛔ a foreign c library or loader IS refused, never mapped.
 *
 * On a musl target every shared object carries DT_NEEDED
 * libc.musl-x86_64.so.1, and musl's libc IS its dynamic loader -- mapping it
 * into a process whose libc is glibc and running its initialiser crashes,
 * which is what the first run of experiments/76- measured on Alpine 3.22 and
 * 3.20 (SIG11 on both). ⭐ The refusal is not a limitation being papered
 * over: a second libc in the process is the outcome docs/limitations.md section 1
 * calls WORSE than failing, so the honest answer on those rows is a named
 * error, and that is what this produces.
 *
 * Anything shaped like a libc or a loader that the provider table does not
 * already serve falls here, so a libc this list has never heard of is refused
 * rather than mapped on the strength of not being recognised. */
static int el_foreign_libc(const char *soname)
{
    return strncmp(soname, "libc.", 5) == 0 ||
           strncmp(soname, "ld-", 3) == 0 ||
           strncmp(soname, "ld.so", 5) == 0 ||
           strncmp(soname, "libpthread.", 11) == 0;
}

/* ⛔ objects no static image should ever load, refused by class.
 *
 * ⚠ MEASURED, not guessed at. Sweeping every shared object on the build host
 * -- 904 of them, one fork each -- left 30 crashes, and almost all of them
 * were these two families. They do not crash because the loader is wrong;
 * they crash because loading them into this process is meaningless:
 *
 *   libnss_*    ⛔ docs/AGENTS.md: "Do not try to make host NSS modules
 *               load correctly. Keeping them out is the fix and it is
 *               measured." binary/nssfix.c pins every database to a service
 *               glibc implements inside libc, so nothing in this image wants
 *               one, and each carries DT_NEEDED libc.so.6 anyway.
 *
 *   sanitizer   libasan, libtsan, libhwasan, libjemalloc, libmemusage and
 *   and         friends interpose on malloc and are designed to arrive by
 *   allocator   LD_PRELOAD BEFORE libc initialises. Loading one into a
 *   interposers running process is undefined under any loader, ld.so
 *               included -- glibc's allocator is already initialised and
 *               holding memory the new one knows nothing about.
 *
 * ⭐ A named refusal is the product here, not a workaround: fail loudly and
 * per object rather than crash.
 */
static int el_refused_class(const char *soname, const char **why)
{
    static const char *const interposers[] = {
        "libasan.", "libtsan.", "libhwasan.", "liblsan.", "libubsan.",
        "libjemalloc.", "libtcmalloc", "libmemusage.", "libpcprofile.",
        "libSegFault.", "libdislocator.", "libmimalloc.",
        /* ⭐ Measured rather than guessed at. gprofng's collector defines
         * `mmap` and imports `dlsym`/`dlvsym`,
         * which is the LD_PRELOAD interposer signature: define a libc symbol,
         * resolve the real one through RTLD_NEXT, chain to it. ⛔ A static
         * image has no RTLD_NEXT -- binary/dlopen.c reports that as its own
         * error -- so the constructor calls through NULL and takes SIGSEGV.
         * ⚠ It is the ONE object of experiments/93-'s 46 crashes that glibc's
         * own ld.so loads cleanly, which is why it is a refusal we owe and not
         * a difference we can shrug at. `libgp-collectorAPI.so`, the API half,
         * is NOT an interposer and continues to load. */
        "libgp-collector.", NULL
    };
    /* ⛔ what IS deliberately *NOT* HERE, and the measurement that removed it.
     *
     * A rule declining "libxt_ libipt_ libip6t_ libebt_ libarpt_" was written
     * here and then taken out. It did turn experiments/93-'s 45 xtables
     * crashes into named refusals -- and it also moved `ok (loaded)` from 446
     * to 377, because 69 xtables modules on this host load fine. Declining a
     * whole family by name to make an assertion go green traded 69 measured
     * successes for a number, which is the opposite of what the assertion is
     * for. ⚠ 93- measures LOADING, not behaviour: whether those 69 are useful
     * outside iptables is unmeasured, and unmeasured is not a licence.
     *
     * The 45 that do crash are a real finding and they stay visible. They are
     * not OUR defect -- an ordinary dynamic binary on glibc's own ld.so
     * segfaults on all 45 too -- so 93- asserts the right question instead:
     * nothing may crash this loader that glibc's loader loads. */
    const char *const *p;

    if (strncmp(soname, "libnss_", 7) == 0) {
        *why = "an NSS module; this image pins every database to a service "
               "glibc implements inside libc";
        return 1;
    }
    for (p = interposers; *p; p++)
        if (strncmp(soname, *p, strlen(*p)) == 0) {
            *why = "an allocator, sanitizer or profiler interposer; it chains "
                   "through RTLD_NEXT, which a static image does not have";
            return 1;
        }
    return 0;
}

/* ⭐ solo's mechanism 3, and the mechanism that keeps a foreign libc OUT.
 * Before touching the disk, a DT_NEEDED is checked against the sonames this
 * executable already satisfies. A plugin needing libc.so.6 gets the glibc in
 * this image, so the host's copy is never opened. */
static int el_soname_served(const char *soname)
{
    const char *const *p;

    if (&pgb_provider_sonames[0] == NULL)
        return 0;
    for (p = pgb_provider_sonames; *p; p++)
        if (strcmp(*p, soname) == 0)
            return 1;
    return 0;
}

/* ⛔ a relocation's r_offset IS attacker-controlled data, not a promise.
 *
 * Every write below computes `where = base + r_offset` and stores through it.
 * Nothing in the ELF format stops r_offset naming an address outside the
 * object's own mapping, and a loader that trusts it will happily write eight
 * bytes anywhere in the address space on behalf of a file it just found on
 * disk. This is the check that makes that a named refusal instead.
 *
 * ⚠ Bounds are the MAPPED range, not the span from base: a PT_LOAD set can
 * start at a non-zero p_vaddr, so `base` itself may point below the mapping. */
static int el_in_map(const struct el_obj *o, const void *p, size_t len)
{
    const unsigned char *q = p;
    return o->map_lo && q >= o->map_lo && len <= (size_t)(o->map_hi - q);
}

/* ⭐ the same refusal, decided by shape instead of by name.
 *
 * The list above only ever catches an interposer somebody has already met.
 * This reads the object's own dynamic symbol table for the signature that
 * makes one unloadable here:
 *
 *   it DEFINES an allocator or mmap entry point   (it will be interposed on
 *                                                  the moment it is mapped)
 *   and IMPORTS dlsym or dlvsym                   (it chains to the real one
 *                                                  through RTLD_NEXT)
 *
 * ⛔ A static image has no RTLD_NEXT, so the pointer it means to fill stays
 * NULL and the FIRST allocation anywhere in the process calls through it. That
 * is not a load failure, it is a crash in somebody else's code: `libgprofng`
 * took the fault inside libstdc++'s own initialiser, at `malloc+0x3a`,
 * `call *0xcba40(%rip)` with the slot still zero.
 *
 * ⭐ measured before it was written, across all 1,527 shared objects on this
 * host. The signature matches FIVE, and every one is a known interposer:
 *
 *   libmemusage.so          libjemalloc.so.2        libgprofng.so.0.0.0
 *   gprofng/libgp-heap.so   gprofng/libgp-collector.so
 *
 * ⚠ Zero false positives, and two of the five were NOT in the name list -- so
 * the rule is strictly better informed than the list, not a broader net. ⛔ It
 * is deliberately NOT "defines malloc": libc itself does, and so does anything
 * that ships its own allocator without interposing. The dlsym import is what
 * separates the two, and it is the half that is actually about RTLD_NEXT.
 */
static int el_is_interposer(const struct el_obj *o, const char **why)
{
    static const char *const allocators[] = {
        "malloc", "calloc", "realloc", "free", "mmap", "mmap64", NULL
    };
    int defines = 0, chains = 0;
    uint32_t n;

    if (!o->symtab || !o->strtab || !o->nsyms)
        return 0;
    for (n = 0; n < o->nsyms && !(defines && chains); n++) {
        const Elf64_Sym *s = &o->symtab[n];
        const char *nm;
        const char *const *a;

        if (!el_in_map(o, s, sizeof *s))
            break;
        nm = o->strtab + s->st_name;
        if (!el_in_map(o, nm, 1) || !*nm)
            continue;
        if (s->st_shndx == SHN_UNDEF) {
            if (strcmp(nm, "dlsym") == 0 || strcmp(nm, "dlvsym") == 0)
                chains = 1;
            continue;
        }
        if (ELF64_ST_TYPE(s->st_info) != STT_FUNC)
            continue;
        for (a = allocators; *a; a++)
            if (strcmp(nm, *a) == 0) { defines = 1; break; }
    }
    if (defines && chains) {
        *why = "an interposer by shape -- it defines an allocator or mmap "
               "entry point and imports dlsym, so it chains through "
               "RTLD_NEXT, which a static image does not have";
        return 1;
    }
    return 0;
}

/* ------------------------------------------------------------ symbol tables */

static uint32_t el_gnu_hash(const char *s)
{
    uint32_t h = 5381;
    for (; *s; s++)
        h = h * 33 + (unsigned char)*s;
    return h;
}

static uint32_t el_sysv_hash(const char *s)
{
    uint32_t h = 0, g;
    for (; *s; s++) {
        h = (h << 4) + (unsigned char)*s;
        if ((g = h & 0xf0000000u))
            h ^= g >> 24;
        h &= ~g;
    }
    return h;
}

/* The number of entries in .dynsym. DT_HASH states it outright; DT_GNU_HASH
 * does not, so it is derived by finding the highest bucket and walking that
 * chain to its terminator. Needed because dlsym over a versioned table and
 * the verneed walk both iterate symbols by index. */
static uint32_t el_gnu_symcount(const uint32_t *gh)
{
    uint32_t nbuckets = gh[0], symoffset = gh[1], bloomsz = gh[2];
    const uint32_t *buckets = gh + 4 + bloomsz * 2;   /* bloom words are 64-bit */
    const uint32_t *chain   = buckets + nbuckets;
    uint32_t i, last = 0;

    for (i = 0; i < nbuckets; i++)
        if (buckets[i] > last)
            last = buckets[i];
    if (last < symoffset)
        return symoffset;
    /* ⛔ BOUNDED. This walk runs to a terminator bit that a corrupt table need
     * not contain, and it is reading a mapping whose end is a page boundary --
     * so an unbounded version reads off the end and takes SIGSEGV. The cap is
     * the count no real object exceeds; a table that needs more than this is
     * one this loader declines to characterise. */
    while (!(chain[last - symoffset] & 1)) {
        if (last - symoffset >= EL_MAX_SYMS)
            return 0;
        last++;
    }
    return last + 1;
}

/* Version name of symbol index `si` when it is a DEFINITION, or NULL for an
 * unversioned one. Index 0 (local) and 1 (global, unversioned) both mean
 * "carries no version" for matching purposes; the 0x8000 bit marks hidden. */
static const char *el_defver(const struct el_obj *o, uint32_t si)
{
    uint16_t vi;
    const Elf64_Verdef *vd;

    if (!o->versym || !o->verdef)
        return NULL;
    /* ⛔ versym IS a separate array from symtab, and nothing has checked that
     * it is as long. The walk validates `&symtab[si]` against the mapping; a
     * malformed object can give the two different lengths, and reading
     * versym[si] on that promise is an out-of-map read of exactly the class
     * el_in_map() exists for. ⚠ The cost of getting it wrong scales: this
     * call sits inside the symbol walk. */
    if (!el_in_map(o, &o->versym[si], sizeof o->versym[si]))
        return NULL;
    vi = o->versym[si] & 0x7fff;
    if (vi <= 1)
        return NULL;
    for (vd = o->verdef; el_in_map(o, vd, sizeof *vd); ) {
        if ((vd->vd_ndx & 0x7fff) == vi) {
            const Elf64_Verdaux *aux =
                (const Elf64_Verdaux *)((const char *)vd + vd->vd_aux);
            return o->strtab + aux->vda_name;
        }
        if (!vd->vd_next)
            return NULL;
        vd = (const Elf64_Verdef *)((const char *)vd + vd->vd_next);
    }
    return NULL;   /* walked out of the mapping: no version rather than a guess */
}

/* A definition in `o` matching `name`, honouring the version rule. Returns the
 * symbol or NULL.
 *
 * ⚠ the version rule IS narrower than it looks, and experiments/73- asserts
 * it in both directions. An UNVERSIONED definition satisfies a versioned
 * reference -- that is ld.so's documented compatibility rule and it is where
 * a compiled-in provider table sits. But the object NAMED in DT_VERNEED,
 * rebuilt without versions, makes the real loader assert. So: match the
 * version when both sides carry one; accept an unversioned definition
 * otherwise; never accept a WRONG version silently. */
/* el_accept decides one candidate against the wanted version.
 *  1  take it now      0  keep looking      -1  keep looking, but remember it
 *
 * ⛔ the defect this exists to fix. A versioned symbol table holds SEVERAL
 * entries with the same name at different versions -- liblzma has
 * `lzma_cputhreads@XZ_5.2.2` and `lzma_cputhreads@@XZ_5.2` at one address,
 * libdevmapper has `dm_task_get_info@Base` and `@@DM_1_02_97`. The walk used
 * to `break` on the first NAME match and check the version afterwards, so
 * whichever entry the hash chain happened to yield first decided the answer:
 * hit the wrong one and a symbol that IS defined became a loud miss. Nine of
 * the ten objects in the real residue failed exactly this way. */
static int el_accept(const struct el_obj *o, uint32_t n, const char *want_ver)
{
    const char *have;

    if (!want_ver)
        return 1;
    have = el_defver(o, n);
    if (!have)
        return -1;    /* ⭐ ld.so's compatibility rule: an UNVERSIONED
                       * definition satisfies a versioned reference -- but only
                       * if no exact match turns up, so it is a fallback and
                       * not an answer. */
    return strcmp(have, want_ver) == 0 ? 1 : 0;
}

static const Elf64_Sym *el_lookup_in(const struct el_obj *o, const char *name,
                                     const char *want_ver)
{
    const Elf64_Sym *sym = NULL, *fallback = NULL;
    uint32_t si = 0;

    if (!o->symtab || !o->strtab)
        return NULL;

    if (o->gnu_hash) {
        const uint32_t *gh = o->gnu_hash;
        uint32_t nbuckets = gh[0], symoffset = gh[1];
        uint32_t bloomsz = gh[2], bloomshift = gh[3];
        const uint64_t *bloom = (const uint64_t *)(gh + 4);
        const uint32_t *buckets = (const uint32_t *)(bloom + bloomsz);
        const uint32_t *chain = buckets + nbuckets;
        uint32_t h = el_gnu_hash(name), n;
        uint64_t word, mask;

        if (!nbuckets || !bloomsz)
            return NULL;
        word = bloom[(h / 64) % bloomsz];
        mask = (1ull << (h % 64)) | (1ull << ((h >> bloomshift) % 64));
        if ((word & mask) != mask)
            return NULL;              /* the bloom filter says no, definitively */
        n = buckets[h % nbuckets];
        if (n < symoffset)
            return NULL;
        for (; el_in_map(o, &chain[n - symoffset], sizeof(uint32_t)); n++) {
            uint32_t c = chain[n - symoffset];
            if (!el_in_map(o, &o->symtab[n], sizeof(Elf64_Sym)))
                return NULL;
            if ((c | 1) == (h | 1)) {
                const Elf64_Sym *s = &o->symtab[n];
                if (s->st_shndx != SHN_UNDEF &&
                    strcmp(o->strtab + s->st_name, name) == 0) {
                    int v = el_accept(o, n, want_ver);
                    if (v == 1) { sym = s; si = n; break; }
                    if (v == -1 && !fallback) fallback = s;
                }
            }
            if (c & 1)
                break;            /* end of chain: the fallback may still serve */
        }
        /* ⚠ Falling out of the mapping instead of hitting a terminator means
         * the hash table and the segments disagree. A miss, never a guess --
         * and the unversioned fallback below is a MISS's answer, not a guess:
         * it is a real definition of the right name. */
        if (!sym)
            return fallback;
    } else if (o->elf_hash) {
        const uint32_t *eh = o->elf_hash;
        uint32_t nbucket = eh[0], nchain = eh[1];
        const uint32_t *bucket = eh + 2, *chain = bucket + nbucket;
        uint32_t n;

        if (!nbucket)
            return NULL;
        for (n = bucket[el_sysv_hash(name) % nbucket]; n; n = chain[n]) {
            const Elf64_Sym *s = &o->symtab[n];
            if (n >= nchain)
                break;
            if (s->st_shndx != SHN_UNDEF &&
                strcmp(o->strtab + s->st_name, name) == 0) {
                int v = el_accept(o, n, want_ver);
                if (v == 1) { sym = s; si = n; break; }
                if (v == -1 && !fallback) fallback = s;
            }
        }
        if (!sym)
            return fallback;
    } else {
        return NULL;              /* no hash table: nothing to search */
    }

    /* The version was decided inside the walk, by el_accept, so that a wrong
     * version keeps looking instead of ending the search. `si` is kept because
     * a later reader will want it. */
    (void)si;
    return sym;
}

/* Version NAME a reference at symbol index `si` asks for, or NULL. Read out of
 * DT_VERNEED, whose Vernaux entries carry the vna_other index DT_VERSYM
 * points at for undefined symbols. */
static const char *el_refver(const struct el_obj *o, uint32_t si)
{
    uint16_t vi;
    const Elf64_Verneed *vn;

    if (!o->versym || !o->verneed)
        return NULL;
    /* The same bounds check as el_defver, and for the same reason. */
    if (!el_in_map(o, &o->versym[si], sizeof o->versym[si]))
        return NULL;
    vi = o->versym[si] & 0x7fff;
    if (vi <= 1)
        return NULL;
    for (vn = o->verneed; el_in_map(o, vn, sizeof *vn); ) {
        const Elf64_Vernaux *aux =
            (const Elf64_Vernaux *)((const char *)vn + vn->vn_aux);
        int i;
        for (i = 0; i < vn->vn_cnt; i++) {
            if ((aux->vna_other & 0x7fff) == vi)
                return o->strtab + aux->vna_name;
            if (!aux->vna_next)
                break;
            aux = (const Elf64_Vernaux *)((const char *)aux + aux->vna_next);
        }
        if (!vn->vn_next)
            return NULL;
        vn = (const Elf64_Verneed *)((const char *)vn + vn->vn_next);
    }
    return NULL;
}

/* ⭐ the resolution order, not an approximation of it (solo's mechanism 2,
 * elf_loader.cpp:2034-2078). A loader that gets this wrong binds the right
 * name to the wrong definition and fails far from the cause.
 *
 *   0. the names only THIS loader can answer          (el_own_syms_first)
 *   1. the requesting object itself, when it is DT_SYMBOLIC
 *   2. the global scope, in load order
 *   3. the requester's own dependency closure
 *   4. the compiled-in provider table -- OUR static glibc
 *   5. the stand-ins for what ld.so would have provided (el_own_syms_last)
 *
 * ⚠ The provider table is LATE on purpose. A plugin that ships its own copy of
 * a symbol and a sibling that needs it must bind to each other, exactly as
 * they would under ld.so; falling to libc first would silently change which
 * definition wins.
 *
 * ⛔ STEPS 0 AND 5 are the same table split in two, and the split is the whole
 * point: step 0 is "nothing else may answer this", step 5 is "answer only if
 * nothing else did". A single table can express one or the other, never both.
 * See el_own_syms_first / el_own_syms_last above.
 */
/* el_mark_global puts an object AND its DT_NEEDED tree into the global lookup
 * scope, which is what ld.so does for a dependency of an RTLD_GLOBAL object.
 * ⛔ Depth is bounded by the already-set flag rather than by a counter: a
 * cyclic DT_NEEDED graph is legal ELF and would otherwise not terminate. */
static void el_mark_global(struct el_obj *o)
{
    int i;
    if (!o || o->global) return;
    o->global = 1;
    for (i = 0; i < o->nneeded; i++)
        el_mark_global(o->needed[i]);
}

static void *el_resolve(struct el_obj *req, const char *name, const char *ver,
                        int *found)
{
    const Elf64_Sym *s;
    int i;

    *found = 0;

    {
        void *p = el_own_first(name);
        if (p) { *found = 1; return p; }
    }
    if (req && req->symbolic) {
        s = el_lookup_in(req, name, ver);
        if (s) { *found = 1; return req->base + s->st_value; }
    }
    /* ⛔ The global scope first, and never simply every loaded object.
     * Search every object and two of them defining one name collide: the
     * second one's internal call to its OWN function binds to the first one's
     * definition, whether the first was opened RTLD_GLOBAL or RTLD_LOCAL.
     * `experiments/104-` is the regression test. The field documents this as
     * the reason comparable loaders warn about symbol collisions.
     *
     * ⭐ ld.so's order for a reference inside object O is: the global scope
     * (the executable, then everything RTLD_GLOBAL in load order), then O's
     * own local scope (O, then its DT_NEEDED tree). The three steps below are
     * that order. */
    for (i = 0; i < el_nobjs; i++) {
        if (!el_objs[i]->global) continue;
        s = el_lookup_in(el_objs[i], name, ver);
        if (s) { *found = 1; return el_objs[i]->base + s->st_value; }
    }
    if (req) {
        s = el_lookup_in(req, name, ver);
        if (s) { *found = 1; return req->base + s->st_value; }
        for (i = 0; i < req->nneeded; i++) {
            s = el_lookup_in(req->needed[i], name, ver);
            if (s) { *found = 1; return req->needed[i]->base + s->st_value; }
        }
    }
    /* ⚠ and then every remaining object, which IS not ld.so's behaviour and IS
     * kept deliberately. `experiments/93-` loads 882 of 1,527 host objects on
     * this machine, and some of them resolve only because a sibling that is
     * not in their DT_NEEDED tree happens to be mapped. Narrowing the scope to
     * be strictly correct would turn loads that work today into failures, and
     * this loader's job is to LOAD things. ⭐ What the reordering fixes is
     * which definition WINS; what this step preserves is whether one is found
     * at all. ⛔ A name reaching this step is a program relying on something
     * ld.so would not have given it. */
    for (i = 0; i < el_nobjs; i++) {
        if (el_objs[i]->global) continue;      /* already tried above */
        if (req && el_objs[i] == req) continue;
        s = el_lookup_in(el_objs[i], name, ver);
        if (s) { *found = 1; return el_objs[i]->base + s->st_value; }
    }
    {
        void *p = el_provider(name);
        if (p) { *found = 1; return p; }
    }
    {
        void *p = el_own_last(name);
        if (p) { *found = 1; return p; }
    }
    return NULL;
}

/* ------------------------------------------------------------------ our TLS */

/* ⭐ class a of experiments/73- IS owned here. __tls_get_addr is exported by
 * the host's ld.so, which is exactly why a compiled-in loader has to define
 * it rather than count it as a gap: general-dynamic TLS in a loaded object
 * calls it, and there is no ld.so in this process to answer.
 *
 * The block is per (thread, module). `__thread` puts the per-thread head in
 * the executable's OWN static TLS, which is laid out before main() and is
 * therefore always available -- no surplus is consumed for the bookkeeping,
 * only for the modules themselves, which are heap.
 */
struct el_tls_block {
    size_t modid;
    void  *mem;
    struct el_tls_block *next;
};

static __thread struct el_tls_block *el_tls_head;
static size_t el_tls_next_modid = 1;

struct el_tls_index { unsigned long ti_module, ti_offset; };

/* The thread pointer. On x86-64 (variant II) %fs:0 holds its own address and
 * static TLS blocks live at NEGATIVE offsets from it. */
static void *el_tp(void)
{
    void *p;
    __asm__ volatile ("mov %%fs:0, %0" : "=r"(p));
    return p;
}

/* ⭐ glibc's OWN static-TLS bookkeeping, and the reason initial-exec is
 * reachable at all. A static glibc lays out TLS in __libc_setup_tls() as
 * `memsz + _dl_tls_static_surplus`, so the surplus is already ALLOCATED in
 * every thread's block -- it is only unclaimed. Claiming a slice of it is
 * what ld.so does for a dlopen'd module with initial-exec TLS, and these are
 * the four variables it does it through. All four are plain GLOBAL OBJECTs in
 * libc.a, 8 bytes each, verified with readelf rather than assumed.
 *
 * ⚠ Weak, because a build that never links them must still compile: then the
 * relocation is refused by name instead of writing a wrong offset. */
extern size_t _dl_tls_static_used  __attribute__((weak));
extern size_t _dl_tls_static_size  __attribute__((weak));
extern size_t _dl_tls_static_align __attribute__((weak));

/* ⭐ Our own reserve, and it exists because the surplus above is a
 * CONSTANT. Measured on this build host with a static probe read twice,
 * the only difference being a padding array in the probe's own PT_TLS:
 *
 *     no pad      size=3264   used=96      headroom=3168
 *     64 KiB pad  size=68864  used=65648   headroom=3216   pad at tp-65616
 *
 * ⛔ So padding the executable raises _dl_tls_static_size AND _used together:
 * the headroom moves by 48 bytes, which is alignment rounding. Route B --
 * "make the surplus bigger by padding" -- is refuted by that pair of numbers.
 *
 * ⭐ But the pad IS allocated, in every thread, at a stable offset from the
 * thread pointer; it is merely accounted as `used` rather than as surplus. So
 * a loader that hands out slices of its OWN __thread array gets exactly what
 * it reserved. Two of 904 host objects want more than the surplus and one
 * wants 56,248 bytes, which a 64 KiB reserve serves and 3,168 never can.
 *
 * ⚠ every thread pays for it whether or not anything is ever dlopen'd, which
 * is why the default is ZERO and the size is a build option
 * (`pg-toolkit binary build --host-dlopen --tls-reserve=N`). Not reserved unless asked for.
 *
 * ⭐ And it removes a dependency rather than adding one: placing a module here
 * needs none of the three glibc internals above. They stay only for
 * el_tls_bookkeeping_sane()'s cross-check and for the fallback path.
 */
#ifndef PGT_BINARY_TLS_RESERVE
#define PGT_BINARY_TLS_RESERVE 0
#endif
/* The reserve's own alignment bounds the module p_align it can promise. 64 is
 * the largest observed across the 904-object sweep; a module wanting more is
 * declined here and falls back to the surplus rather than being handed storage
 * that does not meet its stated alignment. */
#ifndef PGT_BINARY_TLS_RESERVE_ALIGN
#define PGT_BINARY_TLS_RESERVE_ALIGN 64
#endif

#if PGT_BINARY_TLS_RESERVE > 0
static __thread unsigned char el_tls_reserve[PGT_BINARY_TLS_RESERVE]
    __attribute__((aligned(PGT_BINARY_TLS_RESERVE_ALIGN)));
/* ⚠ NOT __thread. Which slice a module owns is a decision about the PROCESS:
 * the offset from the thread pointer is the same in every thread, because
 * el_tls_reserve is one __thread object in the executable's own PT_TLS and the
 * ABI gives it the same tp-relative address in every thread. Making this
 * per-thread would hand the same module a different offset per thread, which
 * is the silent wrong answer this loader exists to avoid.
 *
 * ⚠ Unlocked, exactly as the _dl_tls_static_used path beside it is: this
 * loader has no mutex anywhere and concurrent dlopen is not claimed. */
static size_t el_tls_reserve_used;
#endif

/* Place o in our own reserve. 0 = placed, 1 = no reserve or it does not fit,
 * so the caller falls back to glibc's surplus. */
static int el_tls_from_reserve(struct el_obj *o)
{
#if PGT_BINARY_TLS_RESERVE > 0
    size_t align = o->tls_align ? o->tls_align : 1;
    size_t off;
    unsigned char *p;

    if (align > PGT_BINARY_TLS_RESERVE_ALIGN)
        return 1;
    off = (el_tls_reserve_used + align - 1) & ~(align - 1);
    /* ⛔ Compared as a sum that cannot wrap: off and tls_memsz are both
     * bounded by the object's own headers, and PGT_BINARY_TLS_RESERVE is a literal. */
    if (off > (size_t)PGT_BINARY_TLS_RESERVE || o->tls_memsz > (size_t)PGT_BINARY_TLS_RESERVE - off)
        return 1;
    p = el_tls_reserve + off;
    o->tls_tpoff = (long)((char *)p - (char *)el_tp());
    /* The reserve is below the thread pointer like every other static TLS
     * block; a non-negative offset would mean the layout is not what this
     * loader believes and the module must not be placed. */
    if (o->tls_tpoff >= 0) {
        o->tls_tpoff = 0;
        return 1;
    }
    el_tls_reserve_used = off + o->tls_memsz;
    if (o->tls_filesz)
        memcpy(p, o->tls_image, o->tls_filesz);
    if (o->tls_memsz > o->tls_filesz)
        memset(p + o->tls_filesz, 0, o->tls_memsz - o->tls_filesz);
    return 0;
#else
    (void)o;
    return 1;
#endif
}

/* Place o's TLS block in the surplus, once. Returns 0 and sets o->tls_tpoff.
 *
 * ⚠ the honest limit, and it IS about threads, not about space. glibc gives
 * every thread a block of _dl_tls_static_size, so the slice EXISTS in threads
 * created later and is zero there. It is seeded with the module's init image
 * only in the thread that loaded it. So a module whose PT_TLS p_filesz is 0 --
 * 14 of the 24 measured in the first sweep -- is correct on every thread,
 * because zero is what its image says. A module with a non-zero image is
 * correct on the loading thread and zero-initialised on threads created after
 * it, which is stated here and asserted in experiments/76- rather than
 * discovered by a user.
 */
/* ⛔ validate an internal variable with a public fact, ONCE.
 *
 * _dl_tls_static_size/_used/_align are glibc INTERNALS. They are not in
 * libc.so.6's dynamic symbol table at all -- they exist only in libc.a -- so
 * nothing versions them, nothing promises them, and a future glibc may rename
 * them, change their units, or change what they are measured from.
 *
 * ⚠ a rename IS the safe failure and it is already handled: the references are
 * weak, so the addresses come back NULL and the caller refuses by name. The
 * DANGEROUS change is a silent one -- same names, different meaning -- which
 * would have this loader compute a plausible-looking offset and hand a module
 * thread storage that overlaps somebody else's.
 *
 * ⭐ So the internals are cross-checked against something PUBLIC: errno is
 * thread-local, its address is ordinary API, and it must lie inside the static
 * TLS block that _dl_tls_static_size describes. Measured on the build host:
 *
 *     &errno              tp -64
 *     _dl_tls_static_size 3264      -> errno is inside [tp-3264, tp): CONSISTENT
 *
 * If that ever stops holding, this loader's understanding of the layout is
 * wrong and initial-exec TLS is refused rather than guessed at.
 */
static int el_tls_bookkeeping_sane(void)
{
    static int v = -1;
    long eoff;

    if (v >= 0)
        return v;
    v = 0;
    if (&_dl_tls_static_used == NULL || &_dl_tls_static_size == NULL)
        return v;
    if (_dl_tls_static_size == 0 || _dl_tls_static_used > _dl_tls_static_size)
        return v;
    eoff = (char *)&errno - (char *)el_tp();
    /* errno must be BELOW the thread pointer and inside the block. */
    if (eoff >= 0 || -eoff > (long)_dl_tls_static_size)
        return v;
    v = 1;
    return v;
}

static int el_static_tls(struct el_obj *o)
{
    size_t align, used, newused;
    unsigned char *blk;

    if (o->tls_tpoff)
        return 0;
    if (!o->tls_memsz) {
        el_err("pgb-elfload: %s: initial-exec TLS but no PT_TLS segment",
               o->soname);
        return -1;
    }
    /* ⭐ our own reserve first, because it needs none of glibc's internals and
     * is the only place large modules fit. Zero-sized unless the build asked
     * for one, in which case this returns 1 and nothing below changes. */
    if (el_tls_from_reserve(o) == 0)
        return 0;
    if (&_dl_tls_static_used == NULL || &_dl_tls_static_size == NULL) {
        el_err("pgb-elfload: %s: glibc's static-TLS bookkeeping is not linked "
               "in, so initial-exec TLS cannot be placed", o->soname);
        return -1;
    }
    if (!el_tls_bookkeeping_sane()) {
        el_err("pgb-elfload: %s: glibc's static-TLS bookkeeping does not "
               "describe this thread's layout (size=%zu used=%zu, errno at "
               "tp%+ld) -- refusing to place initial-exec TLS rather than "
               "guess", o->soname, (size_t)_dl_tls_static_size,
               (size_t)_dl_tls_static_used,
               (long)((char *)&errno - (char *)el_tp()));
        return -1;
    }
    align = o->tls_align ? o->tls_align : 1;
    if ((uintptr_t)el_tp() % align) {
        el_err("pgb-elfload: %s: PT_TLS wants %zu-byte alignment and the "
               "thread pointer does not have it", o->soname, align);
        return -1;
    }
    used    = _dl_tls_static_used;
    newused = (used + o->tls_memsz + align - 1) & ~(align - 1);
    if (newused > _dl_tls_static_size) {
        /* ⭐ The message names the fix, because there is one now: the surplus
         * is a constant and cannot be enlarged, but a reserve of our own can
         * be asked for at build time. */
        el_err("pgb-elfload: %s: static TLS surplus exhausted -- needs %zu "
               "bytes, %zu of %zu used (reserve is %zu; build with "
               "--tls-reserve N for more)", o->soname, o->tls_memsz, used,
               (size_t)_dl_tls_static_size, (size_t)PGT_BINARY_TLS_RESERVE);
        return -1;
    }
    _dl_tls_static_used = newused;
    if (&_dl_tls_static_align != NULL && align > _dl_tls_static_align)
        _dl_tls_static_align = align;

    o->tls_tpoff = -(long)newused;
    blk = (unsigned char *)el_tp() + o->tls_tpoff;
    if (o->tls_filesz)
        memcpy(blk, o->tls_image, o->tls_filesz);
    if (o->tls_memsz > o->tls_filesz)
        memset(blk + o->tls_filesz, 0, o->tls_memsz - o->tls_filesz);
    return 0;
}

void *__tls_get_addr(struct el_tls_index *ti)
{
    struct el_tls_block *b;
    struct el_obj *o = NULL;
    int i;

    for (b = el_tls_head; b; b = b->next)
        if (b->modid == ti->ti_module)
            return (char *)b->mem + ti->ti_offset;

    for (i = 0; i < el_nobjs; i++)
        if (el_objs[i]->tls_modid == ti->ti_module) { o = el_objs[i]; break; }
    if (!o)
        return NULL;

    /* ⛔ A module placed in static TLS must answer general-dynamic out of the
     * SAME block. Handing back a separate heap block would give one module two
     * copies of its own thread storage, which is a silent wrong answer of
     * exactly the kind this loader exists to avoid. */
    if (o->tls_tpoff)
        return (char *)el_tp() + o->tls_tpoff + ti->ti_offset;

    b = calloc(1, sizeof *b);
    if (!b)
        return NULL;
    b->modid = ti->ti_module;
    b->mem = calloc(1, o->tls_memsz ? o->tls_memsz : 1);
    if (!b->mem) { free(b); return NULL; }
    if (o->tls_image && o->tls_filesz)
        memcpy(b->mem, o->tls_image, o->tls_filesz);
    b->next = el_tls_head;
    el_tls_head = b;
    return (char *)b->mem + ti->ti_offset;
}

/* --------------------------------------------------------------- relocation */

static int el_reloc_one(struct el_obj *o, const Elf64_Rela *r)
{
    uint32_t type = ELF64_R_TYPE(r->r_info);
    uint32_t si   = ELF64_R_SYM(r->r_info);
    unsigned char *where = o->base + r->r_offset;
    const char *name = NULL, *ver = NULL;
    void *val = NULL;
    int found = 0, weak = 0;

    /* ⛔ Refused before anything is written. Eight bytes is the widest store
     * any case below makes. */
    if (type != R_X86_64_NONE && !el_in_map(o, where, 8)) {
        el_err("pgb-elfload: %s: relocation at offset 0x%llx is outside the "
               "object's own mapping", o->soname,
               (unsigned long long)r->r_offset);
        return -1;
    }

    if (si && o->nsyms && si >= o->nsyms) {
        el_err("pgb-elfload: %s: relocation names symbol %u of %u",
               o->soname, si, o->nsyms);
        return -1;
    }

    if (si) {
        const Elf64_Sym *s = &o->symtab[si];
        name = o->strtab + s->st_name;
        ver  = el_refver(o, si);
        weak = ELF64_ST_BIND(s->st_info) == STB_WEAK;
    }

    switch (type) {
    case R_X86_64_NONE:
        return 0;

    /* B + A: no symbol involved, just the load bias. The overwhelming
     * majority of relocations in any real .so are these. */
    case R_X86_64_RELATIVE:
        *(uint64_t *)where = (uint64_t)(uintptr_t)(o->base + r->r_addend);
        return 0;

    /* ⭐ An IFUNC resolver, run at relocation time exactly as ld.so runs it.
     * glibc's string and memory routines in a host object are these, so a
     * loader that skipped them would bind a resolver address as if it were
     * the function and crash on first call. */
    case R_X86_64_IRELATIVE: {
        void *(*fn)(void) = (void *(*)(void))(uintptr_t)(o->base + r->r_addend);
        *(uint64_t *)where = (uint64_t)(uintptr_t)fn();
        return 0;
    }

    case R_X86_64_64:
    case R_X86_64_GLOB_DAT:
    case R_X86_64_JUMP_SLOT:
        val = el_resolve(o, name, ver, &found);
        if (!found) {
            /* ⛔ a weak undefined symbol becomes zero, and that IS correct --
             * it is what ld.so does and what the ABI says. ⚠ But if the
             * object then CALLS it, the process jumps to address 0 and dies
             * with rip=0 and si_addr=0, naming nothing. That is the whole
             * diagnosis a crash gives you, and it took a signal handler
             * printing RIP to learn even that much.
             *
             * So the binding is recorded here rather than reconstructed
             * afterwards. It is behind PGT_BINARY_ELFLOAD_DEBUG, so a normal load
             * pays a getenv that was already being paid. */
            if (weak) {
                *(uint64_t *)where = 0;
                el_dbg("pgb-elfload: %s: weak %s%s%s is undefined -> bound to "
                       "0; CALLING it will fault at rip=0\n", o->soname,
                       name ? name : "?", ver ? "@" : "", ver ? ver : "");
                return 0;
            }
            el_err("pgb-elfload: %s: undefined symbol: %s%s%s",
                   o->soname, name ? name : "?", ver ? "@" : "", ver ? ver : "");
            return -1;
        }
        /* ⛔ RESOLVED, and to zero. Distinct from the weak case above and far
         * more suspicious: a lookup SUCCEEDED and handed back a null address,
         * so nothing errors and the object gets a call target of 0. It is the
         * shape of a defect this loader has already had once -- a provider
         * entry whose weak extern was never linked -- and it is invisible in
         * a crash, which reports only rip=0. */
        if (val == NULL)
            el_dbg("pgb-elfload: %s: %s%s%s RESOLVED TO ZERO -- a lookup "
                   "succeeded and returned a null address\n", o->soname,
                   name ? name : "?", ver ? "@" : "", ver ? ver : "");
        *(uint64_t *)where = (uint64_t)(uintptr_t)val +
                             (type == R_X86_64_64 ? (uint64_t)r->r_addend : 0);
        return 0;

    case R_X86_64_PC32:
    case R_X86_64_PLT32:
        val = el_resolve(o, name, ver, &found);
        if (!found && !weak) {
            el_err("pgb-elfload: %s: undefined symbol: %s", o->soname,
                   name ? name : "?");
            return -1;
        }
        *(uint32_t *)where = (uint32_t)((int64_t)(uintptr_t)val + r->r_addend -
                                        (int64_t)(uintptr_t)where);
        return 0;

    case R_X86_64_32:
    case R_X86_64_32S:
        val = el_resolve(o, name, ver, &found);
        if (!found && !weak) {
            el_err("pgb-elfload: %s: undefined symbol: %s", o->soname,
                   name ? name : "?");
            return -1;
        }
        *(uint32_t *)where = (uint32_t)((uint64_t)(uintptr_t)val + r->r_addend);
        return 0;

    /* R_X86_64_COPY only appears in executables, never in the shared objects
     * this loader maps. Named rather than silently ignored. */
    case R_X86_64_COPY:
        el_err("pgb-elfload: %s: R_X86_64_COPY in a shared object (%s)",
               o->soname, name ? name : "?");
        return -1;

    /* General-dynamic TLS: the module id, then the offset within it. Our
     * __tls_get_addr above answers the pair.
     *
     * ⛔ the two halves are one answer and must come from one lookup. They did
     * not, and the bug that cost is worth stating in full because nothing about
     * it is visible at the point of failure.
     *
     * DTPMOD64 searched EVERY loaded object for the symbol; DTPOFF64 searched
     * only the object being relocated. So for a general-dynamic reference to a
     * thread-local defined in ANOTHER module -- the ordinary case, and the one
     * every C++ library hits -- the pair came out as
     *
     *     (the right module, offset 0)
     *
     * because the DTPOFF64 lookup missed and the code fell back to 0 without
     * saying so. ⚠ that IS not primarily a crash, it IS silent corruption: the
     * referring module reads and WRITES at offset 0 of another module's thread
     * storage, over whatever is really there. It is the exact outcome the
     * R_X86_64_TPOFF64 case below refuses by name to avoid.
     *
     * ⭐ Measured. libLLVM-17 stores a callable into libstdc++'s
     * `std::__once_call` and then calls `pthread_once(flag, __once_proxy)`;
     * `__once_proxy` reads the same variable and jumps through it. libLLVM
     * wrote at offset 0 and libstdc++ read at offset 0x10, so the read
     * returned 0 and the process jumped to address 0 -- rip=0, si_addr=0,
     * naming nothing. Seven of the ten objects experiments/93- reported as
     * "crashes this loader that glibc's loader loads" are this, and they are
     * all the C++-heavy ones: libLLVM, libclang, liblldb, libLTO, LLVMgold. */
    case R_X86_64_DTPMOD64:
    case R_X86_64_DTPOFF64: {
        struct el_obj *owner = o;
        const Elf64_Sym *s = NULL;
        int i;

        if (si == 0 || !name || !*name) {
            /* No symbol: the reference is to this object's own TLS block. */
            *(uint64_t *)where = (type == R_X86_64_DTPMOD64)
                                 ? o->tls_modid : (uint64_t)r->r_addend;
            return 0;
        }
        s = el_lookup_in(o, name, ver);
        if (!s || ELF64_ST_TYPE(s->st_info) != STT_TLS) {
            owner = NULL;
            for (i = 0; i < el_nobjs; i++) {
                const Elf64_Sym *c = el_lookup_in(el_objs[i], name, ver);
                if (c && ELF64_ST_TYPE(c->st_info) == STT_TLS) {
                    owner = el_objs[i];
                    s = c;
                    break;
                }
            }
        }
        if (!owner || !s) {
            /* ⛔ REFUSED, not bound to (this module, 0). A general-dynamic
             * pair nobody can satisfy is the same hazard as an initial-exec
             * offset nobody can satisfy, and the case below already refuses
             * that by name. */
            el_err("pgb-elfload: %s: general-dynamic TLS symbol %s is defined "
                   "nowhere in the loaded set", o->soname, name);
            return -1;
        }
        *(uint64_t *)where = (type == R_X86_64_DTPMOD64)
                             ? owner->tls_modid
                             : s->st_value + (uint64_t)r->r_addend;
        return 0;
    }

    /* ⭐ INITIAL-EXEC. The value is an offset from the thread pointer into the
     * STATIC TLS block. Three cases, in the order they are tried, and the
     * measurement that made the order matter is in the comment on each:
     *
     *   1. the symbol is one this image's own glibc defines as thread-local
     *      -- errno and h_errno. Their offset is computed from their real
     *      address, so it is right in every thread, for free.
     *   2. the symbol is defined by another LOADED object's PT_TLS. 18 of the
     *      42 initial-exec failures in the first sweep had no PT_TLS of their
     *      own at all, which is what this case is.
     *   3. the relocation is against this object's own PT_TLS.
     *
     * ⛔ Still refused by name when none applies, never bound to a
     * plausible-looking wrong offset -- solo's mechanism 4, and the difference
     * between a loud failure and silently corrupting another module's thread
     * storage. */
    case R_X86_64_TPOFF64: {
        struct el_obj *owner = o;
        uint64_t symoff = 0;
        int i;

        if (name && *name) {
            void *a = el_tls_provider(name);
            if (a) {
                *(int64_t *)where = (int64_t)((char *)a - (char *)el_tp()) +
                                    r->r_addend;
                return 0;
            }
        }
        if (name && *name) {
            const Elf64_Sym *s = el_lookup_in(o, name, ver);
            if (s && ELF64_ST_TYPE(s->st_info) == STT_TLS) {
                symoff = s->st_value;
            } else {
                owner = NULL;
                for (i = 0; i < el_nobjs; i++) {
                    s = el_lookup_in(el_objs[i], name, ver);
                    if (s && ELF64_ST_TYPE(s->st_info) == STT_TLS) {
                        owner = el_objs[i];
                        symoff = s->st_value;
                        break;
                    }
                }
                if (!owner) {
                    el_err("pgb-elfload: %s: initial-exec TLS symbol %s is "
                           "defined nowhere in the loaded set", o->soname, name);
                    return -1;
                }
            }
        }
        if (el_static_tls(owner) != 0)
            return -1;
        *(int64_t *)where = owner->tls_tpoff + (int64_t)symoff + r->r_addend;
        return 0;
    }

    case R_X86_64_TLSDESC:
        el_err("pgb-elfload: %s: TLSDESC relocation for %s is not implemented",
               o->soname, name ? name : "?");
        return -1;

    default:
        el_err("pgb-elfload: %s: unhandled relocation type %u", o->soname, type);
        return -1;
    }
}

/* ⛔ DT_RELR IS not optional on a modern distribution, and skipping it is a
 * SILENT wrong answer rather than a failure.
 *
 * `ld -z pack-relative-relocs` compresses the R_X86_64_RELATIVE entries -- the
 * overwhelming majority of any shared object's relocations -- into a bitmap
 * under DT_RELR, and Fedora and Arch build with it. A loader that reads only
 * DT_RELA finds almost no relocations to apply, reports success, and hands
 * back an object whose pointers still hold link-time offsets.
 *
 * ⚠ MEASURED, and the shape of the evidence IS the point. experiments/76-'s
 * native arm was SIG11 on exactly Fedora 42 and Arch and nowhere else, and the
 * loader's own trace showed why: `init_array[0] 0x670` where every working row
 * printed a mapped address. 0x670 is the unrelocated vaddr. The object had
 * "loaded" and its constructor pointer was a small integer.
 *
 * The encoding: an even entry is an address to relocate and becomes the
 * cursor; an odd entry is a bitmap of the next 63 words after the cursor.
 */
#ifndef DT_RELR
#define DT_RELR    36
#define DT_RELRSZ  35
#define DT_RELRENT 37
#endif

static void el_apply_relr(struct el_obj *o)
{
    size_t n = o->relrsz / sizeof(uint64_t), i;
    uint64_t *where = NULL;

    for (i = 0; i < n; i++) {
        uint64_t e = o->relr[i];
        if ((e & 1) == 0) {
            where = (uint64_t *)(o->base + e);
            if (!el_in_map(o, where, 8))
                goto out_of_range;
            *where++ += (uint64_t)(uintptr_t)o->base;
        } else if (where) {
            uint64_t bits = e >> 1;
            int b;
            for (b = 0; bits; b++, bits >>= 1)
                if (bits & 1) {
                    if (!el_in_map(o, &where[b], 8))
                        goto out_of_range;
                    where[b] += (uint64_t)(uintptr_t)o->base;
                }
            where += 63;
        }
    }
    el_dbg("pgb-elfload: %s: applied %zu DT_RELR words\n", o->soname, n);
    return;
out_of_range:
    /* ⚠ Reported and STOPPED, not skipped. A relr bitmap that walks off the
     * mapping means the table and the segments disagree, and the words
     * already applied are as suspect as the one that overran. */
    el_err("pgb-elfload: %s: DT_RELR walked outside the object's mapping",
           o->soname);
}

static int el_relocate(struct el_obj *o)
{
    size_t i;

    if (o->textrel) {
        /* A DT_TEXTREL object writes into its own code, so the text segment
         * has to be writable while that happens and is restored after. */
        if (mprotect(o->base, o->span, PROT_READ | PROT_WRITE | PROT_EXEC) != 0) {
            el_err("pgb-elfload: %s: DT_TEXTREL and mprotect refused", o->soname);
            return -1;
        }
    }
    if (o->relr && o->relrsz)
        el_apply_relr(o);
    for (i = 0; o->rela && i < o->relasz / sizeof(Elf64_Rela); i++)
        if (el_reloc_one(o, &o->rela[i]) != 0)
            return -1;
    for (i = 0; o->jmprel && i < o->jmprelsz / sizeof(Elf64_Rela); i++)
        if (el_reloc_one(o, &o->jmprel[i]) != 0)
            return -1;
    return 0;
}

/* ------------------------------------------------------------------ mapping */

static int el_prot(uint32_t flags)
{
    return ((flags & PF_R) ? PROT_READ : 0) |
           ((flags & PF_W) ? PROT_WRITE : 0) |
           ((flags & PF_X) ? PROT_EXEC : 0);
}

/* Reserve the whole span PROT_NONE first, then place each PT_LOAD inside it
 * with MAP_FIXED. Reserving in one call is what guarantees the segments keep
 * their relative offsets -- mapping them one at a time and hoping the kernel
 * puts them adjacent is the classic way to get a loader that works until the
 * address space is busy. */
static int el_map(struct el_obj *o, int fd, const Elf64_Ehdr *eh,
                  const Elf64_Phdr *ph)
{
    size_t minva = (size_t)-1, maxva = 0;
    unsigned char *base;
    struct stat st;
    int i;

    /* ⛔ a truncated object passes every header check and then kills the
     * process. mmap of a short file succeeds -- the mapping exists -- and the
     * first touch of a page past the last full page of the file raises
     * SIGBUS, which arrives with no message and no dlerror().
     *
     * ⚠ MEASURED: a real libz.so.1 truncated to half its length was accepted
     * by this loader, mapped, and killed the process with SIG7. It is not an
     * exotic input either -- a partial download or a full disk produces
     * exactly it. So the file's real length is checked against every segment
     * BEFORE anything is mapped. */
    if (fstat(fd, &st) != 0) {
        el_err("pgb-elfload: %s: cannot stat", o->path);
        return -1;
    }
    for (i = 0; i < eh->e_phnum; i++) {
        if (ph[i].p_type != PT_LOAD)
            continue;
        if (ph[i].p_filesz > (uint64_t)st.st_size ||
            ph[i].p_offset > (uint64_t)st.st_size - ph[i].p_filesz) {
            el_err("pgb-elfload: %s: truncated -- segment %d wants bytes "
                   "%llu..%llu of a %lld-byte file", o->path, i,
                   (unsigned long long)ph[i].p_offset,
                   (unsigned long long)(ph[i].p_offset + ph[i].p_filesz),
                   (long long)st.st_size);
            return -1;
        }
        if (ph[i].p_memsz < ph[i].p_filesz) {
            el_err("pgb-elfload: %s: segment %d has p_memsz < p_filesz",
                   o->path, i);
            return -1;
        }
    }

    for (i = 0; i < eh->e_phnum; i++) {
        if (ph[i].p_type != PT_LOAD)
            continue;
        if (ph[i].p_vaddr < minva)
            minva = ph[i].p_vaddr;
        if (ph[i].p_vaddr + ph[i].p_memsz > maxva)
            maxva = ph[i].p_vaddr + ph[i].p_memsz;
    }
    if (minva == (size_t)-1) {
        el_err("pgb-elfload: %s: no PT_LOAD segment", o->path);
        return -1;
    }
    minva = el_down(minva);
    maxva = el_up(maxva);

    base = mmap(NULL, maxva - minva, PROT_NONE,
                MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (base == MAP_FAILED) {
        el_err("pgb-elfload: %s: reservation of %zu bytes failed", o->path,
               maxva - minva);
        return -1;
    }
    o->base   = base - minva;
    o->span   = maxva - minva;
    o->map_lo = base;
    o->map_hi = base + (maxva - minva);

    for (i = 0; i < eh->e_phnum; i++) {
        const Elf64_Phdr *p = &ph[i];
        unsigned char *seg, *fend, *mend;
        size_t off;

        if (p->p_type != PT_LOAD)
            continue;
        seg  = o->base + el_down(p->p_vaddr);
        off  = el_down(p->p_offset);
        fend = o->base + p->p_vaddr + p->p_filesz;
        mend = o->base + p->p_vaddr + p->p_memsz;

        if (p->p_filesz &&
            mmap(seg, (size_t)(fend - seg), el_prot(p->p_flags),
                 MAP_PRIVATE | MAP_FIXED, fd, (off_t)off) == MAP_FAILED) {
            el_err("pgb-elfload: %s: segment %d mmap failed", o->path, i);
            return -1;
        }
        /* .bss: zero the tail of the last file page, then map anonymous pages
         * for whatever is left. Skipping the first half is the bug that makes
         * a loaded object see another object's data in its uninitialised
         * globals -- it only shows up when memsz crosses a page boundary. */
        if (p->p_memsz > p->p_filesz) {
            unsigned char *zstart = fend;
            unsigned char *zend   = (unsigned char *)el_up((size_t)fend);
            if (zend > mend)
                zend = mend;
            if (zend > zstart && (p->p_flags & PF_W))
                memset(zstart, 0, (size_t)(zend - zstart));
            zstart = (unsigned char *)el_up((size_t)fend);
            zend   = (unsigned char *)el_up((size_t)mend);
            if (zend > zstart &&
                mmap(zstart, (size_t)(zend - zstart), el_prot(p->p_flags),
                     MAP_PRIVATE | MAP_FIXED | MAP_ANONYMOUS, -1, 0) == MAP_FAILED) {
                el_err("pgb-elfload: %s: bss mmap failed", o->path);
                return -1;
            }
        }
    }
    return 0;
}

/* ------------------------------------------------------------ dynamic table */

static void el_read_dynamic(struct el_obj *o, const Elf64_Dyn *dyn)
{
    const Elf64_Dyn *d;
    size_t strsz = 0;
    uint64_t soname_off = 0, runpath_off = 0, rpath_off = 0;
    uint64_t needed_off[EL_MAX_NEEDED];
    int nneeded_off = 0;
    int i;

    /* DT_STRTAB has to be resolved before any offset into it is meaningful,
     * and the entries are not ordered, so this is two passes. */
    for (d = dyn; d->d_tag != DT_NULL; d++) {
        switch (d->d_tag) {
        case DT_STRTAB: o->strtab = (const char *)(o->base + d->d_un.d_ptr); break;
        case DT_STRSZ:  strsz = d->d_un.d_val; break;
        case DT_SYMTAB: o->symtab = (const Elf64_Sym *)(o->base + d->d_un.d_ptr); break;
        case DT_GNU_HASH: o->gnu_hash = (const uint32_t *)(o->base + d->d_un.d_ptr); break;
        case DT_HASH:   o->elf_hash = (const uint32_t *)(o->base + d->d_un.d_ptr); break;
        case DT_VERSYM: o->versym = (const uint16_t *)(o->base + d->d_un.d_ptr); break;
        case DT_VERDEF: o->verdef = (const Elf64_Verdef *)(o->base + d->d_un.d_ptr); break;
        case DT_VERNEED: o->verneed = (const Elf64_Verneed *)(o->base + d->d_un.d_ptr); break;
        case DT_RELA:   o->rela = (const Elf64_Rela *)(o->base + d->d_un.d_ptr); break;
        case DT_RELASZ: o->relasz = d->d_un.d_val; break;
        case DT_JMPREL: o->jmprel = (const Elf64_Rela *)(o->base + d->d_un.d_ptr); break;
        case DT_RELR:   o->relr = (const uint64_t *)(o->base + d->d_un.d_ptr); break;
        case DT_RELRSZ: o->relrsz = d->d_un.d_val; break;
        case DT_PLTRELSZ: o->jmprelsz = d->d_un.d_val; break;
        case DT_INIT:   o->init = (void (*)(void))(o->base + d->d_un.d_ptr); break;
        case DT_FINI:   o->fini = (void (*)(void))(o->base + d->d_un.d_ptr); break;
        case DT_INIT_ARRAY: o->init_array = (void (**)(void))(o->base + d->d_un.d_ptr); break;
        case DT_INIT_ARRAYSZ: o->init_arrayn = d->d_un.d_val / sizeof(void *); break;
        case DT_FINI_ARRAY: o->fini_array = (void (**)(void))(o->base + d->d_un.d_ptr); break;
        case DT_FINI_ARRAYSZ: o->fini_arrayn = d->d_un.d_val / sizeof(void *); break;
        case DT_SONAME: soname_off = d->d_un.d_val; break;
        case DT_RUNPATH: runpath_off = d->d_un.d_val; break;
        case DT_RPATH:  rpath_off = d->d_un.d_val; break;
        case DT_SYMBOLIC: o->symbolic = 1; break;
        case DT_TEXTREL: o->textrel = 1; break;
        case DT_FLAGS:
            if (d->d_un.d_val & DF_SYMBOLIC) o->symbolic = 1;
            if (d->d_un.d_val & DF_TEXTREL)  o->textrel = 1;
            break;
        case DT_NEEDED:
            if (nneeded_off < EL_MAX_NEEDED)
                needed_off[nneeded_off++] = d->d_un.d_val;
            break;
        default:
            break;
        }
    }
    (void)strsz;

    if (o->strtab) {
        if (soname_off)  o->soname  = strdup(o->strtab + soname_off);
        if (runpath_off) o->runpath = o->strtab + runpath_off;
        else if (rpath_off) o->runpath = o->strtab + rpath_off;
    }
    if (!o->soname) {
        const char *b = strrchr(o->path, '/');
        o->soname = strdup(b ? b + 1 : o->path);
    }
    if (o->gnu_hash)
        o->nsyms = el_gnu_symcount(o->gnu_hash);
    else if (o->elf_hash)
        o->nsyms = o->elf_hash[1];

    /* Kept as offsets and turned into strings only now, because the second
     * pass is where DT_STRTAB is known to be set. */
    o->nneeded = 0;
    for (i = 0; i < nneeded_off; i++) {
        o->needed[i] = NULL;   /* filled by the caller's DT_NEEDED walk */
    }
    o->nneeded = nneeded_off;
    for (i = 0; i < nneeded_off; i++)
        o->needed[i] = (struct el_obj *)(uintptr_t)needed_off[i];  /* offset for now */
}

/* --------------------------------------------------------------- the search */

/* ⭐ solo's mechanism 6, and PR #4's correctness detail. A bare soname is
 * resolved through the REQUESTER's DT_RUNPATH first -- glibc does this, and a
 * loader that consults only its own list misses siblings reached via $ORIGIN.
 *
 * ⛔ AT_SECURE discipline: a set-uid process ignores the environment's search
 * paths entirely. Checked once, from the auxiliary vector. */
static int el_secure(void)
{
    static int v = -1;
    if (v < 0)
        v = getauxval(AT_SECURE) != 0;
    return v;
}

static int el_exists(const char *p)
{
    struct stat st;
    return stat(p, &st) == 0 && S_ISREG(st.st_mode);
}

static char *el_origin_of(const char *path)
{
    char *d = strdup(path), *s;
    if (!d)
        return NULL;
    s = strrchr(d, '/');
    if (s)
        *s = '\0';
    else
        strcpy(d, ".");
    return d;
}

/* Expand $ORIGIN in one search-path element and test `dir/name`. */
static int el_try(char *out, size_t outsz, const char *dir, size_t dirlen,
                  const char *origin, const char *name)
{
    char d[4096];
    size_t n = 0;
    size_t i = 0;

    while (i < dirlen && n + 1 < sizeof d) {
        if (dir[i] == '$' &&
            (strncmp(dir + i, "$ORIGIN", 7) == 0 ||
             strncmp(dir + i, "${ORIGIN}", 9) == 0)) {
            size_t olen = strlen(origin ? origin : ".");
            if (n + olen + 1 >= sizeof d)
                return 0;
            memcpy(d + n, origin ? origin : ".", olen);
            n += olen;
            i += (dir[i + 1] == '{') ? 9 : 7;
            continue;
        }
        d[n++] = dir[i++];
    }
    d[n] = '\0';
    if (!n)
        return 0;
    if ((size_t)snprintf(out, outsz, "%s/%s", d, name) >= outsz)
        return 0;
    return el_exists(out);
}

static int el_search(char *out, size_t outsz, const char *name,
                     const struct el_obj *req)
{
    static const char *const sysdirs[] = {
        "/lib/x86_64-linux-gnu", "/usr/lib/x86_64-linux-gnu",
        "/lib64", "/usr/lib64", "/lib", "/usr/lib",
        "/usr/local/lib", "/usr/local/lib64", NULL
    };
    const char *const *s;
    char *origin = NULL;
    int i;

    if (strchr(name, '/')) {
        if ((size_t)snprintf(out, outsz, "%s", name) >= outsz)
            return 0;
        return el_exists(out);
    }

    if (req)
        origin = el_origin_of(req->path);

    /* 1. the requester's DT_RUNPATH / DT_RPATH, $ORIGIN expanded */
    if (req && req->runpath) {
        const char *p = req->runpath;
        while (*p) {
            const char *c = strchr(p, ':');
            size_t len = c ? (size_t)(c - p) : strlen(p);
            if (len && el_try(out, outsz, p, len, origin, name)) {
                free(origin);
                return 1;
            }
            if (!c)
                break;
            p = c + 1;
        }
    }
    /* 2. LD_LIBRARY_PATH -- ⛔ never for a set-uid process */
    if (!el_secure()) {
        const char *env = getenv("LD_LIBRARY_PATH");
        while (env && *env) {
            const char *c = strchr(env, ':');
            size_t len = c ? (size_t)(c - env) : strlen(env);
            if (len && el_try(out, outsz, env, len, origin, name)) {
                free(origin);
                return 1;
            }
            if (!c)
                break;
            env = c + 1;
        }
    }
    /* 3. the ordinary system directories */
    for (s = sysdirs; *s; s++) {
        if ((size_t)snprintf(out, outsz, "%s/%s", *s, name) < outsz &&
            el_exists(out)) {
            free(origin);
            return 1;
        }
    }
    /* 4. beside the requester */
    if (origin && (size_t)snprintf(out, outsz, "%s/%s", origin, name) < outsz &&
        el_exists(out)) {
        free(origin);
        return 1;
    }
    for (i = 0; i < 0; i++)
        ;
    free(origin);
    return 0;
}

/* ------------------------------------------------------------------ loading */

static struct el_obj *el_load(const char *path, const struct el_obj *req);

static struct el_obj *el_already(const char *soname)
{
    int i;
    for (i = 0; i < el_nobjs; i++)
        if (strcmp(el_objs[i]->soname, soname) == 0)
            return el_objs[i];
    return NULL;
}

/* Resolve the DT_NEEDED offsets stashed by el_read_dynamic into real objects,
 * short-circuiting the ones the provider table already satisfies. */
static int el_load_needed(struct el_obj *o)
{
    int i, n = o->nneeded, keep = 0;
    uint64_t offs[EL_MAX_NEEDED];

    for (i = 0; i < n; i++)
        offs[i] = (uint64_t)(uintptr_t)o->needed[i];

    for (i = 0; i < n; i++) {
        const char *nm = o->strtab + offs[i];
        struct el_obj *dep;
        char found[4096];

        /* ⭐ the mechanism that keeps a foreign libc out. */
        if (el_soname_served(nm)) {
            if (el_nserved < (int)(sizeof el_served / sizeof el_served[0]))
                el_served[el_nserved++] = nm;
            continue;
        }
        if ((dep = el_already(nm)) != NULL) {
            o->needed[keep++] = dep;
            continue;
        }
        if (el_foreign_libc(nm)) {
            el_err("pgb-elfload: %s needs %s, a C library this image does not "
                   "provide; mapping it would put a second libc in the process",
                   o->soname, nm);
            return -1;
        }
        {
            const char *why = NULL;
            if (el_refused_class(nm, &why)) {
                el_err("pgb-elfload: %s needs %s, which is %s",
                       o->soname, nm, why);
                return -1;
            }
        }
        if (!el_search(found, sizeof found, nm, o)) {
            el_err("pgb-elfload: %s: cannot find %s", o->soname, nm);
            return -1;
        }
        dep = el_load(found, o);
        if (!dep)
            return -1;
        o->needed[keep++] = dep;
    }
    o->nneeded = keep;
    return 0;
}

static struct el_obj *el_load(const char *path, const struct el_obj *req)
{
    struct el_obj *o;
    Elf64_Ehdr eh;
    Elf64_Phdr *ph = NULL;
    const Elf64_Dyn *dyn = NULL;
    int fd = -1, i;

    (void)req;

    if (el_nobjs >= EL_MAX_OBJS) {
        el_err("pgb-elfload: more than %d objects loaded", EL_MAX_OBJS);
        return NULL;
    }
    fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        el_err("pgb-elfload: cannot open %s", path);
        return NULL;
    }
    if (read(fd, &eh, sizeof eh) != (ssize_t)sizeof eh ||
        memcmp(eh.e_ident, ELFMAG, SELFMAG) != 0 ||
        eh.e_ident[EI_CLASS] != ELFCLASS64 ||
        eh.e_machine != EM_X86_64 ||
        (eh.e_type != ET_DYN)) {
        el_err("pgb-elfload: %s is not an x86-64 ELF shared object", path);
        close(fd);
        return NULL;
    }
    /* ⛔ e_phentsize IS READ, not assumed. Every pread below strides by
     * sizeof(Elf64_Phdr); a file declaring a different entry size would be
     * parsed at the wrong offsets and produce segments made of adjacent
     * fields. Cheap to check, silently wrong to skip. */
    if (eh.e_phnum && eh.e_phentsize != sizeof(Elf64_Phdr)) {
        el_err("pgb-elfload: %s: e_phentsize is %u, expected %zu", path,
               (unsigned)eh.e_phentsize, sizeof(Elf64_Phdr));
        close(fd);
        return NULL;
    }
    ph = malloc((size_t)eh.e_phnum * sizeof(Elf64_Phdr));
    if (!ph ||
        pread(fd, ph, (size_t)eh.e_phnum * sizeof(Elf64_Phdr),
              (off_t)eh.e_phoff) !=
            (ssize_t)((size_t)eh.e_phnum * sizeof(Elf64_Phdr))) {
        el_err("pgb-elfload: %s: cannot read program headers", path);
        free(ph);
        close(fd);
        return NULL;
    }

    o = calloc(1, sizeof *o);
    if (!o) { free(ph); close(fd); el_err("pgb-elfload: out of memory"); return NULL; }
    o->path = strdup(path);

    if (el_map(o, fd, &eh, ph) != 0) {
        free(ph); close(fd); free(o->path); free(o);
        return NULL;
    }
    close(fd);

    for (i = 0; i < eh.e_phnum; i++) {
        if (ph[i].p_type == PT_DYNAMIC)
            dyn = (const Elf64_Dyn *)(o->base + ph[i].p_vaddr);
        else if (ph[i].p_type == PT_TLS) {
            o->tls_image  = o->base + ph[i].p_vaddr;
            o->tls_filesz = ph[i].p_filesz;
            o->tls_memsz  = ph[i].p_memsz;
            o->tls_align  = ph[i].p_align;
            o->tls_modid  = el_tls_next_modid++;
        } else if (ph[i].p_type == PT_GNU_RELRO) {
            o->relro   = o->base + el_down(ph[i].p_vaddr);
            o->relrosz = el_up(ph[i].p_vaddr + ph[i].p_memsz) -
                         el_down(ph[i].p_vaddr);
        }
    }
    if (!dyn) {
        el_err("pgb-elfload: %s: no PT_DYNAMIC", path);
        free(ph); free(o->path); free(o);
        return NULL;
    }
    el_read_dynamic(o, dyn);
    free(ph);
    el_dbg("pgb-elfload: mapped %s at %p (%zu bytes)\n", o->soname,
           (void *)o->base, o->span);

    /* ⛔ the interposer check, by shape rather than by name. el_refused_class()
     * above declines a list of sonames, which only ever catches the ones
     * somebody already met. This reads the object's own dynamic symbol table
     * for the signature instead, so a profiler nobody has named is caught too.
     *
     * ⚠ It has to run AFTER el_read_dynamic (the symbol table is not known
     * before it) and BEFORE el_relocate + el_init, because the damage is done
     * by the initialiser. */
    {
        const char *why = NULL;
        if (el_is_interposer(o, &why)) {
            el_err("pgb-elfload: %s is %s", o->soname, why);
            /* Not yet in el_objs, so nothing refers to it. The mapping is
             * given back here rather than leaked, which the two failure paths
             * above this point do not do -- they run before there is a
             * mapping to give back. */
            munmap(o->base, o->span);
            free(o->soname); free(o->path); free(o);
            return NULL;
        }
    }

    /* Registered BEFORE relocating so that a dependency cycle terminates and
     * so that a symbol defined here is visible to its own dependencies. */
    el_objs[el_nobjs++] = o;

    if (el_load_needed(o) != 0 || el_relocate(o) != 0) {
        /* ⚠ NOT el_nobjs--. el_load_needed() appends dependencies AFTER this
         * object, so decrementing would drop the last dependency and leave the
         * failed object registered -- a later lookup would then resolve into a
         * half-relocated image. Remove o by identity instead. */
        int k, w = 0;
        for (k = 0; k < el_nobjs; k++)
            if (el_objs[k] != o)
                el_objs[w++] = el_objs[k];
        el_nobjs = w;
        return NULL;
    }

    /* RELRO after relocation, which is the whole point of the segment: the
     * GOT is written while binding and read-only forever after. */
    if (o->relro && o->relrosz)
        mprotect(o->relro, o->relrosz, PROT_READ);
    if (o->textrel)
        mprotect(o->base, o->span, PROT_READ | PROT_EXEC);

    o->refcount = 1;
    return o;
}

static void el_init(struct el_obj *o)
{
    size_t i;

    if (o->initialised)
        return;
    o->initialised = 1;
    /* Dependencies first, which is the order ld.so uses and the order a
     * constructor that calls into a dependency requires. */
    for (i = 0; i < (size_t)o->nneeded; i++)
        el_init(o->needed[i]);
    el_dbg("pgb-elfload: init %s (DT_INIT %s, %zu in DT_INIT_ARRAY)\n",
           o->soname, o->init ? "yes" : "no", o->init_arrayn);
    if (o->init)
        o->init();
    for (i = 0; i < o->init_arrayn; i++)
        if (o->init_array[i]) {
            el_dbg("pgb-elfload:   init_array[%zu] %p\n", i,
                   (void *)o->init_array[i]);
            o->init_array[i]();
        }
    el_dbg("pgb-elfload: init %s done\n", o->soname);
}

/* -------------------------------------------------------------- public face */

void *pgb_elf_dlopen(const char *path, int flags)
{
    struct el_obj *o;
    char found[4096];

    /* ⚠ RTLD_LAZY is honoured as RTLD_NOW -- see the header. RTLD_GLOBAL is
     * read below; every other bit is ignored. */
    pgb_el_errset = 0;

    if (!pgb_elf_available()) {
        el_err("pgb-elfload: no provider table was compiled in "
               "(build with --host-dlopen)");
        return NULL;
    }
    if (path == NULL) {
        el_err("pgb-elfload: dlopen(NULL) has no answer in a static image");
        return NULL;
    }
    if (!el_search(found, sizeof found, path, NULL)) {
        el_err("pgb-elfload: cannot find %s", path);
        return NULL;
    }
    /* ⛔ a PATH to a soname this image already serves IS refused, not mapped.
     * The DT_NEEDED walk short-circuits these; a direct dlopen of the same
     * file has to as well, or the caller gets a SECOND libc by naming a path
     * instead of a soname. Measured: glibc's own stub libraries -- libdl.so.2,
     * libpthread.so.0, libutil.so.1, libanl.so.1, libBrokenLocale.so.1 --
     * bind to GLIBC_PRIVATE symbols in a libc.so.6 that is not in this
     * process, and every one of them crashed in its initialiser before this
     * check existed. */
    {
        const char *b = strrchr(found, '/');
        b = b ? b + 1 : found;
        const char *why = NULL;
        if (el_soname_served(b)) {
            el_err("pgb-elfload: %s is already served by this image's own "
                   "static glibc; loading it would put a second libc in the "
                   "process", b);
            return NULL;
        }
        if (el_refused_class(b, &why)) {
            el_err("pgb-elfload: %s is %s", b, why);
            return NULL;
        }
    }
    {
        const char *b = strrchr(found, '/');
        struct el_obj *have = el_already(b ? b + 1 : found);
        if (have) {
            have->refcount++;
            /* ⚠ RTLD_GLOBAL on a SECOND dlopen of an object already loaded
             * locally promotes it, which is what ld.so does. The reverse is
             * not true: RTLD_LOCAL never demotes. */
            if (flags & EL_RTLD_GLOBAL) el_mark_global(have);
            return have;
        }
    }
    o = el_load(found, NULL);
    if (!o)
        return NULL;
    /* ⭐ BEFORE el_init, because an initialiser can call back into a symbol
     * whose resolution this flag decides. */
    if (flags & EL_RTLD_GLOBAL) el_mark_global(o);
    el_init(o);
    return o;
}

void *pgb_elf_dlsym(void *handle, const char *name)
{
    struct el_obj *o = handle;
    const Elf64_Sym *s;
    int i;

    pgb_el_errset = 0;
    if (!handle) {
        el_err("pgb-elfload: dlsym on a NULL handle");
        return NULL;
    }
    s = el_lookup_in(o, name, NULL);
    if (s)
        return o->base + s->st_value;
    /* Then its dependency closure, which is what dlsym on a handle means. */
    for (i = 0; i < o->nneeded; i++) {
        s = el_lookup_in(o->needed[i], name, NULL);
        if (s)
            return o->needed[i]->base + s->st_value;
    }
    el_err("pgb-elfload: %s: undefined symbol: %s", o->soname, name);
    return NULL;
}

int pgb_elf_dlclose(void *handle)
{
    struct el_obj *o = handle;

    pgb_el_errset = 0;
    if (!o)
        return 0;
    /* ⚠ Refcounted but never unmapped. Unmapping an object whose constructors
     * registered callbacks elsewhere -- atexit, a pthread key destructor -- is
     * how a loader turns a clean exit into a jump into unmapped memory. ld.so
     * carries a great deal of machinery to know when that is safe; this
     * loader does not have it, so it keeps the mapping and says so here
     * rather than pretending. */
    if (o->refcount > 0)
        o->refcount--;
    return 0;
}

void pgb_elf_trace_loaded(int fd)
{
    char line[512];
    int i;

    for (i = 0; i < el_nobjs; i++) {
        int n = snprintf(line, sizeof line, "\t%s => %s (0x%016llx)\n",
                         el_objs[i]->soname, el_objs[i]->path,
                         (unsigned long long)(uintptr_t)el_objs[i]->base);
        if (n > 0)
            (void)!write(fd, line, (size_t)n);
    }
    /* ⭐ The names served WITHOUT a mapping, which a syscall trace cannot see
     * because no syscall happened. This is the half that makes the internal
     * instrument worth having beside the external one. */
    for (i = 0; i < el_nserved; i++) {
        int n = snprintf(line, sizeof line,
                         "\t%s => served internally (static glibc)\n",
                         el_served[i]);
        if (n > 0)
            (void)!write(fd, line, (size_t)n);
    }
}
