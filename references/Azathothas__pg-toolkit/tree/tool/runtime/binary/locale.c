/* binary/locale.c -- give a static glibc binary a UTF-8 locale on a host that
 * has no glibc locale data.
 *
 * The problem, measured in experiments/30-gconv-and-locale.sh
 * -------------------------------------------------------------------------
 * glibc's C.UTF-8 is NOT compiled into libc. It is a directory of twelve
 * files under /usr/lib/locale, and setlocale() mmaps them. On all four musl
 * environments in the matrix there is no such directory, so:
 *
 *     setlocale(LC_ALL, "C.UTF-8")  ->  NULL
 *     nl_langinfo(CODESET)          ->  "ANSI_X3.4-1968"
 *
 * The process keeps the C locale and reports an ASCII charset. Programs that
 * ask the locale how to interpret bytes -- anything using mbrtowc, wide
 * curses, or a toolkit that checks nl_langinfo -- then treat UTF-8 input as
 * single-byte, which is silent corruption rather than a failure.
 *
 * ⚠ this IS a third, separate dependency. Fixing NSS does not touch it and
 * fixing iconv does not touch it. It is data, not code, and unlike the other
 * two it cannot be linked in as an archive: glibc reaches it by path.
 *
 * The fix, and its honest cost
 * -------------------------------------------------------------------------
 * The C.utf8 tree (408 KiB, nearly all of it LC_CTYPE) is embedded in the
 * binary by the driver's --embed-locale, and materialised to a directory only
 * if the program actually asks for a locale the host cannot provide. LOCPATH
 * then points glibc at it.
 *
 * ⛔ this IS the one mechanism in the tool that touches the filesystem at
 * runtime, and it is off by default for that reason. It is tier 3 of the
 * project's hierarchy (a generic runtime technique) where NSS and iconv are
 * tier 2 (a toolchain change), so it is opt-in and says so.
 *
 * ⭐ WHY --wrap=setlocale and not a constructor. A constructor would have to
 * extract on every run of every program, including the overwhelming majority
 * that never call setlocale at all. Wrapping means the cost is paid at the
 * moment a program asks for a locale, by the programs that ask, and a program
 * that does not call setlocale writes nothing and touches no directory.
 * A program whose host already HAS the locale also writes nothing: the real
 * setlocale is tried first and the embedded copy is a fallback, never a
 * pre-emption.
 *
 * SPDX-License-Identifier: 0BSD
 */

#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <locale.h>
#include <stdio.h>      /* snprintf: implicitly declared here once, which is UB */
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

struct pgb_locale_file { const char *name; const unsigned char *data; unsigned len; };

/* ⛔ Declared here and never defined here, and it must stay that way.
 *
 * A weak definition in this file, so the object links without a generated data
 * file, does not work: the strong definition in the generated file never wins.
 *
 * A weak CONST definition that the same translation unit also READS is
 * constant-folded: GCC compiled `pgb_locale_nfiles` as the literal 0 it could
 * see, so pgb_materialise() returned -1 before looking at anything and
 * `nm locale.o` showed `V pgb_locale_nfiles` with no undefined reference
 * to it anywhere. The link then bound the strong definition to a symbol no
 * code was reading. --embed-locale produced a binary 361 KiB larger carrying
 * the locale data, and behaved exactly as if the flag had not been passed.
 *
 * Reproduce the old defect:  nm locale.o | grep pgb_locale_nfiles
 *   'V' (weak) with no 'U' reference elsewhere means it was folded.
 *   'U' means this file reads it at run time, which is what it must do.
 *
 * The driver links locale.o only together with the generated
 * locale-data.o, so there is no configuration in which these are
 * undefined and no reason for a fallback definition to exist.
 */
extern const struct pgb_locale_file pgb_locale_files[];
extern const unsigned pgb_locale_nfiles;
extern const char pgb_locale_name[];

extern char *__real_setlocale(int category, const char *locale);

static int pgb_locale_ready;      /* 0 untried, 1 materialised, -1 gave up */

