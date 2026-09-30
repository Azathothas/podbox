// SPDX-License-Identifier: 0BSD
/*
 * binary/netdb.c - the network name databases, for hosts that ship none.
 *
 * -- the problem, MEASURED --------------------------------------------------
 *
 * ⛔ the eleventh glibc-static QUIRK, and it was found by a re-runnable SEARCH
 * rather than by a guess. `experiments/82-` enumerates every absolute path the
 * pinned `libc.a` names - 78 of them at glibc 2.41 - and classifies each
 * against the host-data rows this project already owns. `/etc/services` and
 * `/etc/protocols` were among the ones no row owned.
 *
 * The measurement that followed:
 *
 *     getservbyname("http", "tcp") returns NULL on 3 of 11
 *     debian-11, debian-12, ubuntu-20.04 - ⭐ all three GLIBC
 *     all four musl environments ship the file
 *
 * ⚠ so this IS not a musl STORY, and it is the same shape as the other ten:
 * static linking resolves the CODE and touches none of the DATA. A minimal
 * container image drops `netbase`, the file is gone, and every lookup of a
 * service by name answers NULL - which callers read as "no such service"
 * rather than as "this machine has no database".
 *
 * -- what this does, and the order IS a security property --------------------
 *
 * ⭐ the precedent IS `--embed-tzdata` and the rule IS its rule: look at the
 * host FIRST, carry a fallback, never prefer the stale copy. Here "look first"
 * is literal - the wrapper calls glibc's own implementation, which reads the
 * host's `/etc/services`, and only answers from the embedded table when that
 * returns NULL.
 *
 * ⛔ an administrator's /etc/services must win. A binary preferring its own
 * build-time snapshot over a file the machine's owner maintains would be a
 * regression wearing a portability fix's clothes - docs/AGENTS.md item 3
 * says so about every one of these mechanisms.
 *
 * -- ⛔ what it does not cover ----------------------------------------------
 *
 * `-Wl,--wrap=NAME` redirects calls to the exact symbol NAME. glibc's own
 * `getaddrinfo` does its service lookup through the INTERNAL alias
 * `__getservbyname_r`, inside the same object, so a `--wrap` on the public
 * name does not reach it. ⚠ `experiments/66-` measures that boundary rather
 * than asserting it: it asks both questions on all eleven and reports them in
 * separate columns.
 *
 * ⚠ and the table IS the build environment's FILE, not the IANA registry.
 * `/etc/services` on the pinned debian:13 is a few kilobytes, so unlike
 * tzdata's ~1,800 files there is no reason to carry a subset - the whole file
 * is embedded, and `pg-toolkit binary explain` prints how many entries that is.
 */
#include <errno.h>
#include <netdb.h>
#include <netinet/in.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct pgb_serv_ent {
    const char *name;
    const char *proto;
    unsigned short port; /* host order; converted on the way out */
};
struct pgb_proto_ent {
    const char *name;
    int number;
};

extern const struct pgb_serv_ent  pgb_serv_table[];
extern const unsigned             pgb_serv_count;
extern const struct pgb_proto_ent pgb_proto_table[];
extern const unsigned             pgb_proto_count;

struct servent  *__real_getservbyname(const char *, const char *);
struct servent  *__real_getservbyport(int, const char *);
struct protoent *__real_getprotobyname(const char *);
struct protoent *__real_getprotobynumber(int);

/* ⚠ the static return buffer IS glibc's own contract, not a shortcut taken
 * here: getservbyname returns a pointer into per-call static storage and is
 * documented as not thread-safe. Matching that exactly is the only way a
 * wrapper can be a drop-in; a caller wanting thread safety uses the _r form,
 * which is wrapped separately below and writes into the caller's buffer. */
static struct servent  pgb_sv;
static struct protoent pgb_pr;
static char           *pgb_no_aliases[1] = { 0 };
static char            pgb_sv_name[64], pgb_sv_proto[24], pgb_pr_name[64];

static const struct pgb_serv_ent *pgb_serv_find(const char *name, const char *proto)
{
    unsigned i;
    if (!name)
        return 0;
    for (i = 0; i < pgb_serv_count; i++) {
        if (strcmp(pgb_serv_table[i].name, name) != 0)
            continue;
        if (proto && *proto && strcmp(pgb_serv_table[i].proto, proto) != 0)
            continue;
        return &pgb_serv_table[i];
    }
    return 0;
}

