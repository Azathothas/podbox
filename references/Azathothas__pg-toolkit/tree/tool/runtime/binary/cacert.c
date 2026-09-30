/* binary/cacert.c -- give a static binary a TLS trust store on a host whose
 * store is not where the binary was told to look.
 *
 * The problem, measured in poc/30-curl AND docs/limitations.md section 3
 * -------------------------------------------------------------------------
 * A TLS library compiles in exactly ONE default bundle path, chosen when IT
 * was built. Every distribution puts the bundle somewhere else. Measured
 * across the eleven pinned environments, with the apparatus's own CA variables
 * unset, on a curl built against OpenSSL in the pinned Debian environment:
 *
 *     verified     5 of 11
 *     unverified   Debian 11, Debian 12, Ubuntu 20.04 (no bundle at all in
 *                  the minimal image); Rocky 8 (/etc/pki/tls/certs/ca-bundle.crt);
 *                  openSUSE Leap (/etc/ssl/ca-bundle.pem);
 *                  Alpine 3.10 (/etc/ssl/cert.pem)
 *
 * ⚠ this IS a fourth, separate dependency, and it is not glibc's. Fixing NSS,
 * iconv and locale touches none of it. It is DATA reached by PATH, which is
 * why static linking is silent about it.
 *
 * ⛔ and it IS the one where failing open would be a security bug. A shim
 * that "helpfully" disabled verification, or that trusted a bundle it could
 * not find, would turn a connection error into a silent downgrade. Nothing
 * here ever disables anything: it only ever tells the TLS library WHERE a
 * store is, and if it cannot find one it changes nothing and lets the
 * program's own error stand.
 *
 * The fix, in two layers, and the first one IS the important one
 * -------------------------------------------------------------------------
 * 1. ⭐ find the host's store. Six of the eleven failures above are not
 *    "this machine has no certificates", they are "this machine keeps them
 *    somewhere else". Probing the known locations costs nothing, writes
 *    nothing, embeds nothing, and -- crucially -- keeps the host's own trust
 *    decisions, including any root an administrator added or removed.
 *
 * 2. Materialise an embedded copy, ONLY where layer 1 found nothing. That is
 *    the three minimal Debian/Ubuntu images, which genuinely ship no bundle.
 *
 * ⛔ the embedded copy ages and that IS a real hazard, stated here rather
 * than discovered later. It is a snapshot of the pinned build environment's
 * ca-certificates package taken at build time. Roots get revoked and expire.
 * A binary carrying a two-year-old bundle, running on a host that HAS a
 * current one, must not use the stale copy -- which is exactly why layer 1
 * runs first and layer 2 is a fallback, never a pre-emption. Same rule as
 * binary/locale.c, for a stronger reason.
 *
 * ⭐ why a constructor and not a --wrap. There is no single function to wrap:
 * OpenSSL reads SSL_CERT_FILE inside SSL_CTX_set_default_verify_paths(),
 * curl reads CURL_CA_BUNDLE in its own initialisation, GnuTLS has its own
 * path, and an application may call any of them or none. The environment is
 * the one interface all of them already agree on, so this sets the variables
 * before main() and lets each library find them the way it already does.
 *
 * ⛔ what it will not do:
 *   - it never OVERWRITES a variable the caller set. If SSL_CERT_FILE is
 *     already in the environment, this file does nothing at all;
 *   - it never sets a path that does not exist;
 *   - it never touches the filesystem unless the host has no store AND the
 *     binary was built with --embed-cacert;
 *   - it does not help a TLS library that ignores the environment. That is a
 *     real gap and it is not papered over.
 *
 * SPDX-License-Identifier: 0BSD
 */

#define _GNU_SOURCE
#include <fcntl.h>
#include <stdio.h>      /* ⛔ snprintf. Missing here until 2026-09-01d, which
                         * made every --embed-cacert build print
                         *   warning: implicit declaration of function
                         *   'snprintf' [-Wimplicit-function-declaration]
                         * and compile a call whose return type C only assumes.
                         * ⚠ binary/locale.c carries the same include and the
                         * same note. Omitting it is a warning under gcc 12 and
                         * an ERROR under C23, so it is a build that stops
                         * working on a newer compiler rather than one that is
                         * merely untidy.
                         */
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

/* Provided by the generated pgb-cacert-data.c when --embed-cacert is given.
 *
 * ⛔ DECLARED, never defined here, and binary/locale.c carries the whole story:
 * a weak const definition that the same translation unit reads is
 * constant-folded, the strong definition then binds to a symbol no code is
 * reading, and the flag produces a larger binary that behaves as if it had
 * not been passed. The link supplies these or the object is not linked. */
extern const unsigned char pgb_cacert_data[];
extern const unsigned pgb_cacert_len;

/* The known locations, in the order ld.so-style discovery would take them:
 * the two most common first, then per-distribution ones. Every one of these
 * is a path actually observed in the pinned matrix or on a distribution the
 * matrix represents. */
