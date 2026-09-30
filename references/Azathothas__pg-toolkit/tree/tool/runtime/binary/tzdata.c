// SPDX-License-Identifier: 0BSD
/*
 * binary/tzdata.c - carry a handful of timezone descriptions, for hosts with none.
 *
 * -- the problem, MEASURED --------------------------------------------------
 *
 * `experiments/97-timezone.sh`: static glibc carries NO timezone data. libc.a
 * names /usr/share/zoneinfo, /etc/localtime and honours TZDIR, and reads all
 * three from the HOST. Four of this project's eleven target environments ship
 * no zone database at all -- alpine 3.10, 3.20, 3.22 and ubuntu-20.04, which
 * is glibc, so this is not a musl story.
 *
 * ⛔ and the failure IS silent and worse than "it returns UTC". With no
 * database glibc re-reads TZ=Europe/Berlin as a POSIX zone SPECIFICATION -- a
 * bare abbreviation with no offset -- and strftime prints
 *
 *     Europe +0000
 *
 * the zone name the caller asked for, with a UTC offset. %Z, the field that
 * looks like a confirmation, is an echo of the input; only the offset carries
 * the defect. That is the same class as gconv: a wrong answer at the point of
 * USE, long after any startup check has passed.
 *
 * -- what this does ---------------------------------------------------------
 *
 * The same shape as binary/terminfo.c, because glibc honours TZDIR exactly as
 * ncurses honours TERMINFO:
 *
 *   1. a caller who set TZDIR is left alone -- that is their business;
 *   2. ⭐ the host first, ALWAYS. A real zone database is better than a
 *      handful of embedded files, and an administrator's tzdata must win;
 *   3. otherwise the embedded files are written under $TMPDIR and TZDIR is
 *      pointed at them, with setenv's overwrite flag 0 so a caller's value is
 *      never replaced.
 *
 * ⚠ it IS a handful, not a database, and the binary says so. tzdata is ~1,800
 * files; carrying all of them would multiply a 2 MB static binary. The set is
 * `cfg.DefaultTzdataZones`, overridable with PGT_BINARY_TZDATA_ZONES at build time.
 * A zone that is not carried behaves exactly as it does today, which is the
 * honest floor: this closes the case it carries and no other.
 *
 * ⚠ a zone name has a directory in it -- "Europe/Berlin", "America/New_York".
 * Unlike terminfo's single letter subdirectory this needs the leading path
 * components created, and the writer below creates exactly the ones the names
 * require rather than assuming one level: "America/Indiana/Indianapolis" is a
 * real zone with two.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>

struct pgb_tz_file { const char *name; const unsigned char *data; unsigned len; };

extern const struct pgb_tz_file pgb_tz_files[];
extern const unsigned pgb_tz_nfiles;

/* The link pulls this object in by name; see wrapper/flags.go. */
int pgb_tzdata_anchor;

/* zoneinfoRoots' runtime half: if any of these is a directory the host has a
 * database, and the host's database wins. */
static int pgb_tz_host_has(void)
{
    static const char *const roots[] = {
        "/usr/share/zoneinfo", "/usr/lib/zoneinfo", "/etc/zoneinfo", 0
    };
    struct stat st;
    unsigned i;

    for (i = 0; roots[i]; i++)
        if (stat(roots[i], &st) == 0 && S_ISDIR(st.st_mode))
            return 1;
    return 0;
}

/* mkdir every leading component of `rel` under `dir`. */
static int pgb_tz_mkparents(const char *dir, const char *rel)
{
    char path[512];
    size_t base;
    const char *p;

    if ((base = strlen(dir)) + 1 >= sizeof path)
        return -1;
    memcpy(path, dir, base);
    path[base] = '/';
    base++;

    for (p = rel; *p; p++) {
        if (*p != '/')
            continue;
        if (base + (size_t)(p - rel) >= sizeof path)
            return -1;
        memcpy(path + base, rel, (size_t)(p - rel));
        path[base + (size_t)(p - rel)] = '\0';
        if (mkdir(path, 0700) != 0 && access(path, X_OK) != 0)
            return -1;
    }
    return 0;
}

static int pgb_tz_write(const char *dir, const struct pgb_tz_file *f)
{
    char path[512];
    int fd;
    size_t off = 0;

    if (pgb_tz_mkparents(dir, f->name) != 0)
        return -1;
    if ((size_t)snprintf(path, sizeof path, "%s/%s", dir, f->name) >= sizeof path)
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

__attribute__((constructor)) static void pgb_tzdata_init(void)
{
    char dir[256];
    unsigned i;

    if (pgb_tz_nfiles == 0)
        return;
    /* The caller's TZDIR is the caller's business. */
    if (getenv("TZDIR"))
        return;
    /* ⭐ The host first, always. */
    if (pgb_tz_host_has())
        return;

    {
        const char *tmp = getenv("TMPDIR");
        if (!tmp || tmp[0] != '/') tmp = "/tmp";
        if ((size_t)snprintf(dir, sizeof dir, "%s/pgb-tzdata-%ld",
                             tmp, (long)getpid()) >= sizeof dir)
            return;
    }
    if (mkdir(dir, 0700) != 0 && access(dir, X_OK) != 0)
        return;

    for (i = 0; i < pgb_tz_nfiles; i++)
        (void)pgb_tz_write(dir, &pgb_tz_files[i]);

    /* ⚠ overwrite flag 0: a caller's TZDIR is never replaced, and the early
     * return above has already handled the case where they set one. */
    setenv("TZDIR", dir, 0);
}