static const struct pgb_serv_ent *pgb_serv_find_port(int port, const char *proto)
{
    unsigned i;
    for (i = 0; i < pgb_serv_count; i++) {
        if (pgb_serv_table[i].port != (unsigned short)port)
            continue;
        if (proto && *proto && strcmp(pgb_serv_table[i].proto, proto) != 0)
            continue;
        return &pgb_serv_table[i];
    }
    return 0;
}

static const struct pgb_proto_ent *pgb_proto_find(const char *name)
{
    unsigned i;
    if (!name)
        return 0;
    for (i = 0; i < pgb_proto_count; i++)
        if (strcmp(pgb_proto_table[i].name, name) == 0)
            return &pgb_proto_table[i];
    return 0;
}

static const struct pgb_proto_ent *pgb_proto_find_num(int n)
{
    unsigned i;
    for (i = 0; i < pgb_proto_count; i++)
        if (pgb_proto_table[i].number == n)
            return &pgb_proto_table[i];
    return 0;
}

static struct servent *pgb_fill_serv(const struct pgb_serv_ent *e)
{
    if (!e)
        return 0;
    snprintf(pgb_sv_name, sizeof pgb_sv_name, "%s", e->name);
    snprintf(pgb_sv_proto, sizeof pgb_sv_proto, "%s", e->proto);
    pgb_sv.s_name    = pgb_sv_name;
    pgb_sv.s_aliases = pgb_no_aliases;
    pgb_sv.s_port    = (int)htons(e->port);
    pgb_sv.s_proto   = pgb_sv_proto;
    return &pgb_sv;
}

static struct protoent *pgb_fill_proto(const struct pgb_proto_ent *e)
{
    if (!e)
        return 0;
    snprintf(pgb_pr_name, sizeof pgb_pr_name, "%s", e->name);
    pgb_pr.p_name    = pgb_pr_name;
    pgb_pr.p_aliases = pgb_no_aliases;
    pgb_pr.p_proto   = e->number;
    return &pgb_pr;
}

struct servent *__wrap_getservbyname(const char *name, const char *proto)
{
    struct servent *s = __real_getservbyname(name, proto);
    if (s)
        return s; /* ⭐ THE HOST'S FILE WINS, ALWAYS. */
    return pgb_fill_serv(pgb_serv_find(name, proto));
}

struct servent *__wrap_getservbyport(int port, const char *proto)
{
    struct servent *s = __real_getservbyport(port, proto);
    if (s)
        return s;
    return pgb_fill_serv(pgb_serv_find_port(ntohs((unsigned short)port), proto));
}

struct protoent *__wrap_getprotobyname(const char *name)
{
    struct protoent *p = __real_getprotobyname(name);
    if (p)
        return p;
    return pgb_fill_proto(pgb_proto_find(name));
}

struct protoent *__wrap_getprotobynumber(int n)
{
    struct protoent *p = __real_getprotobynumber(n);
    if (p)
        return p;
    return pgb_fill_proto(pgb_proto_find_num(n));
}

/* -- the _r forms -----------------------------------------------------------
 *
 * ⚠ these write into the caller's buffer and must not touch the static one.
 * The contract on failure is specific and getting it wrong is worse than not
 * wrapping at all: *result is set to NULL and the return value is 0 for
 * "not found", ERANGE for "your buffer is too small". */
int __real_getservbyname_r(const char *, const char *, struct servent *,
                           char *, size_t, struct servent **);
int __real_getservbyport_r(int, const char *, struct servent *,
                           char *, size_t, struct servent **);
int __real_getprotobyname_r(const char *, struct protoent *,
                            char *, size_t, struct protoent **);
int __real_getprotobynumber_r(int, struct protoent *,
                              char *, size_t, struct protoent **);

/* pgb_pack copies name and proto into the caller's buffer and lays an empty,
 * NULL-terminated alias vector after them, aligned for a pointer. */
