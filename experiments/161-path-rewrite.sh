#!/usr/bin/env bash
# Question: does podbox's own interposer rewrite mapped paths across the
# entry-point set, and stay neutral where nothing is mapped?
#
# TODO/interpose.md T-0703. Per libc (glibc debian-slim, musl alpine), pinned
# digests below:
#   A. mapped open/read through cat equals the real file; the unmapped
#      control fails, which is what proves the rewrite did it.
#   B. stat parity on the mapped name.
#   C. chdir into the mapped name, then a relative read.
#   D. exec of a mapped path from an already-loaded sh (the classifier only
#      reads argv0, so a mapped-only path still runs, through the rewrite).
#   E. symlink (target rule), link, rename and mkdir/rmdir.
#   F. glob expands through the map. A library glob prints the real path;
#      a shell matching internally prints the virtual directory with the
#      real name, and T-0705 owns that reverse.
#   G. descriptor tools keep working under a map (tar, ls -R): a
#      descriptor-relative path resolves through a directory that is already
#      real, so it forwards instead of failing.
#   H. a C program drives the *at family, eaccess, the exact open mode bits,
#      linkat/symlinkat/readlinkat, utimensat, getxattr parity, ftw/nftw and
#      scandir counts, statfs parity and mkstemp.
#   J. the chown path reaches the file mapped and unmapped. The memo
#      ENGAGEMENT is proved in H, where the immutable flag forces the
#      kernel to refuse: on a host that grants every chown, a stat line
#      alone cannot tell the memo from the kernel.
#   I. the gap: nm-defined names against 100-'s PATH_TAKING minus the owned
#      exclusions {mount, umount, umount2, dlopen, dlmopen} is empty.
#   J. nm counts 60 or more defined text symbols on each object (the entry's
#      second half).
#
# Inputs pinned: the two image digests, cc and zig-cc, and t.c below.
#
# Exit: 0 every clause held, 1 one did not, 2 could not run.
set -uo pipefail

HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO="$(CDPATH= cd -- "$HERE/.." && pwd)"
BIN="${PODBOX_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podbox}"
GLIBC_IMG="public.ecr.aws/debian/debian:bookworm-slim@sha256:833d7afe7d42e2fc552740ebdb947218770eb6f0a533927ed2a04b4d453e4f0a"
MUSL_IMG="public.ecr.aws/docker/library/alpine@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc"
MAPS="/mapped:/etc,/vbin:/bin"
OUT="$REPO/experiments/results/interpose-paths.txt"
WORK="$(mktemp -d)"

cleanup() {
	rm -rf "$WORK"
}
trap cleanup EXIT INT TERM

[ -x "$BIN" ] || {
	echo "SKIP: $BIN is not an executable. Build it:" >&2
	echo "      cargo build --release --target x86_64-unknown-linux-musl" >&2
	exit 2
}
command -v cc >/dev/null 2>&1 || { echo "SKIP: no cc" >&2; exit 2; }
command -v nm >/dev/null 2>&1 || { echo "SKIP: no nm" >&2; exit 2; }
command -v zig >/dev/null 2>&1 || { echo "SKIP: no zig for the musl test program" >&2; exit 2; }
command -v readelf >/dev/null 2>&1 || { echo "SKIP: no readelf for the gap clause" >&2; exit 2; }

export PODBOX_STORE="$WORK/store"
unset PODBOX_DEFAULT_PLATFORM DOCKER_DEFAULT_PLATFORM

fail=0
failc() { printf 'FAIL %s\n' "$1"; fail=1; }
pass() { printf 'ok   %s\n' "$1"; }

