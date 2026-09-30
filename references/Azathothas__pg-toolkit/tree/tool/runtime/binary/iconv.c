/* binary/iconv.c -- give a static glibc binary its encodings back.
 *
 * The problem, measured in experiments/30-gconv-and-locale.sh
 * -------------------------------------------------------------------------
 * glibc reaches every encoding except a small builtin set through gconv
 * modules that it dlopen()s from a path compiled into libc. Each module
 * carries DT_NEEDED libc.so.6.
 *
 * In a static binary that produces two failures and no third, working case:
 *
 *   host gconv path == build gconv path (Debian, Ubuntu): the modules load,
 *   drag in a second libc, and the process dies. SIGFPE or SIGABRT.
 *
 *   host gconv path != build gconv path (Fedora, Rocky, Arch, openSUSE) or
 *   the host has no glibc at all (every musl distro): 11 of 12 tested
 *   encodings silently return EINVAL from iconv_open().
 *
 * The fix
 * -------------------------------------------------------------------------
 * GNU libiconv carries the same encoding tables as ordinary code in an
 * archive. Linked statically it needs no dlopen, no module path and no data
 * directory, so it cannot be affected by anything on the host.
 *
 * ⭐ the redirection IS a link-time one and that IS why no source changes.
 * The driver passes -Wl,--wrap=iconv_open,--wrap=iconv,--wrap=iconv_close, so
 * ld rewrites every UNDEFINED reference to those names into __wrap_* and this
 * file answers them. It therefore catches calls from the application, from any
 * static library in the link, and from libraries compiled long before this
 * tool existed -- none of them have to have seen a pgb header.
 *
 * ⭐ why this FILE IS in an archive and nssfix.o IS NOT. An archive member
 * is pulled in only when something references a symbol it defines. Nothing
 * references __wrap_iconv_open unless the program actually calls iconv_open,
 * so a program that does no character conversion links none of this and none
 * of libiconv, and pays none of the ~900 KiB. The nssfix constructor has no
 * referenced symbol at all, which is why it is passed as a plain object with
 * -Wl,-u instead. experiments/40 measures both sizes.
 *
 * ⚠ what IS not redirected, and why it IS still fine.
 * glibc's own internal conversions -- wide-character I/O, printf("%ls"),
 * setlocale()'s charset handling -- call the hidden __gconv_* entry points,
 * not the public iconv_open, so --wrap cannot see them and does not try.
 * Those paths stay on glibc's gconv. They are safe because the conversions
 * they need between glibc's INTERNAL form and ASCII or UTF-8 are BUILTIN and
 * require no module. It is only a non-UTF-8, non-ASCII locale charset that
 * would send glibc's internals back to a dlopen, which is one more reason the
 * driver's locale story is C.UTF-8 rather than an arbitrary host locale.
 *
 * SPDX-License-Identifier: 0BSD
 */

#include <stddef.h>

/* libiconv's own names. Declared here rather than by including its <iconv.h>
 * so this file compiles against whichever iconv.h is on the include path --
 * including glibc's, which is what a normal build environment has. */
typedef void *pgb_iconv_t;
extern pgb_iconv_t libiconv_open(const char *tocode, const char *fromcode);
extern size_t      libiconv(pgb_iconv_t cd, char **inbuf, size_t *inbytesleft,
                            char **outbuf, size_t *outbytesleft);
extern int         libiconv_close(pgb_iconv_t cd);

/* The ABI matches: both return (iconv_t)-1 from open on failure, both return
 * (size_t)-1 from the conversion and set errno, both take the same argument
 * shapes. No translation layer is needed and none is added -- a shim that
 * "helpfully" remapped errno would be a place for behaviour to diverge. */
pgb_iconv_t __wrap_iconv_open(const char *tocode, const char *fromcode)
{
    return libiconv_open(tocode, fromcode);
}

size_t __wrap_iconv(pgb_iconv_t cd, char **inbuf, size_t *inbytesleft,
                    char **outbuf, size_t *outbytesleft)
{
    return libiconv(cd, inbuf, inbytesleft, outbuf, outbytesleft);
}

int __wrap_iconv_close(pgb_iconv_t cd)
{
    return libiconv_close(cd);
}