static int pgb_mkdir_p(const char *path)
{
    char buf[4096];
    size_t n = strlen(path);
    if (n >= sizeof buf) return -1;
    memcpy(buf, path, n + 1);
    for (char *p = buf + 1; *p; p++) {
        if (*p != '/') continue;
        *p = '\0';
        if (mkdir(buf, 0700) != 0 && errno != EEXIST) return -1;
        *p = '/';
    }
    return (mkdir(buf, 0700) != 0 && errno != EEXIST) ? -1 : 0;
}

/* Write the embedded tree under a writable directory and point LOCPATH at it.
 * Returns 0 on success.
 *
 * ⚠ CONCURRENCY. Two processes from the same binary can race here. Each file
 * is written to a pid-unique temporary name and rename()d into place, which is
 * atomic on every filesystem this can land on, so a reader either sees the
 * old absent file or the complete new one, never a half-written LC_CTYPE. The
 * content is identical either way, so the loser of a race costs one wasted
 * write and nothing else. */
static int pgb_materialise(void)
{
    if (pgb_locale_nfiles == 0) return -1;

    const char *base = getenv("XDG_RUNTIME_DIR");
    if (!base || !*base) base = getenv("TMPDIR");
    if (!base || !*base) base = "/tmp";

    /* The directory name carries the total byte count so a future build with
     * different locale data cannot silently reuse a stale extraction. */
    unsigned total = 0;
    for (unsigned i = 0; i < pgb_locale_nfiles; i++) total += pgb_locale_files[i].len;

    static char root[4096], dir[4096];
    if (snprintf(root, sizeof root, "%s/.pgb-locale-%u", base, total) >= (int) sizeof root)
        return -1;
    if (snprintf(dir, sizeof dir, "%s/%s", root, pgb_locale_name) >= (int) sizeof dir)
        return -1;
    if (pgb_mkdir_p(dir) != 0) return -1;

    for (unsigned i = 0; i < pgb_locale_nfiles; i++) {
        char final[4096], tmp[4096];
        if (snprintf(final, sizeof final, "%s/%s", dir, pgb_locale_files[i].name) >= (int) sizeof final)
            return -1;

        /* ⛔ a GLIBC locale IS a tree, not a flat directory. LC_MESSAGES is
         * itself a directory holding SYS_LC_MESSAGES, so an entry name can
         * carry a separator and its parent has to exist first.
         *
         * ⛔ Embedding only the regular files at the top level and skipping
         * LC_MESSAGES fails silently. glibc fails to load that one category,
         * which fails the whole LC_ALL composite, and the process keeps the C
         * locale without saying so. ⚠ It looks like it works on Debian,
         * because glibc falls through to the host's own /usr/lib/locale/C.utf8
         * for the missing piece -- borrowing from exactly the host directory
         * this mechanism exists to stop depending on. On Alpine, where there
         * is nothing to borrow, the codeset stayed ANSI_X3.4-1968. */
        char *slash = strrchr(final, '/');
        if (slash && slash > final) {
            *slash = '\0';
            if (pgb_mkdir_p(final) != 0) return -1;
            *slash = '/';
        }

        struct stat st;
        if (stat(final, &st) == 0 && st.st_size == (off_t) pgb_locale_files[i].len)
            continue;                                   /* already there */
        if (snprintf(tmp, sizeof tmp, "%s.%ld", final, (long) getpid()) >= (int) sizeof tmp)
            return -1;
        int fd = open(tmp, O_WRONLY | O_CREAT | O_TRUNC, 0600);
        if (fd < 0) return -1;
        const unsigned char *p = pgb_locale_files[i].data;
        unsigned left = pgb_locale_files[i].len;
        while (left) {
            ssize_t w = write(fd, p, left);
            if (w < 0) { if (errno == EINTR) continue; close(fd); unlink(tmp); return -1; }
            p += w; left -= (unsigned) w;
        }
        if (close(fd) != 0) { unlink(tmp); return -1; }
        if (rename(tmp, final) != 0) { unlink(tmp); return -1; }
    }

    setenv("LOCPATH", root, 1);
    return 0;
}

/* pgb_utf8_default is set by the generated data object when the build asked
 * for --utf8-default, so this file needs no compile-time define and the same
 * object serves both flags. */
extern const int pgb_utf8_default;

