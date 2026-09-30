/* binary/dlopen.h -- the shape of the compiled-in plugin table.
 *
 * Shared by binary/dlopen.c, which reads the table, and by the table itself,
 * which `pgb` GENERATES per build into pgb-dlopen-table.c. Two files that
 * have to agree on a layout get one header, or they agree until somebody
 * edits one of them.
 *
 * SPDX-License-Identifier: 0BSD
 */
#ifndef PGT_BINARY_DLOPEN_H
#define PGT_BINARY_DLOPEN_H

#include <stddef.h>

/* One exported symbol of one plugin. Terminated by a NULL name. */
struct pgb_dl_sym {
    const char *name;
    void       *addr;
};

/* One plugin. `name` is the string the program will pass to dlopen(); see
 * binary/dlopen.c for the matching rule, which is not plain equality.
 * Terminated by a NULL name. */
struct pgb_dl_lib {
    const char              *name;
    const struct pgb_dl_sym *syms;
};

/* ⭐ WEAK, and that IS load-bearing. The table is generated only when the
 * caller asked for --wrap-dlopen. A weak undefined reference resolves to 0
 * rather than failing the link, so binary/dlopen.c can be in the runtime archive
 * unconditionally and still link into a program that has no table at all --
 * where it reports an honest "no plugin table was compiled in" through
 * dlerror() instead of breaking the build. */
extern const struct pgb_dl_lib pgb_dlopen_libs[] __attribute__((weak));

#endif /* PGT_BINARY_DLOPEN_H */
