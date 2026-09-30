/* binary/terminfo.c -- give a static binary a terminal description on a host
 * that has no terminfo database, or has one without the entry it needs.
 *
 * The problem, measured in poc/20-nano AND docs/limitations.md section 3
 * -------------------------------------------------------------------------
 * ncurses reaches terminal descriptions by PATH, not by code. A `setupterm()`
 * probe linked against the same static ncursesw, across the eleven pinned
 * environments:
 *
 *     OK                                    all 7 glibc
 *     rc=-1 err=-1  no database at all      Alpine 3.10, 3.20, 3.22
 *     rc=-1 err=0   database present, no
 *                   xterm-256color entry    Void musl
 *
 * ⚠ the two failures are different and the second IS the interesting one.
 * "No database" is the obvious case. "A database that does not describe THIS
 * terminal" is the one an embedded copy has to handle too, and a mechanism
 * that only checked for the directory would miss it.
 *
 * ⛔ whether a glibc portability tool should own a terminal database IS a
 * real question and the answer here IS "only if asked". docs/limitations.md
 * section 3 says the argument is weak, and it is: terminfo is ncurses' data, not
 * libc's. So this is opt-in (`--embed-terminfo`), it is off by default, and
 * it embeds a HANDFUL of entries rather than a database -- enough that a
 * terminal program is usable, not enough to pretend to be /usr/share/terminfo.
 *
 * The mechanism, and why it IS a constructor
 * -------------------------------------------------------------------------
 * ncurses resolves a terminal in this order: $TERMINFO (a single directory),
 * $TERMINFO_DIRS (a search list), $HOME/.terminfo, then its compiled-in
 * default. So pointing it at a directory is the whole interface, exactly as
 * LOCPATH is for glibc locales and SSL_CERT_FILE is for OpenSSL.
 *
 * ⭐ and the host comes first, for the same reason as binary/cacert.c: a host
 * that has a real database has a BETTER one than five embedded entries, and
 * a description an administrator installed must win. This only acts when the
 * host cannot answer for $TERM.
 *
 * ⛔ it writes to the filesystem, which is why it is opt-in. Same tier as
 * --embed-locale and for the same reason.
 *
 * SPDX-License-Identifier: 0BSD
 */

#define _GNU_SOURCE
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

struct pgb_ti_file { const char *name; const unsigned char *data; unsigned len; };

/* Supplied by the generated pgb-terminfo-data.c. ⛔ Declared, never defined
 * here: binary/locale.c records what a weak const definition read by its own
 * translation unit does, and it is not what anyone expects. */
extern const struct pgb_ti_file pgb_ti_files[];
extern const unsigned pgb_ti_nfiles;

const char pgb_terminfo_anchor[] = "pgb-terminfo";

/* ncurses stores an entry as <dir>/<first letter>/<name>, and on some builds
 * as <dir>/<two hex digits of the first byte>/<name>. The letter form is what
 * every Linux ncurses reads, and it is what `tic` writes by default. */
static int pgb_ti_host_has(const char *term)
{
    static const char *const roots[] = {
        "/usr/share/terminfo", "/lib/terminfo", "/usr/lib/terminfo",
        "/etc/terminfo", "/usr/share/lib/terminfo", NULL
    };
    char path[512];
    struct stat st;
    int i;

    if (!term || !*term)
        return 0;

    /* An explicit TERMINFO the caller set is the caller's business. */
    if (getenv("TERMINFO") || getenv("TERMINFO_DIRS"))
        return 1;

    for (i = 0; roots[i]; i++) {
        if (snprintf(path, sizeof path, "%s/%c/%s", roots[i], term[0], term)
            >= (int)sizeof path)
            continue;
        if (stat(path, &st) == 0 && S_ISREG(st.st_mode) && st.st_size > 0)
            return 1;
        if (snprintf(path, sizeof path, "%s/%02x/%s",
                     roots[i], (unsigned char)term[0], term) >= (int)sizeof path)
            continue;
        if (stat(path, &st) == 0 && S_ISREG(st.st_mode) && st.st_size > 0)
            return 1;
    }
    return 0;
}

static int pgb_ti_write(const char *dir, const struct pgb_ti_file *f)
{
    char path[512];
    char sub[512];
    int fd;
    size_t off = 0;

    if (snprintf(sub, sizeof sub, "%s/%c", dir, f->name[0]) >= (int)sizeof sub)
        return -1;
    if (mkdir(sub, 0700) != 0 && access(sub, X_OK) != 0)
        return -1;
    if (snprintf(path, sizeof path, "%s/%s", sub, f->name) >= (int)sizeof path)
        return -1;

    fd = open(path, O_WRONLY | O_CREAT | O_TRUNC | O_NOFOLLOW | O_CLOEXEC, 0600);
    if (fd < 0)
        return -1;
    while (off < f->len) {
        ssize_t n = write(fd, f->data + off, f->len - off);
        if (n <= 0) { close(fd); return -1; }
        off += (size_t)n;
    }
    close(fd);
    return 0;
}

__attribute__((constructor)) static void pgb_terminfo_init(void)
{
    const char *term = getenv("TERM");
    char dir[256];
    unsigned i;

    if (pgb_ti_nfiles == 0)
        return;
    /* ⛔ A program with no TERM is not a terminal program, and writing a
     * database for it would be pure cost. Also the case for `TERM=dumb`,
     * which ncurses answers from a builtin. */
    if (!term || !*term || strcmp(term, "dumb") == 0)
        return;
    /* ⭐ The host first, always. Five embedded entries are not better than a
     * real database, and a description an administrator installed must win. */
    if (pgb_ti_host_has(term))
        return;

    {
        const char *tmp = getenv("TMPDIR");
        if (!tmp || tmp[0] != '/') tmp = "/tmp";
        if (snprintf(dir, sizeof dir, "%s/pgb-terminfo-%ld",
                     tmp, (long)getpid()) >= (int)sizeof dir)
            return;
    }
    if (mkdir(dir, 0700) != 0 && access(dir, X_OK) != 0)
        return;

    for (i = 0; i < pgb_ti_nfiles; i++)
        (void)pgb_ti_write(dir, &pgb_ti_files[i]);

    /* ⚠ TERMINFO, not TERMINFO_DIRS: ncurses searches TERMINFO first and
     * falls back to its compiled-in path afterwards anyway, so a host entry
     * this binary does not carry is still reachable. setenv's overwrite flag
     * is 0 -- the caller's value is never replaced, and pgb_ti_host_has()
     * has already returned early if the caller set one. */
    setenv("TERMINFO", dir, 0);
}