/* Does this locale name ask for UTF-8? "" means "read it out of the
 * environment", so that case looks at the same variables glibc would. */
static int pgb_wants_utf8(const char *locale)
{
    const char *s = locale;
    if (s && !*s) {
        s = getenv("LC_ALL");
        if (!s || !*s) s = getenv("LC_CTYPE");
        if (!s || !*s) s = getenv("LANG");
    }
    if (!s || !*s) return 0;
    return strstr(s, "UTF-8") != NULL || strstr(s, "utf8") != NULL ||
           strstr(s, "UTF8")  != NULL || strstr(s, "utf-8") != NULL;
}

/* Is the environment SILENT about the locale? "" means "read it out of the
 * environment"; if none of the three variables says anything, the answer is
 * the implementation's choice and glibc chooses "C". */
static int pgb_env_silent(void)
{
    const char *s = getenv("LC_ALL");
    if (s && *s) return 0;
    s = getenv("LC_CTYPE");
    if (s && *s) return 0;
    s = getenv("LANG");
    return !(s && *s);
}

/* ⭐ the one axis where native musl beats both GLIBC columns, 11-0.
 *
 * `experiments/63-` measured the three-way parity matrix and found exactly one
 * row against us: with NO locale variable set at all, `setlocale(LC_ALL, "")`
 * followed by `nl_langinfo(CODESET)` answers
 *
 *     glibc   ANSI_X3.4-1968      musl   UTF-8
 *
 * on eleven of eleven. ⛔ `--embed-locale` alone cannot move it, and the
 * reason is precise rather than incidental: that mechanism answers a REQUEST
 * the host could not satisfy, and here the host satisfied the request. glibc
 * returned "C" successfully. Nothing failed.
 *
 * ⚠ so this IS a separate, separately opt-in change, and it is a change to a
 * Documented default rather than a repair: POSIX leaves the implementation to
 * choose when the environment is silent, glibc chooses "C", and this chooses
 * "C.UTF-8" -- which is what musl does unconditionally and what several
 * distributions now do by policy. A program that assumed a single-byte
 * default sees a multibyte one. It is off unless `--utf8-default` asks for it,
 * and `pg-toolkit binary explain` prints it.
 *
 * ⛔ it still never overrides an answer the environment gave. If LC_ALL,
 * LC_CTYPE or LANG names anything at all, this does nothing. */
static int pgb_utf8_default_wanted(const char *locale)
{
    if (!pgb_utf8_default) return 0;
    if (!locale || *locale) return 0;   /* only setlocale(cat, "") */
    return pgb_env_silent();
}

char *__wrap_setlocale(int category, const char *locale)
{
    char *r = __real_setlocale(category, locale);

    if (r != NULL && pgb_utf8_default_wanted(locale)) {
        char *u = __real_setlocale(category, "C.UTF-8");
        if (u) return u;
        if (pgb_locale_ready == 0)
            pgb_locale_ready = (pgb_materialise() == 0) ? 1 : -1;
        if (pgb_locale_ready == 1) {
            u = __real_setlocale(category, "C.UTF-8");
            if (u) return u;
            u = __real_setlocale(category, pgb_locale_name);
            if (u) return u;
        }
        /* ⚠ Nothing worked: put back what the caller would have got. A
         * half-applied locale change is worse than none. */
        return __real_setlocale(category, locale);
    }

    /* ⛔ the host wins when the host can answer. Only a genuine failure on a
     * request that wanted UTF-8 reaches the embedded copy. Pre-empting a
     * working host locale would replace, say, a full en_GB.UTF-8 with a
     * C.UTF-8 that has different collation -- a regression introduced by a
     * portability tool, which is the worst kind. */
    if (r != NULL) return r;
    if (pgb_locale_ready < 0) return r;
    if (!pgb_wants_utf8(locale)) return r;

    if (pgb_locale_ready == 0)
        pgb_locale_ready = (pgb_materialise() == 0) ? 1 : -1;
    if (pgb_locale_ready != 1) return r;

    r = __real_setlocale(category, locale);
    if (r) return r;
    r = __real_setlocale(category, "C.UTF-8");
    if (r) return r;
    return __real_setlocale(category, pgb_locale_name);
}