static const char *const pgb_cacert_files[] = {
    "/etc/ssl/certs/ca-certificates.crt",   /* Debian, Ubuntu, Alpine, Arch, Void, Gentoo */
    "/etc/pki/tls/certs/ca-bundle.crt",     /* Fedora, RHEL, Rocky, CentOS */
    "/etc/ssl/ca-bundle.pem",               /* openSUSE, SLES */
    "/etc/ssl/cert.pem",                    /* Alpine <= 3.10, OpenBSD-ish layouts */
    "/etc/pki/tls/cacert.pem",              /* older RHEL */
    "/etc/ssl/certs/ca-bundle.crt",         /* some Arch derivatives */
    "/usr/share/ssl/certs/ca-bundle.crt",   /* very old RHEL */
    "/usr/local/share/certs/ca-root-nss.crt",
    "/etc/ca-certificates/extracted/tls-ca-bundle.pem", /* Arch's extracted tree */
    NULL
};

/* A hashed directory of individual certificates. OpenSSL will use this
 * through SSL_CERT_DIR even where no single concatenated file exists. */
static const char *const pgb_cacert_dirs[] = {
    "/etc/ssl/certs",
    "/etc/pki/tls/certs",
    "/system/etc/security/cacerts",
    NULL
};

static int pgb_is_file(const char *p)
{
    struct stat st;
    /* ⛔ st_size, not just existence. Debian's ca-certificates.crt is created
     * empty by the package before update-ca-certificates runs, and an empty
     * bundle is a store that trusts nothing -- which fails closed, but with a
     * verification error rather than a "no store" error, and sends whoever
     * reads it looking in the wrong place. */
    return stat(p, &st) == 0 && S_ISREG(st.st_mode) && st.st_size > 0;
}

static int pgb_is_dir(const char *p)
{
    struct stat st;
    return stat(p, &st) == 0 && S_ISDIR(st.st_mode);
}

/* Write the embedded bundle somewhere the process can read it back.
 * Returns a static buffer holding the path, or NULL. */
static const char *pgb_cacert_materialise(void)
{
    static char path[256];
    const char *tmp;
    int fd;
    size_t off = 0;

    if (pgb_cacert_len == 0)
        return NULL;

    tmp = getenv("TMPDIR");
    if (!tmp || tmp[0] != '/')
        tmp = "/tmp";

    /* ⚠ The pid keeps two concurrent runs apart. This is not a security
     * boundary and is not claimed to be one: the file is created O_EXCL with
     * mode 0600, so a pre-existing path is a failure rather than something to
     * be overwritten or followed. */
    if (snprintf(path, sizeof path, "%s/pgb-cacert-%ld.pem",
                 tmp, (long)getpid()) >= (int)sizeof path)
        return NULL;

    fd = open(path, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600);
    if (fd < 0) {
        /* A leftover from a recycled pid, or an unwritable TMPDIR. Reuse only
         * if it is already exactly what we would have written. */
        if (pgb_is_file(path)) {
            struct stat st;
            if (stat(path, &st) == 0 && (size_t)st.st_size == pgb_cacert_len)
                return path;
        }
        return NULL;
    }

    while (off < pgb_cacert_len) {
        ssize_t n = write(fd, pgb_cacert_data + off, pgb_cacert_len - off);
        if (n <= 0) {
            close(fd);
            unlink(path);
            return NULL;
        }
        off += (size_t)n;
    }
    close(fd);
    return path;
}

/* ⭐ the anchor. A constructor that nothing references is dropped from an
 * archive, and a trust-store fix that silently did not link is precisely the
 * failure this file exists to prevent. internal/wrapper/flags.go forces it with
 * -Wl,-u,pgb_cacert_anchor, the same technique binary/nssfix.c uses. */
const char pgb_cacert_anchor[] = "pgb-cacert";

__attribute__((constructor)) static void pgb_cacert_init(void)
{
    const char *found = NULL;
    const char *dir = NULL;
    int i;

    /* ⛔ never override the caller. If the operator has already said where the
     * trust store is, that is the answer, and second-guessing it would be the
     * one behaviour a security-relevant shim must not have. */
    if (getenv("SSL_CERT_FILE") || getenv("CURL_CA_BUNDLE"))
        return;

    for (i = 0; pgb_cacert_files[i]; i++) {
        if (pgb_is_file(pgb_cacert_files[i])) {
            found = pgb_cacert_files[i];
            break;
        }
    }

    /* ⚠ A hashed directory is a real store and is reported separately: some
     * hosts have one and no concatenated file. Only set SSL_CERT_DIR when the
     * caller has not. */
    if (!getenv("SSL_CERT_DIR")) {
        for (i = 0; pgb_cacert_dirs[i]; i++) {
            if (pgb_is_dir(pgb_cacert_dirs[i])) {
                dir = pgb_cacert_dirs[i];
                break;
            }
        }
    }

    if (!found)
        found = pgb_cacert_materialise();   /* layer 2, and only here */

    if (found) {
        setenv("SSL_CERT_FILE", found, 0);
        setenv("CURL_CA_BUNDLE", found, 0);
    }
    if (dir)
        setenv("SSL_CERT_DIR", dir, 0);

    /* ⛔ no else branch on purpose. When nothing is found the program's own
     * behaviour stands, which means its own verification error. Anything this
     * file could do instead would be a downgrade. */
}