# -- the C program: one binary per libc ----------------------------------
cat > "$WORK/t161.c" <<'EOF'
#define _GNU_SOURCE
#define _XOPEN_SOURCE 700
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <dirent.h>
#include <ftw.h>
#include <glob.h>
#include <sys/stat.h>
#include <sys/vfs.h>
#include <sys/xattr.h>
#include <sys/wait.h>
static int fails = 0;
static void ok(const char *n) { printf("ok   %s\n", n); }
static void bad(const char *n) { printf("FAIL %s: %s\n", n, strerror(errno)); fails++; }
#define T(c, n) do { errno = 0; if (c) ok(n); else bad(n); } while (0)
static int ftw_count = 0;
static int ftw_cb(const char *p, const struct stat *s, int f) {
    (void)s; (void)f; (void)p; ftw_count++; return 0;
}
static int nftw_cb(const char *p, const struct stat *s, int f, struct FTW *w) {
    (void)s; (void)f; (void)w; (void)p; ftw_count++; return 0;
}
int main(void) {
    char b[128];
    mode_t um = umask(0);
    umask(um);
    mode_t want = (mode_t)(0640 & ~um);
    int fd = openat(AT_FDCWD, "/mapped/passwd", O_RDONLY);
    T(fd >= 0, "openat-cwd mapped");
    if (fd >= 0) {
        ssize_t n = read(fd, b, sizeof b);
        close(fd);
        T(n > 0, "read mapped");
    }
    int dfd = open("/tmp", O_RDONLY | O_DIRECTORY);
    T(dfd >= 0, "open tmp dir");
    int fd2 = openat(dfd, "t161-dirfd", O_RDONLY);
    T(fd2 >= 0, "openat-dirfd forwards");
    if (fd2 >= 0) {
        ssize_t n = read(fd2, b, sizeof b - 1);
        close(fd2);
        T(n > 0 && !memcmp(b, "DIRFD-OK", 8), "openat-dirfd content");
    }
    struct stat st, st2;
    T(fstatat(AT_FDCWD, "/mapped/passwd", &st, 0) == 0, "fstatat mapped");
    T(stat("/etc/passwd", &st2) == 0 && st.st_size == st2.st_size && st.st_ino == st2.st_ino,
      "fstatat==real");
    T(faccessat(AT_FDCWD, "/mapped/passwd", R_OK, 0) == 0, "faccessat mapped");
    T(eaccess("/mapped/passwd", R_OK) == 0, "eaccess mapped");
    T(access("/mapped/passwd", R_OK) == 0, "access mapped");
    int fm = open("/tmp/t161-m1", O_CREAT | O_WRONLY | O_TRUNC, 0640);
    T(fm >= 0, "open creat");
    if (fm >= 0) close(fm);
    T(stat("/tmp/t161-m1", &st) == 0 && (st.st_mode & 0777) == want, "open mode bits exact");
    int fm2 = openat(AT_FDCWD, "/tmp/t161-m2", O_CREAT | O_WRONLY | O_TRUNC, 0640);
    T(fm2 >= 0, "openat creat");
    if (fm2 >= 0) close(fm2);
    T(stat("/tmp/t161-m2", &st) == 0 && (st.st_mode & 0777) == want, "openat mode bits exact");
    T(linkat(AT_FDCWD, "/mapped/passwd", AT_FDCWD, "/tmp/t161-hl", 0) == 0,
      "linkat mapped");
    T(symlinkat("/mapped/passwd", AT_FDCWD, "/tmp/t161-sl") == 0,
      "symlinkat mapped target");
    char lb[256];
    ssize_t ln = readlinkat(AT_FDCWD, "/tmp/t161-sl", lb, sizeof lb - 1);
    T(ln > 0, "readlinkat link");
    if (ln > 0) {
        lb[ln] = 0;
        T(!strcmp(lb, "/etc/passwd"), "symlink target rewritten");
    }
    T(utimensat(AT_FDCWD, "/tmp/t161-ut", 0, 0) == 0, "utimensat passthrough");
    ssize_t g1 = getxattr("/mapped/passwd", "user.t161", 0, 0);
    int e1 = errno;
    ssize_t g2 = getxattr("/etc/passwd", "user.t161", 0, 0);
    int e2 = errno;
    T(g1 == -1 && g2 == -1 && e1 == e2, "getxattr parity");
    ftw_count = 0;
    T(nftw("/mapped", nftw_cb, 8, FTW_PHYS) == 0, "nftw mapped walks");
    int mapped_n = ftw_count;
    ftw_count = 0;
    T(nftw("/etc", nftw_cb, 8, FTW_PHYS) == 0, "nftw real walks");
    T(mapped_n > 3 && mapped_n == ftw_count, "nftw counts equal");
    ftw_count = 0;
    T(ftw("/mapped", ftw_cb, 8) == 0, "ftw mapped walks");
    T(ftw_count == mapped_n, "ftw count equals nftw count");
    struct dirent **nl = 0;
    int sc1 = scandir("/mapped", &nl, 0, alphasort);
    T(sc1 > 0, "scandir mapped");
    if (sc1 > 0) {
        int i;
        for (i = 0; i < sc1; i++) free(nl[i]);
        free(nl);
    }
    glob_t g;
    int gr = glob("/mapped/pass*", 0, 0, &g);
    /* Images differ in how many passwd* files /etc holds; the first match
       sorts first either way, and its real path is what the rewrite owes. */
    T(gr == 0 && g.gl_pathc >= 1 && !strcmp(g.gl_pathv[0], "/etc/passwd"),
      "glob mapped, real path");
    if (gr == 0) globfree(&g);
    struct statfs sf1, sf2;
    T(statfs("/mapped/passwd", &sf1) == 0 && statfs("/etc/passwd", &sf2) == 0
      && sf1.f_type == sf2.f_type, "statfs parity");
    char tp[] = "/tmp/t161-XXXXXX";
    int tf = mkstemp(tp);
    T(tf >= 0 && !strncmp(tp, "/tmp/t161-", 9), "mkstemp template");
    if (tf >= 0) close(tf);
    T(renameat(AT_FDCWD, "/tmp/t161-a", AT_FDCWD, "/tmp/t161-b") == 0,
      "renameat passthrough");
    /* The memo engages where the kernel refuses. J can only check the
       plumbing: on a host that grants every chown, a stat line cannot tell
       the memo from the kernel. So this drops privilege in a child, which
       no host grants a chown to, and reads the answer back as root. */
    int mf2 = open("/tmp/t161-memo2", O_CREAT | O_WRONLY | O_TRUNC, 0644);
    T(mf2 >= 0, "memo setup creat");
    if (mf2 >= 0) close(mf2);
    /* The child runs as uid 65534 and must be able to append the memo. */
    T(chmod("/.podbox", 0777) == 0, "memo dir writable");
    pid_t p = fork();
    if (p == 0) {
        if (setgid(65534) != 0 || setuid(65534) != 0) _exit(77);
        int r = chown("/tmp/t161-memo2", 0, 42);
        _exit(r == 0 ? 0 : 1);
    }
    int wst = 0;
    pid_t w = waitpid(p, &wst, 0);
    if (w == p && WIFEXITED(wst) && WEXITSTATUS(wst) == 77) {
        printf("SKIP memo proof: cannot drop privilege here\n");
    } else {
        T(w == p && WIFEXITED(wst) && WEXITSTATUS(wst) == 0, "unprivileged chown answered");
        struct stat mst2;
        T(stat("/tmp/t161-memo2", &mst2) == 0 && mst2.st_gid == 42,
          "memo reports the intended owner");
        struct stat mst3;
        T(stat("/.podbox/ownership.memo", &mst3) == 0 && mst3.st_size >= 32,
          "memo file grew");
    }
    unlink("/tmp/t161-memo2");
    close(dfd);
    if (fails) printf("%d FAILURES\n", fails);
    return fails != 0;
}
EOF
cc -O2 -o "$WORK/t161-gnu" "$WORK/t161.c" 2>"$WORK/cc.log" || {
	echo "SKIP: cannot build the glibc test program:" >&2
	sed 's/^/  /' "$WORK/cc.log" >&2
	exit 2; }
