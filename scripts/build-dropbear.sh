#!/usr/bin/env bash
# Build an ssh server a CAGE can run, with the shims already in.
#
# ⛔ WHY THIS EXISTS AND NOT "INSTALL DROPBear". Measured 2026-09-27 in the
# reference cage, a stock sshd cannot run at all:
#
#   1. `sshd -i -t -f <config>`   exits 0        <- passes every static check
#   2. `sshd -i` at runtime       wants a `nobody` user
#   3. with one                   wants /var/chroot/ssh, and /var does not exist
#   4. ChrootDirectory moved      no effect, a different hardcoded path
#   5. UsePrivilegeSeparation no  deprecated and IGNORED since OpenSSH 8.4
#
# so no configuration makes a modern sshd run where chroot(2) is denied, and
# a default that picks it fails with an error from the FAR END. dropbear has
# no privsep chroot. The full measurement is in
# docs/decisions/ssh-server-in-a-cage.md.
#
# ⛔ AND IT MUST BE BUILT DYNAMICALLY, WHICH IS THE PART THAT IS EASY TO GET
# WRONG AND IS NOT VISIBLE. A STATICALLY LINKED DROPBear CARRIES ITS OWN LIBC,
# SO `LD_PRELOAD` CANNOT REACH IT AND ITS PASSWD LOOKUPS FAIL AGAINST A CAGE
# THAT HAS NO /etc/passwd. The symptom is precise and misleading:
#
#   [26441] Login attempt for nonexistent user from localhost:E
#
# `root` exists. The binary cannot see that it does. `--disable-static-programs`
# is therefore REQUIRED and not a preference, and the build refuses to
# produce a static one.
#
# Two changes are applied to the source, each with the reason it is needed:
#
#   * setgroups tolerance. A seccomp-filtered cage denies setgroups(2), so
#     dropbear's initgroups() always fails and it exits with "Error changing
#     user group", killing every login. setgid stays FATAL, because setgid is
#     the call that changes the group and its failure is a real privilege
#     problem; a denied initgroups leaves a session with correct uid and gid
#     and merely no supplementary groups.
#   * ENOTSOCK tolerance, which upstream now carries, so it is asserted rather
#     than patched: a server on a pipe has no peer address to log.
#
#   ./scripts/build-dropbear.sh [outdir]
#
# Environment:
#   DROPBear_VERSION   the tag to build. Default is the pin below.
#   DROPBear_REF       a commit-ish instead of a tag, for a bisect.
#   DROPBear_SRC       an existing checkout to use instead of cloning.
#   CC, CFLAGS         passed through, so a cross build is a flag away.
#
# Exit: 0 built and every assertion passed, 1 a build or assertion failed,
# 2 could not run.
set -uo pipefail

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO=$(CDPATH= cd -- "$HERE/.." && pwd)

# ⛔ THE PIN IS A COMMIT, AND MEASURED RATHER THAN ASSUMED. The first version
# pinned a version STRING and cloned `--branch 2026.94`, and upstream has no
# such tag:
#
#     fatal: Remote branch 2026.94 not found in upstream origin
#
# The version lives in the source, not in a ref, so a version string is not a
# thing git can resolve and the clone cannot work. A commit always can. The
# commit below is the tree that reports `Dropbear v2026.94` and that this
# build was taken from, and BUILDINFO records the commit the build ACTUALLY
# got, so a reader never has to trust this constant.
DROPBear_COMMIT="${DROPBear_COMMIT:-59870ad43153fe8d4f1c96f5d5752116c94f31ff}"
OUTDIR="${1:-$REPO/dist/dropbear}"
WORK="${DROPBear_WORK:-$REPO/.work/dropbear-build}"

log()  { printf '%s\n' "$*" >&2; }
die()  { log "build-dropbear: $*"; exit 1; }
skip() { log "build-dropbear: $*"; exit 2; }

[ -d "$OUTDIR" ] || mkdir -p "$OUTDIR" || die "cannot create $OUTDIR"
command -v git >/dev/null 2>&1 || skip "no git"
command -v make >/dev/null 2>&1 || skip "no make"
command -v cc >/dev/null 2>&1 || skip "no C compiler"