static int pgb_pack(char *buf, size_t len, const char *a, const char *b,
                    char **outa, char **outb, char ***aliases)
{
    size_t na = strlen(a) + 1, nb = b ? strlen(b) + 1 : 0, off;
    uintptr_t addr;
    char *p = buf;
    if (na + nb + sizeof(char *) * 2 > len)
        return ERANGE;
    memcpy(p, a, na);
    *outa = p;
    p += na;
    if (b) {
        memcpy(p, b, nb);
        *outb = p;
        p += nb;
    } else if (outb) {
        *outb = 0;
    }
    /* ⛔ align the absolute address, not the offset into the buffer. The
     * caller's buffer is a char array and nothing promises it starts on a
     * pointer boundary, so rounding the OFFSET up leaves the alias vector
     * misaligned whenever `buf` itself is. x86_64 tolerates that and several
     * architectures this project intends to reach do not. */
    addr = (uintptr_t)p;
    addr = (addr + sizeof(char *) - 1) & ~(uintptr_t)(sizeof(char *) - 1);
    off = (size_t)(addr - (uintptr_t)buf);
    if (off + sizeof(char *) > len)
        return ERANGE;
    *aliases = (char **)(buf + off);
    (*aliases)[0] = 0;
    return 0;
}

int __wrap_getservbyname_r(const char *name, const char *proto,
                           struct servent *result_buf, char *buf, size_t len,
                           struct servent **result)
{
    const struct pgb_serv_ent *e;
    int rc = __real_getservbyname_r(name, proto, result_buf, buf, len, result);
    if (rc == 0 && *result)
        return rc;
    e = pgb_serv_find(name, proto);
    if (!e) {
        *result = 0;
        return rc;
    }
    rc = pgb_pack(buf, len, e->name, e->proto, &result_buf->s_name,
                  &result_buf->s_proto, &result_buf->s_aliases);
    if (rc != 0) {
        *result = 0;
        return rc;
    }
    result_buf->s_port = (int)htons(e->port);
    *result = result_buf;
    return 0;
}

int __wrap_getservbyport_r(int port, const char *proto,
                           struct servent *result_buf, char *buf, size_t len,
                           struct servent **result)
{
    const struct pgb_serv_ent *e;
    int rc = __real_getservbyport_r(port, proto, result_buf, buf, len, result);
    if (rc == 0 && *result)
        return rc;
    e = pgb_serv_find_port(ntohs((unsigned short)port), proto);
    if (!e) {
        *result = 0;
        return rc;
    }
    rc = pgb_pack(buf, len, e->name, e->proto, &result_buf->s_name,
                  &result_buf->s_proto, &result_buf->s_aliases);
    if (rc != 0) {
        *result = 0;
        return rc;
    }
    result_buf->s_port = (int)htons(e->port);
    *result = result_buf;
    return 0;
}

int __wrap_getprotobyname_r(const char *name, struct protoent *result_buf,
                            char *buf, size_t len, struct protoent **result)
{
    const struct pgb_proto_ent *e;
    char *unused = 0;
    int rc = __real_getprotobyname_r(name, result_buf, buf, len, result);
    if (rc == 0 && *result)
        return rc;
    e = pgb_proto_find(name);
    if (!e) {
        *result = 0;
        return rc;
    }
    rc = pgb_pack(buf, len, e->name, 0, &result_buf->p_name, &unused,
                  &result_buf->p_aliases);
    if (rc != 0) {
        *result = 0;
        return rc;
    }
    result_buf->p_proto = e->number;
    *result = result_buf;
    return 0;
}

int __wrap_getprotobynumber_r(int n, struct protoent *result_buf,
                              char *buf, size_t len, struct protoent **result)
{
    const struct pgb_proto_ent *e;
    char *unused = 0;
    int rc = __real_getprotobynumber_r(n, result_buf, buf, len, result);
    if (rc == 0 && *result)
        return rc;
    e = pgb_proto_find_num(n);
    if (!e) {
        *result = 0;
        return rc;
    }
    rc = pgb_pack(buf, len, e->name, 0, &result_buf->p_name, &unused,
                  &result_buf->p_aliases);
    if (rc != 0) {
        *result = 0;
        return rc;
    }
    result_buf->p_proto = e->number;
    *result = result_buf;
    return 0;
}