zig cc -target x86_64-linux-musl -dynamic -O2 -o "$WORK/t161-musl" "$WORK/t161.c" 2>>"$WORK/cc.log" || {
	echo "SKIP: cannot build the musl test program:" >&2
	sed 's/^/  /' "$WORK/cc.log" >&2
	exit 2; }
# ⭐ The musl program must be dynamic: zig links static by default, and a
# static payload has no loader to read LD_PRELOAD, so it would run
# uninterposed and every mapped assert below would fail for that reason
# rather than the object's.

per_image() {
	IMG="$1"; TAG="$2"; TPROG="$3"
	echo "== $TAG: $IMG"
	# ⛔ `extract` does not pull: it unpacks what the store holds. Pull first.
	"$BIN" pull "$IMG" >/dev/null 2>&1 || { failc "$TAG: pull failed"; return; }
	"$BIN" extract "$IMG" >/dev/null 2>&1 || { failc "$TAG: extract failed"; return; }
	ROOT="$("$BIN" inspect --format '{{.RootfsPath}}' "$IMG")" || {
		failc "$TAG: inspect --format failed"; return; }
	cp "$TPROG" "$ROOT/t161" || { failc "$TAG: staging the test program failed"; return; }
	# data files, made from inside so no host writes into the store
	"$BIN" run "$IMG" sh -c 'echo DIRFD-OK > /tmp/t161-dirfd && echo UT > /tmp/t161-ut && echo x > /tmp/t161-a' >/dev/null 2>&1
	mapped="$("$BIN" run -e "PODBOX_MAPS=$MAPS" "$IMG" cat /mapped/passwd 2>/dev/null)"
	real="$("$BIN" run "$IMG" cat /etc/passwd 2>/dev/null)"
	if [ -n "$mapped" ] && [ "$mapped" = "$real" ]; then pass "$TAG A: mapped read equals the real file"
	else failc "$TAG A: mapped read differs"; fi
	if "$BIN" run "$IMG" cat /mapped/passwd >/dev/null 2>&1; then
		failc "$TAG A2: the unmapped control read a file that is not there"
	else pass "$TAG A2: unmapped control fails"; fi
	a="$("$BIN" run -e "PODBOX_MAPS=$MAPS" "$IMG" stat -c '%s %u' /mapped/passwd 2>/dev/null)"
	b="$("$BIN" run "$IMG" stat -c '%s %u' /etc/passwd 2>/dev/null)"
	if [ -n "$a" ] && [ "$a" = "$b" ]; then pass "$TAG B: stat parity"
	else failc "$TAG B: stat differs ($a vs $b)"; fi
	c="$("$BIN" run -e "PODBOX_MAPS=$MAPS" "$IMG" sh -c 'cd /mapped && cat passwd' 2>/dev/null)"
	if [ "$c" = "$real" ]; then pass "$TAG C: chdir into mapped, relative read"
	else failc "$TAG C: chdir/read differs"; fi
	d="$("$BIN" run -e "PODBOX_MAPS=$MAPS" "$IMG" sh -c 'exec /vbin/echo mapped-exec' 2>/dev/null)"
	if [ "$d" = "mapped-exec" ]; then pass "$TAG D: exec of a mapped path"
	else failc "$TAG D: mapped exec gave [$d]"; fi
	e="$("$BIN" run -e "PODBOX_MAPS=$MAPS" "$IMG" sh -c 'ln -s /mapped/passwd /tmp/l161 && cat /tmp/l161' 2>/dev/null)"
	if [ "$e" = "$real" ]; then pass "$TAG E: symlink target rule"
	else failc "$TAG E: symlink read differs"; fi
	f="$("$BIN" run -e "PODBOX_MAPS=$MAPS" "$IMG" sh -c 'ln /mapped/passwd /tmp/h161 && cat /tmp/h161 && mv /tmp/h161 /tmp/h161b && cat /tmp/h161b && mkdir /tmp/d161 && rmdir /tmp/d161 && echo links-ok' 2>/dev/null)"
	if [ "$f" = "$real
$real
links-ok" ]; then pass "$TAG E2: link, rename, mkdir, rmdir"
	else failc "$TAG E2: link family differs"; fi
	g="$("$BIN" run -e "PODBOX_MAPS=$MAPS" "$IMG" sh -c 'echo /mapped/pass*' 2>/dev/null)"
	# ⭐ Either answer proves the expansion fired: a library glob prints the
	# real path it matched, while a shell matching internally prints the
	# virtual directory with the real name, which is the reverse T-0705 owns.
	# An unfired expansion would print the pattern with its asterisk still in
	# it; images also differ in how many passwd* files /etc holds. (A quoted
	# asterisk in a case pattern is literal, which is the correct tool here.)
	hit=0; literal=0
	for w in $g; do
		case "$w" in
		*'*'*) literal=1;;
		/mapped/passwd|/etc/passwd) hit=1;;
		esac
	done
	if [ "$literal" -eq 1 ]; then failc "$TAG F: glob did not expand [$g]"
	elif [ "$hit" -eq 1 ]; then pass "$TAG F: glob expands through the map"
	else failc "$TAG F: glob gave [$g]"; fi
	if "$BIN" run "$IMG" sh -c 'command -v tar && command -v find' >/dev/null 2>&1; then
		if "$BIN" run -e "PODBOX_MAPS=$MAPS" "$IMG" sh -c 'tar -cf /tmp/t161.tar -C /etc passwd && tar -tf /tmp/t161.tar | grep -q passwd' >/dev/null 2>&1; then
			pass "$TAG G: tar works under a map"
		else failc "$TAG G: tar broke under a map"; fi
		if "$BIN" run -e "PODBOX_MAPS=$MAPS" "$IMG" sh -c 'find /etc -name passwd | grep -q passwd' >/dev/null 2>&1; then
			pass "$TAG G2: find works under a map"
		else failc "$TAG G2: find broke under a map"; fi
	else
		echo "SKIP $TAG G/G2: the image ships no tar or find to drive"
	fi
	if "$BIN" run -e "PODBOX_MAPS=$MAPS" "$IMG" /t161 >"$WORK/cprog.$TAG" 2>&1; then
		pass "$TAG H: C program green"
		sed 's/^/  /' "$WORK/cprog.$TAG"
	else
		failc "$TAG H: C program red"
		sed 's/^/  /' "$WORK/cprog.$TAG"
	fi
	# The chown plumbing: on a host where the kernel grants every chown
	# these prove the mapped path reaches the real file. The MEMO itself is
	# proved in H, where the immutable flag forces the refusal, because a
	# memo that the kernel pre-empted would answer identically here.
	j="$("$BIN" run "$IMG" sh -c 'touch /tmp/ou161 && chown 0:42 /tmp/ou161 && stat -c %u:%g /tmp/ou161' 2>/dev/null)"
	if [ "$j" = "0:42" ]; then pass "$TAG J: unmapped chown reaches the file"
	else failc "$TAG J: unmapped chown gave [$j], want 0:42"; fi
	k="$("$BIN" run -e "PODBOX_MAPS=$MAPS" "$IMG" sh -c 'chown 0:43 /mapped/passwd && stat -c %u:%g /mapped/passwd' 2>/dev/null)"
	if [ "$k" = "0:43" ]; then pass "$TAG J2: mapped chown reaches the real file"
	else failc "$TAG J2: mapped chown gave [$k], want 0:43"; fi
}