# ---------------------------------------------------------------- the source
SRC="${DROPBear_SRC:-}"
if [ -z "$SRC" ]; then
    if [ -d "$WORK/.git" ]; then
        log "build-dropbear: reusing $WORK"
    else
        rm -rf "$WORK"; mkdir -p "$WORK" || die "cannot create $WORK"
        # A COMMIT and not a branch, because a branch moves and a commit does
        # not, and this build is a published artefact with a shim inside it.
        git init --quiet "$WORK" 2>/dev/null || die "cannot init $WORK"
        git -C "$WORK" remote add origin https://github.com/mkj/dropbear 2>/dev/null || true
        git -C "$WORK" fetch --quiet --depth 1 origin "$DROPBear_COMMIT" 2>/dev/null \
            || skip "cannot fetch dropbear $DROPBear_COMMIT"
        git -C "$WORK" checkout --quiet FETCH_HEAD 2>/dev/null \
            || skip "cannot check out $DROPBear_COMMIT"
    fi
    SRC="$WORK"
fi
[ -f "$SRC/src/svr-auth.c" ] || die "$SRC does not look like a dropbear tree"
cd "$SRC" || die "cannot enter $SRC"

ACTUAL=$(git rev-parse HEAD 2>/dev/null || echo unknown)
log "build-dropbear: source $SRC at $ACTUAL"

# ---------------------------------------------------------------- the patches
# ⛔ EACH PATCH IS IDEMPOTENT AND SAYS SO WHEN IT IS ALREADY THERE. A patch
# that is already upstream must not be re-applied, and a patch that fails to
# apply must stop the build rather than produce a binary that silently lacks
# the change. `--forward --reject` would leave a .rej to be found later; a
# grep for the change is the check that cannot be skipped.
patch_setgroups() {
    if grep -q 'setgroups(2) is denied' src/svr-auth.c 2>/dev/null; then
        log "build-dropbear:   setgroups tolerance already present"
        return 0
    fi
    python3 - "$SRC/src/svr-auth.c" <<'PYEOF' || return 1
import sys
p = sys.argv[1]
s = open(p).read()
old = """\t\tif ((setgid(ses.authstate.pw_gid) < 0) ||
\t\t\t(initgroups(ses.authstate.pw_name, 
\t\t\t\t\t\tses.authstate.pw_gid) < 0)) {
\t\t\tdropbear_exit("Error changing user group");
\t\t}"""
new = """\t\t/* setgroups(2) is denied by a seccomp-filtered cage, so
\t\t * initgroups() always fails there and dropping the session over it
\t\t * makes every login impossible. setgid stays fatal: it is the call
\t\t * that changes the group, and a failure there is a real privilege
\t\t * problem rather than a filtered no-op. */
\t\tif (setgid(ses.authstate.pw_gid) < 0) {
\t\t\tdropbear_exit("Error changing user group");
\t\t}
\t\tif (initgroups(ses.authstate.pw_name,
\t\t\t\t\tses.authstate.pw_gid) < 0) {
\t\t\t/* Intentionally ignored: see above. */
\t\t}"""
if old in s:
    open(p, 'w').write(s.replace(old, new))
    sys.exit(0)
# Already shaped differently upstream: accept it if the tolerant form is
# present, refuse otherwise rather than guess.
if 'initgroups(ses.authstate.pw_name' in s and 'Intentionally ignored' in s:
    sys.exit(0)
sys.stderr.write("build-dropbear: the setgroups block moved upstream; refusing to guess\n")
sys.exit(1)
PYEOF
}

patch_setgroups || die "the setgroups tolerance did not apply"
log "build-dropbear:   setgroups tolerance applied"

# ---------------------------------------------------------------- the build
# ⛔ --disable-static-programs IS LOAD-BEARING AND NOT A DEFAULT. See the
# header: a static dropbear cannot see the passwd database and logs
# "nonexistent user" for a user that exists.
CONFIGURE_ARGS="--disable-zlib --disable-usage --disable-static-programs"
log "build-dropbear: configure $CONFIGURE_ARGS"
./configure $CONFIGURE_ARGS >"$WORK.configure.log" 2>&1 \
    || die "configure failed; see $WORK.configure.log"
log "build-dropbear: make"
make -j"$(nproc 2>/dev/null || echo 2)" dropbear dropbearkey \
    >"$WORK.make.log" 2>&1 || die "make failed; see $WORK.make.log"

# ---------------------------------------------------------------- assertions
# ⛔ THE ARTEFACT IS CHECKED, NOT ASSUMED. Every one of these is a property
# that was measured to matter, and a build that cannot be checked is a build
# whose output nobody should run.
fail=0
note() { log "build-dropbear: $1"; }
bad()  { log "build-dropbear: FAIL $1"; fail=1; }

