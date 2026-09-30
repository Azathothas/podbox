/* binary/nssfix.c -- stop a static glibc binary loading the host's NSS modules.
 *
 * The problem, measured in experiments/20-static-glibc-nss-dlopen.sh
 * -------------------------------------------------------------------------
 * A `gcc -static` glibc binary still reads the HOST's /etc/nsswitch.conf at
 * runtime and dlopen()s whatever it names. Those modules are ordinary shared
 * objects carrying DT_NEEDED libc.so.6, so a second libc and the dynamic
 * loader enter a process that `file` calls "statically linked" and `ldd` calls
 * "not a dynamic executable".
 *
 * Across the 11 pinned environments this happens on 5 and KILLS the process on
 * 2 of them, Arch Linux (SIGFPE via libnss_mymachines, from `hosts:`) and
 * openSUSE Leap 15.6 (SIGFPE via libnss_compat, from `passwd:` -- so this is
 * not only a DNS problem). Fedora 42 loads two host modules and survives,
 * which is worse for a shipper than a clean failure would be.
 *
 * The fix
 * -------------------------------------------------------------------------
 * __nss_configure_lookup() replaces the service line for one database,
 * in-process, before any lookup happens. It is a PUBLIC glibc symbol
 * (versioned GLIBC_2.2.5) and it is present in libc.a, which is what makes it
 * usable from a static link at all.
 *
 * Pointed at services that glibc 2.34 and later implement INSIDE libc --
 * `files` for every database and `files dns` for hosts -- there is nothing
 * left to dlopen. Measured: zero host NSS modules on all 11 environments, DNS
 * still resolving.
 *
 * ⛔ THE GLIBC 2.34 floor IS load-bearing, not a detail. Before 2.34 `files`
 * and `dns` were themselves separate libnss_*.so files, so this override moves
 * the dlopen rather than removing it. experiments/21-glibc-version-floor.sh
 * measures that directly. The build environment pins a glibc at or above 2.34
 * for exactly this reason.
 *
 * ⚠ what this deliberately gives up. A program built this way cannot see
 * LDAP, SSSD, NIS, mDNS or systemd-resolved user and host databases. That is
 * the whole point -- those are the host modules being kept out -- but it is a
 * real behaviour change and it is documented in docs/limitations.md rather
 * than buried here. A program that MUST see enterprise directory data is
 * outside the class this tool serves.
 *
 * SPDX-License-Identifier: 0BSD
 */

#include <stddef.h>

/* Declared rather than included: <nss.h> does not declare it, and the glibc
 * header that does is internal. The signature is stable and public. */
extern int __nss_configure_lookup(const char *db, const char *service_line);

/* Every database glibc dispatches through NSS. Anything omitted here keeps
 * reading the host's nsswitch.conf line, so the list is deliberately complete
 * rather than "the ones that seemed to matter": openSUSE's crash arrived
 * through `passwd`, which a hosts-only fix would have missed. */
static const struct { const char *db; const char *line; } pgb_nss_map[] = {
    { "aliases",    "files"     },
    { "ethers",     "files"     },
    { "group",      "files"     },
    { "gshadow",    "files"     },
    { "hosts",      "files dns" },   /* both builtin from glibc 2.34 */
    { "initgroups", "files"     },
    { "netgroup",   "files"     },
    { "networks",   "files"     },
    { "passwd",     "files"     },
    { "protocols",  "files"     },
    { "publickey",  "files"     },
    { "rpc",        "files"     },
    { "services",   "files"     },
    { "shadow",     "files"     },
};

/* Priority 101: the lowest an application may use (0-100 are reserved by the
 * implementation). It has to beat any other constructor in the program that
 * might perform a lookup on its way up -- a library initialiser calling
 * gethostbyname() from its own constructor would otherwise get the host's
 * chain before this ran. */
__attribute__((constructor(101)))
static void pgb_nss_configure(void)
{
    for (size_t i = 0; i < sizeof pgb_nss_map / sizeof *pgb_nss_map; i++)
        (void) __nss_configure_lookup(pgb_nss_map[i].db, pgb_nss_map[i].line);
}

/* Referenced by the driver's link line via -Wl,-u,pgb_runtime_anchor so this
 * translation unit cannot be dropped. A constructor alone does not keep an
 * object alive when it arrives from an archive, and an nssfix that silently
 * did not get linked in is the exact failure this file exists to prevent. */
__attribute__((used, visibility("default")))
const char pgb_runtime_anchor[] = "pgb-runtime";