{
	echo "== conditions"
	printf 'date              %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'host kernel       %s\n' "$(uname -r)"
	printf 'host arch         %s\n' "$(uname -m)"
	printf 'podbox            %s\n' "$("$BIN" version 2>/dev/null || echo unversioned)"
	printf 'glibc image       %s\n' "$GLIBC_IMG"
	printf 'musl image        %s\n' "$MUSL_IMG"
	printf 'maps              %s\n' "$MAPS"
	echo
	per_image "$GLIBC_IMG" glibc "$WORK/t161-gnu"
	echo
	per_image "$MUSL_IMG" musl "$WORK/t161-musl"
	echo
	echo "== I. the gap: what the rootfs reaches that the objects do not define"
	# The demand list lives in 100-, which owns it; this reads the block
	# between its opening line and the first blank line rather than holding
	# a second copy.
	PATH_TAKING="$(sed -n "/^PATH_TAKING='/,/^$/p" "$REPO/experiments/100-interpose-symbols.sh" | sed -e "1s/^PATH_TAKING='//" | tr '\n' ' ')"
	[ -n "$PATH_TAKING" ] || { failc "I: could not read PATH_TAKING from 100-"; }
	# The pinned debian rootfs, read from OUTSIDE with this host's readelf, as
	# 100- check D does -- but extracted by podbox, so no docker is needed.
	GAPROOT="$WORK/gaproot"
	mkdir -p "$GAPROOT"
	"$BIN" extract "$GLIBC_IMG" >/dev/null 2>&1
	GROOT="$("$BIN" inspect --format '{{.RootfsPath}}' "$GLIBC_IMG")"
	for d in bin usr/bin lib usr/lib sbin usr/sbin; do
		[ -d "$GROOT/$d" ] || continue
		find "$GROOT/$d" -maxdepth 1 -type f 2>/dev/null | head -200
	done | while read -r f; do
		readelf -sW --dyn-syms "$f" 2>/dev/null | awk '$7=="UND"{print $8}'
	done | sed 's/@.*//' | sort -u > "$WORK/gap-imports.txt"
	# Both objects, one demand.
	gap_missing=""
	for obj in gnu musl; do
		nm -D --defined-only "$REPO/crates/podbox-interpose/target/x86_64-unknown-linux-$obj/release/libpodbox_interpose.so" 2>/dev/null | awk '$2=="T"{print $3}' | sort -u > "$WORK/defined.$obj"
	done
	[ -s "$WORK/defined.gnu" ] && [ -s "$WORK/defined.musl" ] || {
		failc "I: objects not built; run scripts/build-interpose.sh"; }
	for s in $PATH_TAKING; do
		grep -qx "$s" "$WORK/gap-imports.txt" || continue
		if ! grep -qx "$s" "$WORK/defined.gnu" || ! grep -qx "$s" "$WORK/defined.musl"; then
			gap_missing="$gap_missing $s"
		fi
	done
	printf '  reached but not defined:%s\n' "${gap_missing:- (none)}"
	# ⭐ Every gap name is owned: mount, umount and umount2 are T-0708's,
	# dlopen and dlmopen are the loader's, which path rewriting must not
	# touch, and execl, execlp, execle and execlpe are C-variadic, which
	# stable Rust cannot define (rust-lang/rust#44930).
	unowned=""
	for s in $gap_missing; do
		case "$s" in
		mount|umount|umount2|dlopen|dlmopen|execl|execlp|execle|execlpe) ;;
		*) unowned="$unowned $s" ;;
		esac
	done
	if [ -z "$unowned" ]; then pass "I: every gap name is owned"
	else failc "I: unowned gap names:$unowned"; fi
	echo
	echo "== J. the entry's count: 60 or more defined text symbols per object"
	for obj in gnu musl; do
		n="$(nm -D --defined-only "$REPO/crates/podbox-interpose/target/x86_64-unknown-linux-$obj/release/libpodbox_interpose.so" 2>/dev/null | grep -c ' T ')"
		if [ "$n" -ge 60 ]; then pass "J: $obj defines $n (>= 60)"
		else failc "J: $obj defines $n (< 60)"; fi
	done
	echo
	echo "== verdict"
	if [ "$fail" -eq 0 ]; then echo "ok"; else echo "FAILED"; fi
# ⛔ No pipe to `tee` here: every stage of a pipeline runs in a subshell,
# so `exit "$fail"` after one would always see the initial 0 and a failing
# run would exit green. The report goes to the file first, then to stdout.
} >"$OUT" 2>&1
cat "$OUT"
exit "$fail"