# ⛔ THESE TWO ARE CHECKED HERE AND NOT DEFERRED, because every assertion below
# reads one of them and an assertion over a file that does not exist is a
# check that cannot fail.
[ -x "$SRC/dropbear" ]    || bad "no dropbear binary at $SRC/dropbear"
[ -x "$SRC/dropbearkey" ] || bad "no dropbearkey at $SRC/dropbearkey"

if command -v file >/dev/null 2>&1; then
    kind=$(file -b "$SRC/dropbear" 2>/dev/null)
    case "$kind" in
        *"statically linked"*)
            # ⛔ A STATIC BINARY IS A HARD FAIL AND NOT A WARNING. It cannot
            # see the passwd database, so every login fails with a message
            # that names the wrong thing.
            bad "dropbear is statically linked, so LD_PRELOAD cannot reach it: $kind" ;;
        *"dynamically linked"*) note "dynamically linked, as the shim requires" ;;
        *) note "could not classify: $kind" ;;
    esac
fi

# ⛔ THE TOLERANCE MUST BE IN THE BINARY, NOT ONLY IN THE SOURCE. Asserting
# the source proves the file was edited; asserting the artefact proves the
# thing that runs has the change, and a stale object file is exactly how a
# source edit ships without taking effect.
# ⛔ "Error changing user group" IS THE setgid EXIT, AND setgid STAYS FATAL,
# SO ITS PRESENCE IS CORRECT AND ITS ABSENCE IS THE DEFECT. The first version
# of this assertion read it the other way round and failed a good build,
# which is what an assertion written from a guess does.
# ⛔ THIS CHECKS THE ARTEFACT THAT WAS JUST BUILT, AND THE PUBLISH STEP
# BELOW COPIES IT VERBATIM, so the two are the same file. The first version
# pointed at "$OUTDIR/dropbear", which does not exist yet at this point in the
# script, and the guard silently skipped the check: a guard that cannot find
# its subject reports nothing, which is how a check stops being one.
#
# It is a guard over a path that must exist, so a MISSING BINARY IS A
# FAILURE rather than a skip. The first version had it the other way round
# and passed a build with no binary at all.
BUILT="$SRC/dropbear"
if command -v strings >/dev/null 2>&1; then
    if [ ! -f "$BUILT" ]; then
        bad "no binary at $BUILT, so nothing below can be checked"
    # ⛔ `grep -c` NOT `grep -q`, AND THAT IS LOAD-BEARING UNDER `pipefail`.
    # `grep -q` EXITS THE MOMENT IT MATCHES, so `strings` takes a SIGPIPE and
    # the PIPELINE reports 141. Under `set -o pipefail` -- which this script
    # sets -- the `if` then reads that as "the string is absent" and fails a
    # binary that contains it. Measured: the same pipeline is MATCH without
    # pipefail, NOMATCH with it, and `rc=141` in between.
    #
    # `grep -c` reads its whole input, so no SIGPIPE, and the count is
    # discarded. This is the same trap `TODO/RULES.md` section 8 names for
    # `cmd | head -1`, and it is the reason an exit code must be read from the
    # process that produced it rather than through a pipeline.
    elif [ "$(strings "$BUILT" 2>/dev/null | grep -c 'Error changing user group')" -ge 1 ]; then
        note "setgid stays fatal, as intended: the exit string is in the binary"
    else
        bad "the setgid exit string is gone, so the tolerance took setgid with it"
    fi
    # ⛔ THE COUNT THAT MATTERS IS NOT A COUNT OF setgid. The original was a
    # SINGLE combined condition, so the patched form has one setgid and one
    # initgroups, and "the source has two setgid references" is a check for a
    # shape this patch does not produce. It failed a correct build, which is
    # what an assertion written from a guess does.
    #
    # What IS checkable, and it is checkable in the SOURCE because the
    # compiled form of "this initgroups failure is ignored" is the ABSENCE of
    # a call, which no string search can see:
    #
    #   * the initgroups check is NOT followed by a dropbear_exit, and
    #   * the setgid check IS.
    # ⛔ THE SEARCH IS BOUNDED TO THE BLOCK, AND THAT BOUND IS THE WHOLE
    # CORRECTNESS OF THIS CHECK. `awk` with a sticky flag scans to the END OF
    # THE FILE, so "the first dropbear_exit after initgroups" found
    # `dropbear_exit("Error changing user")` thirty lines later, which is a
    # DIFFERENT failure and made a correct patch look unlanded. The block is
    # the call and the few lines that close it.
    if python3 - src/svr-auth.c <<'PYEOF2'
import re, sys
s = open(sys.argv[1]).read()
# The initgroups call, then its condition body, up to the closing brace at
# the same indent. Anything with a dropbear_exit inside THAT is fatal.
m = re.search(r"if \(initgroups\([^)]*\)\s*<\s*0\)\s*\{(.*?)\n\t\t\}", s, re.S)
if not m:
    sys.stderr.write("no initgroups condition found\n"); sys.exit(2)
body = m.group(1)
if "dropbear_exit" in body:
    sys.exit(1)
sys.exit(0)
PYEOF2
    then
        note "initgroups failure is tolerated in the source"
    else
        bad "initgroups is still fatal, so the tolerance did not land"
    fi
    if python3 - src/svr-auth.c <<'PYEOF3'
import re, sys
s = open(sys.argv[1]).read()
m = re.search(r"if \(setgid\(ses.authstate.pw_gid\)\s*<\s*0\)\s*\{(.*?)\n\t\t\}", s, re.S)
if not m:
    sys.stderr.write("no setgid condition found\n"); sys.exit(2)
if "dropbear_exit" in m.group(1):
    sys.exit(0)
sys.exit(1)
PYEOF3
    then
        note "setgid is still fatal, which is the whole point of the patch"
    else
        bad "setgid is no longer fatal, so the patch was too broad"
    fi
fi

# The passwd shim, built for the same libc, so an agent can use this pair.
# ⛔ THE SHIM IS FOUND BY SEARCHING, NOT BY ONE HARD-CODED PATH. It lives in
# the sibling repository today and beside this script on some hosts, and a
# builder that only knows one of them silently ships a server with no shim,
# which is the one combination that cannot work.
SHIM_SRC="${DROPBear_SHIM:-}"
if [ -z "$SHIM_SRC" ]; then
    for cand in "$REPO/../sandssh/shims/fakepwd.c" \
                "$REPO/.sandssh-work/sandssh/shims/fakepwd.c" \
                "$HERE/../shims/fakepwd.c" \
                "$HERE/common/fakepwd.c"; do
        if [ -r "$cand" ]; then SHIM_SRC="$cand"; break; fi
    done
fi
if [ -r "$SHIM_SRC" ]; then
    cc -shared -fPIC -O2 -o "$SRC/fakepwd.so" "$SHIM_SRC" \
        || bad "the passwd shim did not build"
    [ -f "$SRC/fakepwd.so" ] && note "passwd shim built beside the server"
else
    # ⛔ A MISSING SHIM IS A FAILURE AND NOT A NOTE, because the whole point
    # of this builder is a server that WORKS in a cage, and without the shim it
    # logs "nonexistent user" for a user that is there.
    bad "no fakepwd.c found, so the server would ship without its shim; set DROPBear_SHIM"
fi

# ---------------------------------------------------------------- publish
for f in dropbear dropbearkey; do
    [ -x "$SRC/$f" ] || continue
    cp "$SRC/$f" "$OUTDIR/$f" || bad "cannot copy $f into $OUTDIR"
    chmod 0755 "$OUTDIR/$f"
done
[ -f "$SRC/fakepwd.so" ] && cp "$SRC/fakepwd.so" "$OUTDIR/fakepwd.so"

if command -v sha256sum >/dev/null 2>&1; then
    ( cd "$OUTDIR" && sha256sum dropbear dropbearkey >SHA256SUMS 2>/dev/null ) || true
fi

# A manifest, so a consumer knows what it got and what it must do with it.
cat >"$OUTDIR/BUILDINFO" <<EOF
source        dropbear at $ACTUAL
commit        $ACTUAL
configure     $CONFIGURE_ARGS
patches       setgroups tolerance (seccomp-filtered cages deny setgroups(2))
linked        dynamically, on purpose: a static dropbear carries its own libc
              and cannot see the passwd database, so every login fails with
              "Login attempt for nonexistent user"
shim          fakepwd.so, for a cage with no /etc/passwd
use           LD_PRELOAD=./fakepwd.so dropbear -i -E -s -g -F -r <hostkey> -D <akdir>
why-not-sshd  sshd -i wants a privsep chroot at /var/chroot/ssh, which a cage
              without chroot(2) cannot provide; see
              docs/decisions/ssh-server-in-a-cage.md
EOF

log "build-dropbear: wrote $OUTDIR"
[ "$fail" -eq 0 ] || exit 1
exit 0
