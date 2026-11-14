#!/bin/sh
# Question: before the release for beta testers, which commands of the
# user surface of podvm give a result that differs from the documented
# result or from the result of docker, on the default rung and on the
# userland rung?
#
# The beta testers and the hosted runners of podrunner take the userland
# rung (loader family). The fixture of experiment 356 forces that rung on
# a host that permits user namespaces: a user and mount namespace, then
# experiments/deny-chroot.c denies chroot(2). The default rung of a cloud
# session (root, user namespaces permitted) is the namespace rung.
#
# Inputs:
#   PODVM_443_BIN    the podvm binary (default: .work/qa/podvm).
#   PODVM_STORE      the store (default: .work/qa/store). Each run uses
#                    a private store; the script does not touch the
#                    store of the user.
#   PODVM_443_ONLY   a list of sections to run, separated by spaces
#                    (default: all): img run imgs tools life sys perf lab
#                    machine.
#   cc, unshare, python3 (jsonschema) and timeout on the host.
#   The images are tags of mirror.gcr.io/library (not pinned: the QA pass
#   tests what a beta tester pulls today).
#
# Clauses: each row of experiments/results/qa-surface.txt is one clause,
#   in the format
#     ID | COMMAND | RUNG | EXPECTED | GOT | VERDICT
#   RUNG "ns" is the default rung, RUNG "ul" is the fixture (userland).
#   EXPECTED is the result of docker, or the documented result where
#   podvm documents a difference. A row that measures and does not judge
#   has the verdict INFO.
#
# Each command has a bound (timeout). A row whose command reaches its
# bound reads "rc=124" in GOT.
#
# Exit: 0 each clause matched, 1 one or more clauses failed, 2 the lane
# could not run.
set -u

REPO="$(cd "$(dirname "$0")/.." && pwd)"
QA="$REPO/.work/qa"
PB="${PODVM_443_BIN:-$QA/podvm}"
export PODVM_STORE="${PODVM_STORE:-$QA/store}"
W="$QA/work443"
FXD="$QA/fx"
OUT="$REPO/experiments/results/qa-surface.txt"
ONLY="${PODVM_443_ONLY:-img run imgs tools life sys perf lab machine}"

[ -x "$PB" ] || { echo "no podvm binary at $PB: COULD NOT RUN" >&2; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" >&2; exit 2; }
unshare -Urm true >/dev/null 2>&1 || { echo "user+mount namespaces refused: COULD NOT RUN" >&2; exit 2; }
mkdir -p "$W" "$FXD" "$PODVM_STORE" || exit 2
chmod 700 "$PODVM_STORE"
cc -O2 -Wall -Werror -static -o "$FXD/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>"$W/cc.err" ||
	{ echo "the fixture does not build: COULD NOT RUN" >&2; exit 2; }
cat >"$FXD/wrap.sh" <<'EOF'
#!/bin/sh
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF
FXCMD="unshare -Urm sh $FXD/wrap.sh $FXD"

R=mirror.gcr.io/library
A=$R/alpine:3.20
D=$R/debian:bookworm-slim
U=$R/ubuntu:24.04
PY=$R/python:3.12-slim
N=$R/node:22-slim
G=$R/golang:1.24-bookworm
B=$R/busybox:musl
export PB W A D U PY N G B
# The proxy of the host is a local port with no credential; the payload
# shares the network of the host, so the variables reach the proxy.
HP="${HTTPS_PROXY:-${https_proxy:-}}"
PX=""
[ -n "$HP" ] && PX="-e https_proxy=$HP -e HTTPS_PROXY=$HP -e http_proxy=$HP -e HTTP_PROXY=$HP"
export PX
PPORT=$(printf '%s' "$HP" | sed -n 's#.*:\([0-9][0-9]*\)/*$#\1#p')
export PPORT

on() { case " $ONLY " in *" $1 "*) return 0;; esac; return 1; }

# The result file starts again only for a full run; a run of some
# sections appends.
[ -z "${PODVM_443_ONLY:-}" ] && : >"$OUT"
# Each row is printable ASCII: an error can echo the bytes of a bad input.
note() { printf '%s\n' "$*" | LC_ALL=C tr -c '\11\12\40-\176' '?' | tee -a "$OUT"; }
npass=0; nfail=0; ninfo=0

# The noise of a first extraction and of the fixups, which a quiet run
# must not print. The GOT column leaves it out, and the row of the quiet
# check counts it.
noise='Extracted \(|entries carry an id|restore ownership|no mapping in this user|is not mapped in this user|without ownership and record|^rehash: warning'

# t ID RUNG EXPECTED CONDITION COMMAND
#   The command runs in sh -c with X set to the fixture prefix on rung ul
#   and empty on rung ns. The condition reads $rc, $out, $err and $ms.
t() {
	id=$1; rung=$2; exp=$3; cond=$4; cmd=$5
	if [ "$rung" = ul ]; then X="$FXCMD"; else X=""; fi
	RG=$rung
	export X RG
	s=$(date +%s%N)
	timeout -k 5 "${TO:-120}" sh -c "$cmd" >"$W/$id.$rung.out" 2>"$W/$id.$rung.err" </dev/null
	rc=$?
	ms=$(( ($(date +%s%N) - s) / 1000000 ))
	of="$W/$id.$rung.out"
	out=$(head -c 4000 "$of")
	err=$(head -c 20000 "$W/$id.$rung.err")
	if [ "$cond" = INFO ]; then
		v=INFO; ninfo=$((ninfo + 1))
	elif eval "$cond"; then
		v=PASS; npass=$((npass + 1))
	else
		v=FAIL; nfail=$((nfail + 1))
	fi
	so=$(printf '%s' "$out" | tr '\n' '|' | cut -c1-160)
	se=$(printf '%s\n' "$err" | grep -Ev "$noise" | tr '\n' '|' | sed 's/|$//' | cut -c1-220)
	shown=$(printf '%s' "$cmd" | tr '\n' ' ' | sed 's/\$X //g; s/\$PB/podvm/g')
	nz=$(printf '%s\n' "$err" | grep -Ec "$noise")
	nzs=""; [ "$nz" -gt 0 ] && nzs=" noise_lines=$nz"
	note "$id | $shown | $rung | $exp | rc=$rc ${ms}ms out=[$so] err=[$se]$nzs | $v"
	unset TO
}
has() { hs=$1; shift; printf '%s' "$hs" | grep -q "$@"; }
noisy() { printf '%s\n' "$err" | grep -Ec "$noise"; }

note "== conditions"
note "date        $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "kernel      $(uname -sr)"
note "podvm       $("$PB" version 2>&1 | head -1) ($("$PB" version --verbose 2>&1 | sed -n 's/^commit: //p'))"
note "store       $PODVM_STORE"
note "rung ns     $("$PB" probe 2>/dev/null | head -1)"
note "rung ul     the fixture: $FXCMD"
note "legend      A=$A D=$D U=$U PY=$PY N=$N G=$G B=$B PX=(-e of the local proxy port)"
note ""

# ---------------------------------------------------------------- img
if on img; then
note "== 1. image verbs"
for i in "$A" "$D" "$U" "$PY" "$N" "$G" "$B"; do
	TO=600 t "img.pull.$(basename "$i" | tr ':' '_')" ns "rc 0" '[ $rc -eq 0 ]' "\$PB pull -q $i"
done
t img.pull.ul ul "rc 0" '[ $rc -eq 0 ]' '$X $PB pull -q $A'
t img.pull.q-stdout ns "pull -q prints the reference or nothing; stderr empty" '[ $rc -eq 0 ] && [ -z "$err" ]' '$PB pull -q $A'
t img.images ns "lists alpine" 'has "$out" alpine' '$PB images'
t img.images.ul ul "lists alpine" 'has "$out" alpine' '$X $PB images'
t img.images-q ns "ids only" '[ $rc -eq 0 ] && ! has "$out" REPOSITORY' '$PB images -q'
t img.images-format ns "repo:tag lines" 'has "$out" "alpine:3.20"' "\$PB images --format '{{.Repository}}:{{.Tag}}'"
t img.images-filter ns "only alpine" 'has "$out" alpine && ! has "$out" debian' '$PB images --filter name=alpine'
t img.images-digests ns "DIGEST column" 'has "$out" DIGEST' '$PB images --digests'
t img.inspect ns "JSON array, rc 0" '[ $rc -eq 0 ] && python3 -I -c "import json,sys; d=json.load(sys.stdin); assert isinstance(d,list)" <"$of"' '$PB inspect $A'
t img.inspect-f ns "the id" '[ $rc -eq 0 ] && has "$out" sha256' "\$PB inspect -f '{{.Id}}' \$A"
t img.inspect-cfg ns "the Cmd of python" 'has "$out" python3' "\$PB inspect -f '{{.Config.Cmd}}' \$PY"
t img.inspect-missing ns "rc 1, names the image" '[ $rc -ne 0 ] && has "$err" nosuch' '$PB inspect nosuch/image:1'
t img.tag ns "rc 0, new name listed" '[ $rc -eq 0 ] && has "$out" qa/alpine' '$PB tag $A qa/alpine:mine && $PB images'
t img.tag-run ns "runs by the new name" '[ $rc -eq 0 ] && [ "$out" = ok ]' '$PB run --rm qa/alpine:mine echo ok'
t img.tag-run.ul ul "runs by the new name" '[ $rc -eq 0 ] && [ "$out" = ok ]' '$X $PB run --rm qa/alpine:mine echo ok'
t img.rmi-tag ns "rc 0, name gone, alpine stays" '[ $rc -eq 0 ] && ! has "$out" qa/alpine && has "$out" alpine' '$PB rmi qa/alpine:mine >/dev/null && $PB images'
t img.rmi-missing ns "rc 1 (docker), names it" '[ $rc -eq 1 ] && has "$err" nosuch' '$PB rmi nosuch/image:1'
t img.save-o ns "OCI layout tar" '[ $rc -eq 0 ] && has "$out" index.json && has "$out" oci-layout' 'rm -f $W/bb.tar; $PB save -o $W/bb.tar $B && tar tf $W/bb.tar'
t img.save-stdout ns "same bytes as -o" '[ $rc -eq 0 ]' '$PB save $B > $W/bb2.tar && cmp $W/bb.tar $W/bb2.tar'
t img.save-tty ns "refuses to write a tar to a terminal, or writes" INFO 'script -qc "$PB save $B" /dev/null | head -c 100 | wc -c'
t img.rmi ns "rc 0, busybox gone" '[ $rc -eq 0 ] && ! has "$out" busybox' '$PB rmi $B >/dev/null && $PB images'
t img.load-i ns "rc 0, busybox back" '[ $rc -eq 0 ] && has "$out" busybox' '$PB load -i $W/bb.tar && $PB images'
t img.load-run-quiet ns "busybox runs; a quiet run prints nothing on stderr (also the first extraction)" '[ $rc -eq 0 ] && [ "$out" = hi ] && [ -z "$err" ]' '$PB run --rm $B echo hi'
t img.rmi2 ns "rc 0" '[ $rc -eq 0 ]' '$PB rmi $B'
t img.load-stdin ns "rc 0, busybox back" '[ $rc -eq 0 ] && has "$out" busybox' '$PB load < $W/bb2.tar && $PB images'
t img.load-run.ul ul "busybox runs on ul" '[ $rc -eq 0 ] && [ "$out" = hi ]' '$X $PB run --rm $B echo hi'
t img.load-garbage ns "rc != 0, names the input" '[ $rc -ne 0 ] && [ $rc -ne 124 ]' 'head -c 4096 /dev/urandom > $W/junk.tar; $PB load -i $W/junk.tar'
# import: the rootfs of busybox, as a plain tar.
t img.cp-r-hardlinks ns "cp -r of busybox /bin keeps the hard links: under 10 MiB on the host" '[ $rc -eq 0 ] && [ "${out:-999}" -lt 10 ]' 'rm -rf $W/bbcp; $PB cp -r $B:/bin $W/bbcp >/dev/null 2>&1 && du -sm $W/bbcp | cut -f1; rm -rf $W/bbcp'
t img.import ns "rc 0, listed" '[ $rc -eq 0 ] && has "$out" qa/imported' 'rm -rf $W/bbroot; mkdir -p $W/bbroot/bin $W/bbroot/tmp && $PB cp $B:/bin/busybox $W/bbroot/bin/busybox >/dev/null && for a in sh echo; do ln -s busybox $W/bbroot/bin/$a; done && tar -C $W/bbroot -cf $W/bbroot.tar . && gzip -c $W/bbroot.tar > $W/bbroot.tgz && $PB import $W/bbroot.tar qa/imported:1 && $PB images'
t img.import-run ns "echo hi" '[ $rc -eq 0 ] && [ "$out" = hi ]' '$PB run --rm qa/imported:1 /bin/echo hi'
t img.import-run.ul ul "echo hi" '[ $rc -eq 0 ] && [ "$out" = hi ]' '$X $PB run --rm qa/imported:1 /bin/echo hi'
t img.import-sh ns "sh -c works" '[ $rc -eq 0 ] && [ "$out" = hi ]' "\$PB run --rm qa/imported:1 /bin/sh -c 'echo hi'"
t img.import-gz ns "rc 125, names compressed" '[ $rc -ne 0 ] && has "$err" -i compress' '$PB import $W/bbroot.tgz qa/imported:gz'
t img.import-noname ns "imported:latest" '[ $rc -eq 0 ] && has "$out" imported' '$PB import $W/bbroot.tar && $PB images'
t img.rmi-imported ns "rc 0" '[ $rc -eq 0 ]' '$PB rmi qa/imported:1 imported:latest'
t img.extract-diag ns "extract as root with a full id map claims no chown EINVAL" '[ $rc -eq 0 ] && ! has "$err" EINVAL' '$PB extract --force $A'
t img.verify ns "rc 0" '[ $rc -eq 0 ]' '$PB verify'
t img.image-ls ns "same as images" 'has "$out" alpine' '$PB image ls'
t img.image-prune ns "rc 0" '[ $rc -eq 0 ]' '$PB image prune'
t img.pull-bad ns "rc != 0 fast, names the reference" '[ $rc -ne 0 ] && [ $rc -ne 124 ] && has "$err" nosuchrepo' '$PB pull mirror.gcr.io/library/nosuchrepo-qa443:1'
t img.pull-http ns "refusal by name" '[ $rc -ne 0 ] && [ $rc -ne 124 ]' '$PB pull http://mirror.gcr.io/library/alpine:3.20'
note ""
fi

# ---------------------------------------------------------------- run
if on run; then
note "== 2. run flags"
mkdir -p "$W/vol" && echo invol >"$W/vol/in.txt"
printf 'EA=1\n# a comment\n\nEB="two words"\nEC=x=y\n' >"$W/env.list"
for r in ns ul; do
	rm -f "$W/vol/out-$r.txt" "$W/vol/ro-$r.txt"
	t run.true.$r $r "rc 0, stderr empty (quiet default)" '[ $rc -eq 0 ] && [ -z "$err" ]' '$X $PB run --rm $A true'
	t run.true-deb.$r $r "rc 0, stderr empty" '[ $rc -eq 0 ] && [ -z "$err" ]' '$X $PB run --rm $D true'
	t run.true-py.$r $r "rc 0, stderr empty" '[ $rc -eq 0 ] && [ -z "$err" ]' '$X $PB run --rm $PY true'
	t run.echo.$r $r "hello" '[ $rc -eq 0 ] && [ "$out" = hello ]' '$X $PB run --rm $A echo hello'
	t run.exit3.$r $r "rc 3" '[ $rc -eq 3 ]' "\$X \$PB run --rm \$A sh -c 'exit 3'"
	t run.exit3-deb.$r $r "rc 3" '[ $rc -eq 3 ]' "\$X \$PB run --rm \$D bash -c 'exit 3'"
	t run.sigterm.$r $r "rc 143" '[ $rc -eq 143 ]' "\$X \$PB run --rm \$A sh -c 'kill -TERM \$\$'"
	t run.sigkill.$r $r "rc 137" '[ $rc -eq 137 ]' "\$X \$PB run --rm \$A sh -c 'kill -KILL \$\$'"
	t run.sigsegv-deb.$r $r "rc 139" '[ $rc -eq 139 ]' "\$X \$PB run --rm \$D bash -c 'kill -SEGV \$\$'"
	t run.127.$r $r "rc 127" '[ $rc -eq 127 ]' '$X $PB run --rm $A nosuchcmd'
	t run.126.$r $r "rc 126" '[ $rc -eq 126 ]' '$X $PB run --rm $A /etc/alpine-release'
	t run.env.$r $r "bar" '[ "$out" = bar ]' "\$X \$PB run --rm -e FOO=bar \$A sh -c 'echo \$FOO'"
	t run.env-pass.$r $r "hostval (docker -e NAME takes the host value)" '[ "$out" = hostval ]' "FOO=hostval \$X \$PB run --rm -e FOO \$A sh -c 'echo \$FOO'"
	t run.env-unset-pass.$r $r "empty, rc 0 (docker drops an unset -e NAME)" '[ $rc -eq 0 ] && [ -z "$out" ]' "unset QAUNSET; \$X \$PB run --rm -e QAUNSET \$A sh -c 'echo \${QAUNSET:-}'"
	t run.envfile.$r $r "1|two words|x=y" '[ "$out" = "1|two words|x=y" ]' "\$X \$PB run --rm --env-file \$W/env.list \$A sh -c 'echo \"\$EA|\$EB|\$EC\"'"
	t run.envfile-order.$r $r "9 (a later -e wins)" '[ "$out" = 9 ]' "\$X \$PB run --rm --env-file \$W/env.list -e EA=9 \$A sh -c 'echo \$EA'"
	t run.env-path.$r $r "image PATH kept" 'has "$out" /usr/local/bin' "\$X \$PB run --rm \$PY sh -c 'echo \$PATH'"
	t run.env-home.$r $r "HOME=/root" '[ "$out" = /root ]' "\$X \$PB run --rm \$A sh -c 'echo \$HOME'"
	t run.env-leak.$r $r "no host variable leaks (GH_TOKEN, HTTPS_PROXY)" '[ -z "$out" ]' "\$X \$PB run --rm \$A sh -c 'env | grep -E \"^(GH_TOKEN|GITHUB_TOKEN|HTTPS_PROXY|AWS_SECRET_ACCESS_KEY)=\" | cut -d= -f1'"
	t run.workdir.$r $r "/tmp" '[ "$out" = /tmp ]' '$X $PB run --rm -w /tmp $A pwd'
	t run.workdir-new.$r $r "/newdir (docker creates it)" '[ "$out" = /newdir ]' '$X $PB run --rm -w /newdir $A pwd'
	t run.workdir-img.$r $r "image WORKDIR /go" '[ "$out" = /go ]' '$X $PB run --rm $G pwd'
	t run.vol.$r $r "invol, write lands on host" '[ $rc -eq 0 ] && [ "$out" = invol ] && [ -f $W/vol/out-$RG.txt ]' "\$X \$PB run --rm -v \$W/vol:/data \$A sh -c 'cat /data/in.txt; echo new > /data/out-\$0.txt' \$RG"
	t run.vol-ls.$r $r "ls lists in.txt" 'has "$out" in.txt' '$X $PB run --rm -v $W/vol:/data $A ls /data'
	t run.vol-deb.$r $r "invol via glibc" '[ "$out" = invol ]' '$X $PB run --rm -v $W/vol:/data $D cat /data/in.txt'
	t run.vol-ro-read.$r $r "invol" '[ "$out" = invol ]' '$X $PB run --rm -v $W/vol:/data:ro $A cat /data/in.txt'
	t run.vol-ro-write.$r $r "write fails, host file absent" '[ $rc -ne 0 ] && [ $rc -ne 125 ] && [ ! -e $W/vol/ro-$RG.txt ]' "\$X \$PB run --rm -v \$W/vol:/data:ro \$A sh -c 'echo x > /data/ro-\$0.txt' \$RG"
	t run.vol-ro-write-deb.$r $r "write fails" '[ $rc -ne 0 ] && [ $rc -ne 125 ] && [ ! -e $W/vol/ro2-$RG.txt ]' "\$X \$PB run --rm -v \$W/vol:/data:ro \$D bash -c 'echo x > /data/ro2-\$0.txt' \$RG"
	t run.vol-file.$r $r "a file volume reads" '[ "$out" = invol ]' '$X $PB run --rm -v $W/vol/in.txt:/in.txt $A cat /in.txt'
	t run.vol-rel.$r $r "docker refuses a relative host path or takes a named volume" INFO '$X $PB run --rm -v vol:/data $A true'
	t run.vol-wd.$r $r "workdir inside a volume works (docker)" '[ "$out" = /data ]' '$X $PB run --rm -v $W/vol:/data -w /data $A pwd'
	t run.name.$r $r "rc 0; ps -a lists it Exited; logs reads it" '[ $rc -eq 0 ] && has "$out" "qa-$RG-n1" && has "$out" named' '$PB rm -f qa-$RG-n1 >/dev/null 2>&1; $X $PB run --name qa-$RG-n1 $A echo named >/dev/null && $PB ps -a && $PB logs qa-$RG-n1'
	t run.name-dup.$r $r "rc 125, names the name" '[ $rc -eq 125 ] && has "$err" "qa-$RG-n1"' '$X $PB run --name qa-$RG-n1 $A true'
	t run.name-rm.$r $r "rc 0" '[ $rc -eq 0 ]' '$PB rm qa-$RG-n1'
	t run.rm-name.$r $r "--rm --name leaves no record" '[ $rc -eq 0 ] && ! has "$out" "qa-$RG-n2"' '$X $PB run --rm --name qa-$RG-n2 $A true && $PB ps -a'
	t run.label.$r $r "ps --filter label= finds it; inspect shows the label" 'has "$out" "qa-$RG-l1" && has "$out" team' "\$PB rm -f qa-\$RG-l1 >/dev/null 2>&1; \$X \$PB run --name qa-\$RG-l1 --label team=qa \$A true && \$PB ps -a --filter label=team=qa --format '{{.Names}}' && \$PB inspect -f '{{.Labels}}' qa-\$RG-l1; \$PB rm qa-\$RG-l1 >/dev/null"
	t run.user.$r $r "1000 1000" '[ "$out" = "1000 1000" ]' "\$X \$PB run --rm -u 1000:1000 \$A sh -c 'echo \$(id -u) \$(id -g)'"
	t run.user-name.$r $r "nobody" '[ "$out" = nobody ]' '$X $PB run --rm -u nobody $A id -un'
	if [ $r = ul ]; then
		# The userland rung fakes the identity (TODO/interpose.md T-0711):
		# the kernel still writes as the caller, and podvm says so.
		t run.user-deny.$r $r "documented: -u is faked here, and stderr says so" '[ $rc -eq 0 ] && has "$err" "faking the identity"' "\$X \$PB run --rm -u 1000 \$A sh -c 'echo x > /root/qa'"
	else
		t run.user-deny.$r $r "a write to /root as uid 1000 fails" '[ $rc -ne 0 ]' "\$X \$PB run --rm -u 1000 \$A sh -c 'echo x > /root/qa'"
	fi
	t run.user-node.$r $r "user node writes its home" '[ $rc -eq 0 ] && [ "$out" = node ]' "\$X \$PB run --rm -u node \$N sh -c 'touch /home/node/x && id -un'"
	t run.user-entrypoint.$r $r "-u 1000 with an image ENTRYPOINT: 1000" '[ "$out" = 1000 ]' '$X $PB run --rm -u 1000 $N id -u'
	t run.nss-hosts.$r $r "--add-host resolves through getent (glibc)" 'has "$out" "10.1.2.3"' '$X $PB run --rm --add-host qafoo:10.1.2.3 $D getent hosts qafoo'
	t run.nss-hosts-musl.$r $r "--add-host resolves through getent (musl)" 'has "$out" "10.1.2.3"' '$X $PB run --rm --add-host qafoo:10.1.2.3 $A getent hosts qafoo'
	t run.nss-passwd.$r $r "uid 1000 of the image is node, not a host user" 'has "$out" "^node:"' '$X $PB run --rm $N getent passwd 1000'
	t run.nss-passwd-alpine.$r $r "alpine has no uid 1000: empty" '[ -z "$out" ]' '$X $PB run --rm $A getent passwd 1000'
	t run.node-readdir.$r $r "node readdir of /etc lists the image (debian_version)" 'has "$out" debian_version' "\$X \$PB run --rm \$N node -e 'console.log(require(\"fs\").readdirSync(\"/etc\").join(\" \"))'"
	t run.node-readdir-tmp.$r $r "node readdir of a new /tmp dir: [a]" 'has "$out" "a"' "\$X \$PB run --rm \$N node -e 'const f=require(\"fs\"); f.mkdirSync(\"/tmp/p\"); f.writeFileSync(\"/tmp/p/a\",\"\"); console.log(f.readdirSync(\"/tmp/p\"))'"
	t run.tty-term.$r $r "-t under a terminal: a pty in the payload" 'has "$out" "/dev/pts"' 'script -qc "$X $PB run --rm -t $A tty" /dev/null'
	t run.shm.$r $r "/dev/shm exists and is writable (docker)" '[ $rc -eq 0 ]' "\$X \$PB run --rm \$A sh -c 'echo x > /dev/shm/qa\$\$ && rm /dev/shm/qa\$\$'"
	t run.user-bad.$r $r "rc 125 (docker: unable to find user)" '[ $rc -ne 0 ]' '$X $PB run --rm -u nosuchuser $A true'
	t run.entrypoint.$r $r "hi" '[ "$out" = hi ]' '$X $PB run --rm --entrypoint /bin/echo $A hi'
	t run.entrypoint-py.$r $r "Python 3.12" 'has "$out" "Python 3.12"' '$X $PB run --rm --entrypoint python3 $PY --version'
	t run.cmd-default.$r $r "python image Cmd runs python3 (stdin empty): rc 0" '[ $rc -eq 0 ]' '$X $PB run --rm $PY'
	t run.stdin.$r $r "abc" '[ "$out" = abc ]' "printf 'abc\\n' | \$X \$PB run --rm -i \$A cat"
	t run.stdin-deb.$r $r "abc" '[ "$out" = abc ]' "printf 'abc\\n' | \$X \$PB run --rm -i \$D cat"
	t run.stdin-noi.$r $r "docker without -i: empty stdin" INFO "printf 'abc\\n' | \$X \$PB run --rm \$A cat"
	t run.stdin-big.$r $r "1048576 bytes through" '[ "$out" = 1048576 ]' "head -c 1048576 /dev/zero | \$X \$PB run --rm -i \$A wc -c"
	t run.streams.$r $r "stdout=out, err on stderr only" '[ "$out" = out ] && has "$err" err1' "\$X \$PB run --rm \$A sh -c 'echo out; echo err1 >&2'"
	t run.streams-pipe.$r $r "pipe gets payload bytes only" '[ "$out" = 2 ]' "\$X \$PB run --rm \$A sh -c 'echo a; echo b' | wc -l"
	t run.timeout.$r $r "rc 137 within ~3 s" '[ $rc -eq 137 ] && [ $ms -lt 10000 ]' '$X $PB run --rm --timeout 2 $A sleep 30'
	t run.timeout-tree.$r $r "the tree dies: rc 137, no orphan sleep" '[ $rc -eq 137 ] && ! pgrep -f "sleep 4431" >/dev/null' "\$X \$PB run --rm --timeout 2 \$A sh -c 'sleep 4431 & sleep 4431; wait'"
	t run.timeout-bad.$r $r "rc 125" '[ $rc -eq 125 ]' '$X $PB run --rm --timeout 0 $A true'
	t run.readonly.$r $r "touch fails: Read-only file system" '[ $rc -ne 0 ] && [ $rc -ne 125 ] && has "$err" -i "read-only"' "\$X \$PB run --rm --read-only \$A touch /qa-ro"
	t run.readonly-deb.$r $r "touch fails: Read-only file system" '[ $rc -ne 0 ] && [ $rc -ne 125 ] && has "$err" -i "read-only"' "\$X \$PB run --rm --read-only \$D touch /qa-ro"
	t run.readonly-tmpfs.$r $r "x" '[ "$out" = x ]' "\$X \$PB run --rm --read-only --tmpfs /scratch \$A sh -c 'echo x > /scratch/f && cat /scratch/f'"
	t run.readonly-ro-slash.$r $r "a :ro volume with a trailing / stays read-only under --read-only (T-1804)" '[ $rc -ne 0 ] && [ "$(cat $W/rovol-$RG/f)" = orig ]' "mkdir -p \$W/rovol-\$RG && echo orig > \$W/rovol-\$RG/f; \$X \$PB run --rm --read-only -v \$W/rovol-\$RG:/data/:ro \$A sh -c 'echo CHANGED > /data/f'"
	t run.tmpfs.$r $r "0 then y" '[ "$out" = "0
y" ]' "\$X \$PB run --rm --tmpfs /scratch \$A sh -c 'ls -A /scratch | wc -l; echo y > /scratch/y; cat /scratch/y'"
	t run.tmpfs-tmp.$r $r "/tmp empty and writable" '[ $rc -eq 0 ]' "\$X \$PB run --rm --tmpfs /tmp \$A sh -c 'echo z > /tmp/z && cat /tmp/z'"
	t run.ulimit.$r $r "256" '[ "$out" = 256 ]' "\$X \$PB run --rm --ulimit nofile=256:256 \$A sh -c 'ulimit -n'"
	t run.ulimit-bad.$r $r "rc 125" '[ $rc -eq 125 ]' '$X $PB run --rm --ulimit bogus=1 $A true'
	t run.addhost.$r $r "10.1.2.3 foo" 'has "$out" "10.1.2.3"' '$X $PB run --rm --add-host foo:10.1.2.3 $A grep foo /etc/hosts'
	t run.resolv.$r $r "getent of a name works" '[ $rc -eq 0 ]' '$X $PB run --rm $D getent hosts localhost'
	t run.proc.$r $r "/proc/self/status readable (docker)" '[ $rc -eq 0 ] && has "$out" Name' '$X $PB run --rm $A head -2 /proc/self/status'
	t run.ps.$r $r "ps lists processes (docker)" '[ $rc -eq 0 ]' '$X $PB run --rm $A ps'
	t run.nproc.$r $r "nproc > 0" '[ $rc -eq 0 ] && [ "${out:-0}" -gt 0 ]' '$X $PB run --rm $D nproc'
	t run.devnull.$r $r "write to /dev/null, read zero" '[ "$out" = 4 ]' "\$X \$PB run --rm \$A sh -c 'echo x > /dev/null; head -c 4 /dev/zero | wc -c'"
	t run.urandom.$r $r "2000000 bytes from /dev/urandom (a device does not end)" '[ "$out" = 2000000 ]' "\$X \$PB run --rm \$A sh -c 'head -c 2000000 /dev/urandom | wc -c'"
	t run.tty.$r $r "-t allocates a terminal (docker)" '[ $rc -eq 0 ] && has "$out" tty' "\$X \$PB run --rm -t \$A sh -c 'test -t 1 && echo tty'"
	t run.capdrop.$r $r "rc 0" '[ $rc -eq 0 ]' '$X $PB run --rm --cap-drop ALL $A true'
	t run.capdrop-named.$r $r "rc 125 by name" '[ $rc -eq 125 ]' '$X $PB run --rm --cap-drop NET_RAW $A true'
	t run.nnp.$r $r "rc 0" '[ $rc -eq 0 ]' '$X $PB run --rm --security-opt no-new-privileges $A true'
	t run.secopt-bad.$r $r "rc 125 by name" '[ $rc -eq 125 ]' '$X $PB run --rm --security-opt seccomp=unconfined $A true'
	t run.pull-never.$r $r "rc 125, names the image" '[ $rc -eq 125 ] && has "$err" nosuchimg' '$X $PB run --rm --pull never mirror.gcr.io/library/nosuchimg:1 true'
	if [ $r = ul ]; then
		# Experiment 356, clause 6 (T-1415): where chroot is denied, the
		# userland rung is the best rung, so --strict warns and runs, and
		# --strict=all refuses (TODO/cli.md T-1766 corrects this row).
		t run.strict.$r $r "rc 0 and a substitution warning: the best rung here (experiment 356, clause 6)" '[ $rc -eq 0 ] && has "$err" "warning \[substitution\]"' '$X $PB run --rm --strict $A true'
		t run.strict-all.$r $r "rc 125: --strict=all refuses the userland rung" '[ $rc -eq 125 ]' '$X $PB run --rm --strict=all $A true'
	else
		t run.strict.$r $r "rc 0 on the namespace rung" '[ $rc -eq 0 ]' '$X $PB run --rm --strict $A true'
	fi
	t run.verbose.$r $r "banner on stderr names the rung" 'has "$err" "mode="' '$X $PB run --rm --verbose $A true'
	t run.quiet-verbose.$r $r "stderr empty" '[ -z "$err" ]' '$X $PB run --rm -q --verbose $A true'
	t run.platform-arm.$r $r "docker: runs arm64 under binfmt, or a named refusal" INFO '$X $PB run --rm --platform arm64 $A uname -m'
	t run.report.$r $r "rc 3; report valid against the schema; exit_code 3" '[ $rc -eq 3 ] && python3 -I -c "import json,jsonschema,sys; d=json.load(open(sys.argv[1])); jsonschema.validate(d,json.load(open(sys.argv[2]))); assert d[\"exit_code\"]==3, d" $W/rep-$RG.json '"$REPO"'/crates/podvm/schemas/run-report.v1.schema.json' "rm -f \$W/rep-\$RG.json; \$X \$PB run --rm --label k=v --ulimit nofile=512 --timeout 30 --report \$W/rep-\$RG.json \$A sh -c 'exit 3'"
	t run.report-sig.$r $r "signal 15, exit_code 143; valid" 'python3 -I -c "import json,jsonschema,sys; d=json.load(open(sys.argv[1])); jsonschema.validate(d,json.load(open(sys.argv[2]))); assert d[\"signal\"]==15 and d[\"exit_code\"]==143, d" $W/rep2-$RG.json '"$REPO"'/crates/podvm/schemas/run-report.v1.schema.json' "rm -f \$W/rep2-\$RG.json; \$X \$PB run --rm --report \$W/rep2-\$RG.json \$A sh -c 'kill -TERM \$\$'"
	t run.report-refused.$r $r "rc 125; report with refused 125; valid" '[ $rc -eq 125 ] && python3 -I -c "import json,jsonschema,sys; d=json.load(open(sys.argv[1])); jsonschema.validate(d,json.load(open(sys.argv[2]))); assert d[\"refused\"]==125, d" $W/rep3-$RG.json '"$REPO"'/crates/podvm/schemas/run-report.v1.schema.json' "rm -f \$W/rep3-\$RG.json; \$X \$PB run --rm --pull never --report \$W/rep3-\$RG.json mirror.gcr.io/library/nosuchimg:1 true"
	t run.report-timeout.$r $r "timed_out true; valid" 'python3 -I -c "import json,jsonschema,sys; d=json.load(open(sys.argv[1])); jsonschema.validate(d,json.load(open(sys.argv[2]))); assert d[\"timed_out\"] is True, d" $W/rep4-$RG.json '"$REPO"'/crates/podvm/schemas/run-report.v1.schema.json' "rm -f \$W/rep4-\$RG.json; \$X \$PB run --rm --timeout 1 --report \$W/rep4-\$RG.json \$A sleep 20"
	t run.report-env.$r $r "no -e value in the report" '[ $rc -eq 0 ] && ! grep -q s3cr3tvalue $W/rep5-$RG.json' "rm -f \$W/rep5-\$RG.json; \$X \$PB run --rm -e QA=s3cr3tvalue --report \$W/rep5-\$RG.json \$A true"
	t run.report-payload.$r $r "a --report after the image is the payload's (T-1801)" '[ $rc -eq 0 ] && [ "$(cat $W/keep-$RG.txt)" = keep ]' "echo keep > \$W/keep-\$RG.txt; \$X \$PB run --rm \$A true --report \$W/keep-\$RG.txt"
	t run.report-refused-env.$r $r "a refused run writes no -e value (T-1802)" '[ $rc -eq 125 ] && ! grep -q s3cr3tvalue $W/rep6-$RG.json' "rm -f \$W/rep6-\$RG.json; \$X \$PB run --rm --pull never -e QA=s3cr3tvalue --report \$W/rep6-\$RG.json mirror.gcr.io/library/nosuchimg:1 true"
	# --podvm-confine: each case has a bound of 60 s; a hang reads rc=124.
	TO=60 t run.confine-rootread.$r $r "reads /etc, writes /tmp, not /root" 'has "$out" 3.20 && has "$out" wrote && has "$err" -i denied' "\$X \$PB run --rm --podvm-confine 'read=/;write=/tmp' \$A sh -c 'cat /etc/alpine-release; echo w>/tmp/w && echo wrote; echo x>/root/x'"
	TO=60 t run.confine-deny.$r $r "cat /etc fails EACCES fast, no hang" '[ $rc -ne 0 ] && [ $rc -ne 124 ] && has "$err" -i denied' "\$X \$PB run --rm --podvm-confine 'read=/usr;write=/tmp' \$A cat /etc/alpine-release"
	TO=60 t run.confine-ls-root.$r $r "ls / fails EACCES fast, no hang" '[ $rc -ne 0 ] && [ $rc -ne 124 ]' "\$X \$PB run --rm --podvm-confine 'read=/usr;write=/tmp' \$A ls /"
	TO=60 t run.confine-host-root.$r $r "a host path outside the rootfs is not read" '[ $rc -ne 0 ] && [ $rc -ne 124 ]' "\$X \$PB run --rm --podvm-confine 'read=/;write=/tmp' \$A ls \$W"
	TO=60 t run.confine-deb.$r $r "bash: denied fast" '[ $rc -ne 0 ] && [ $rc -ne 124 ] && has "$err" -i denied' "\$X \$PB run --rm --podvm-confine 'read=/usr;write=/tmp' \$D bash -c 'cat /etc/debian_version'"
	TO=60 t run.confine-py.$r $r "python: PermissionError on /etc/hostname" '[ $rc -eq 1 ] && has "$err" PermissionError' "\$X \$PB run --rm --podvm-confine 'read=/usr;write=/tmp' \$PY python3 -c 'open(\"/etc/hostname\").read()'"
	TO=60 t run.confine-py-ok.$r $r "python prints 1" '[ "$out" = 1 ]' "\$X \$PB run --rm --podvm-confine 'read=/;write=/tmp' \$PY python3 -c 'import ssl,sqlite3,json; print(1)'"
	TO=60 t run.confine-node.$r $r "node prints 1" '[ "$out" = 1 ]' "\$X \$PB run --rm --podvm-confine 'read=/;write=/tmp' \$N node -e 'console.log(1)'"
	TO=60 t run.confine-node-deny.$r $r "node: EACCES on /etc/hostname, fast" '[ $rc -ne 0 ] && [ $rc -ne 124 ]' "\$X \$PB run --rm --podvm-confine 'read=/usr;write=/tmp' \$N node -e 'require(\"fs\").readFileSync(\"/etc/hostname\")'"
	TO=60 t run.confine-devnull.$r $r "a redirect to /dev/null with /dev not in write: rc 0 or EACCES, no hang" '[ $rc -ne 124 ]' "\$X \$PB run --rm --podvm-confine 'read=/;write=/tmp' \$A sh -c 'echo x >/dev/null; echo done'"
	TO=60 t run.confine-vol.$r $r "a -v path in read: invol" '[ "$out" = invol ]' "\$X \$PB run --rm -v \$W/vol:/data --podvm-confine 'read=/data;write=/tmp' \$A cat /data/in.txt"
	TO=60 t run.confine-stdin.$r $r "piped stdin reaches a confined payload" '[ "$out" = abc ]' "printf 'abc\\n' | \$X \$PB run --rm -i --podvm-confine 'read=/;write=/tmp' \$A cat"
	TO=60 t run.confine-setsid.$r $r "a unit with no terminal (setsid, stdin /dev/null) runs" '[ "$out" = ok ]' "setsid -w \$X \$PB run --rm --podvm-confine 'read=/;write=/tmp' \$A echo ok </dev/null"
	TO=60 t run.confine-errand.$r $r "profile errand runs echo" '[ "$out" = ok ]' '$X $PB run --rm --profile errand $A echo ok'
	TO=60 t run.confine-errand-py.$r $r "profile errand runs python" '[ "$out" = 1 ]' "\$X \$PB run --rm --profile errand \$PY python3 -c 'print(1)'"
	TO=60 t run.confine-exec-only.$r $r "exec=/bin only: ls runs" INFO "\$X \$PB run --rm --podvm-confine 'exec=/bin' \$A ls /bin/ls"
	# python's urllib, not busybox wget: wget sends an absolute GET for an
	# https URL to a proxy, and the proxy of the cloud host answers 405
	# with or without --podvm-confine.
	TO=60 t run.confine-connect.$r $r "connect=443 and the proxy port: https works" '[ "$out" = 200 ]' "\$X \$PB run --rm \$PX --podvm-confine \"read=/;write=/tmp;connect=443,\$PPORT\" \$PY python3 -c 'import urllib.request; print(urllib.request.urlopen(\"https://pypi.org/simple/six/\").status)'"
	TO=60 t run.confine-connect-deny.$r $r "connect=1 only: the proxy port is denied, fast" '[ $rc -ne 0 ] && [ $rc -ne 124 ]' "\$X \$PB run --rm \$PX --podvm-confine 'read=/;write=/tmp;connect=1' \$A wget -q -T 5 -O /dev/null https://pypi.org/simple/six/"
	TO=60 t run.confine-bad.$r $r "rc 125, names the key" '[ $rc -eq 125 ] && has "$err" foo' "\$X \$PB run --rm --podvm-confine 'foo=/' \$A true"
	TO=60 t run.confine-detached.$r $r "-d with confine: logs show the denial, wait gives 1" 'has "$out" -i denied' "\$PB rm -f qa-\$RG-cf >/dev/null 2>&1; \$X \$PB run -d --name qa-\$RG-cf --podvm-confine 'read=/usr;write=/tmp' \$A sh -c 'cat /etc/alpine-release; echo end' >/dev/null && \$PB wait qa-\$RG-cf; \$PB logs qa-\$RG-cf; \$PB rm -f qa-\$RG-cf >/dev/null"
	TO=60 t run.confine-sleep.$r $r "a confined sleep 1 ends" '[ $rc -eq 0 ] && [ $ms -lt 20000 ]' "\$X \$PB run --rm --podvm-confine 'read=/;write=/tmp' \$D sleep 1"
	TO=60 t run.confine-threads.$r $r "a confined python thread pool ends" '[ "$out" = 45 ]' "\$X \$PB run --rm --podvm-confine 'read=/;write=/tmp' \$PY python3 -c 'from concurrent.futures import ThreadPoolExecutor as T; print(sum(T(4).map(int, range(10))))'"
	TO=60 t run.confine-subproc.$r $r "a confined subprocess ends" '[ "$out" = hi ]' "\$X \$PB run --rm --podvm-confine 'read=/;write=/tmp' \$PY python3 -c 'import subprocess; print(subprocess.check_output([\"echo\",\"hi\"]).decode().strip())'"
done
note ""
fi

# ---------------------------------------------------------------- imgs
if on imgs; then
note "== 3. images"
for r in ns ul; do
	t imgs.alpine.$r $r "Alpine" 'has "$out" Alpine' '$X $PB run --rm $A head -1 /etc/os-release'
	t imgs.debian.$r $r "bash 5" 'has "$out" "^5"' "\$X \$PB run --rm \$D bash -c 'echo \$BASH_VERSION'"
	t imgs.ubuntu.$r $r "Ubuntu 24.04" 'has "$out" "24.04"' '$X $PB run --rm $U cat /etc/os-release'
	t imgs.python.$r $r "1, stderr empty" '[ "$out" = 1 ] && [ -z "$err" ]' "\$X \$PB run --rm \$PY python3 -c 'import ssl,sqlite3,json; print(1)'"
	t imgs.python-mp.$r $r "multiprocessing pool works (docker)" '[ "$out" = 285 ]' "\$X \$PB run --rm \$PY python3 -c 'import multiprocessing as m; print(sum(m.Pool(2).map(abs, [i*i for i in range(10)])))'"
	t imgs.python-venv.$r $r "venv creates" '[ $rc -eq 0 ]' "\$X \$PB run --rm \$PY sh -c 'python3 -m venv /tmp/v && /tmp/v/bin/python -c \"print(2)\"'"
	t imgs.node.$r $r "1" '[ "$out" = 1 ]' "\$X \$PB run --rm \$N node -e 'console.log(1)'"
	t imgs.node-fs.$r $r "a file round trip" '[ "$out" = ok ]' "\$X \$PB run --rm \$N node -e 'const f=require(\"fs\"); f.writeFileSync(\"/tmp/a\",\"ok\"); console.log(f.readFileSync(\"/tmp/a\",\"utf8\"))'"
	t imgs.node-child.$r $r "child_process works" '[ "$out" = hi ]' "\$X \$PB run --rm \$N node -e 'console.log(require(\"child_process\").execSync(\"echo hi\").toString().trim())'"
	TO=300 t imgs.go-version.$r $r "go1.24" 'has "$out" go1.24' '$X $PB run --rm $G go version'
	TO=400 t imgs.go-build.$r $r "hello-go twice" '[ "$out" = "hello-go
hello-go" ]' "\$X \$PB run --rm \$G sh -c 'mkdir -p /tmp/h && cd /tmp/h && printf \"package main\\nimport \\\"fmt\\\"\\nfunc main(){fmt.Println(\\\"hello-go\\\")}\\n\" > m.go && go run m.go && go build -o /tmp/h/h m.go && /tmp/h/h'"
	t imgs.busybox.$r $r "hi" '[ "$out" = hi ]' '$X $PB run --rm $B busybox echo hi'
	t imgs.busybox-sh.$r $r "hi" '[ "$out" = hi ]' "\$X \$PB run --rm \$B sh -c 'echo hi'"
	t imgs.busybox-ls.$r $r "ls / lists bin" 'has "$out" bin' '$X $PB run --rm $B ls /'
done
note ""
fi

# ---------------------------------------------------------------- tools
if on tools; then
note "== 3b. tools inside the images (network through the proxy of the host)"
for r in ns ul; do
	TO=400 t tools.apt-update-deb.$r $r "rc 0" '[ $rc -eq 0 ]' '$X $PB run --rm $PX $D apt-get update -q'
	TO=600 t tools.apt-install-ubu.$r $r "curl installs and fetches https" 'has "$out" "HTTP/"' "\$X \$PB run --rm \$PX \$U sh -c 'apt-get update -q >/dev/null && apt-get install -y -q --no-install-recommends curl ca-certificates >/dev/null && curl -sSI https://pypi.org/simple/ | head -1'"
	TO=300 t tools.apk.$r $r "apk add curl, no trigger error" 'has "$out" curl && ! has "$err" ERROR' "\$X \$PB run --rm \$PX \$A sh -c 'apk add -q --no-cache curl && curl --version | head -1'"
	TO=300 t tools.pip.$r $r "six imports" 'has "$out" "^1\."' "\$X \$PB run --rm \$PX \$PY sh -c 'pip install -q --no-cache-dir --root-user-action=ignore six && python3 -c \"import six; print(six.__version__)\"'"
	TO=300 t tools.npm.$r $r "true" '[ "$(printf "%s" "$out" | tail -1)" = true ]' "\$X \$PB run --rm \$PX \$N sh -c 'mkdir -p /tmp/p && cd /tmp/p && npm init -y >/dev/null && npm install --silent is-number >/dev/null && node -e \"console.log(require(\\\"is-number\\\")(5))\"'"
	TO=300 t tools.git-clone.$r $r "README listed" 'has "$out" README' "\$X \$PB run --rm \$PX \$G sh -c 'git clone -q --depth 1 https://github.com/octocat/Hello-World /tmp/hw && ls /tmp/hw'"
	TO=120 t tools.curl-https.$r $r "HTTP/ 200" 'has "$out" "200"' "\$X \$PB run --rm \$PX \$G curl -sS -o /dev/null -w '%{http_code}' https://pypi.org/simple/six/"
	TO=300 t tools.go-mod.$r $r "go get of a module" '[ $rc -eq 0 ]' "\$X \$PB run --rm \$PX \$G sh -c 'mkdir -p /tmp/m && cd /tmp/m && go mod init qa/m >/dev/null 2>&1 && go get golang.org/x/text@v0.14.0'"
	TO=120 t tools.py-https.$r $r "200" '[ "$out" = 200 ]' "\$X \$PB run --rm \$PX \$PY python3 -c 'import urllib.request as u; print(u.urlopen(\"https://pypi.org/simple/six/\").status)'"
	TO=120 t tools.node-https.$r $r "200" '[ "$out" = 200 ]' "\$X \$PB run --rm \$PX -e NODE_USE_ENV_PROXY=1 \$N node -e 'fetch(\"https://pypi.org/simple/six/\").then(r=>console.log(r.status))'"
done
note ""
fi

# ---------------------------------------------------------------- life
if on life; then
note "== 4. lifecycle"
echo cpin >"$W/cpin.txt"
for r in ns ul; do
	n=qa-$r-d
	"$PB" rm -f $n qa-$r-c qa-$r-p >/dev/null 2>&1
	t life.run-d.$r $r "a 64-hex id" 'printf "%s" "$out" | grep -Eq "^[0-9a-f]{64}$"' "\$X \$PB run -d --name $n \$A sh -c 'echo start; while :; do sleep 1; done'"
	t life.ps.$r $r "lists $n Up" 'has "$out" "$n" && has "$out" Up' '$X $PB ps'
	t life.ps-q.$r $r "ids only" '[ $rc -eq 0 ] && ! has "$out" NAMES' '$X $PB ps -q'
	t life.ps-format.$r $r "name and status" 'has "$out" "$n"' "\$X \$PB ps --format '{{.Names}} {{.Status}}'"
	t life.ps-filter.$r $r "the name only" 'has "$out" "$n"' "\$X \$PB ps --filter name=$n --format '{{.Names}}'"
	t life.logs.$r $r "start" 'has "$out" start' "\$X \$PB logs $n"
	TO=4 t life.logs-f.$r $r "start, follows until the bound (rc 124)" 'has "$out" start && [ $rc -eq 124 ]' "\$X \$PB logs -f $n"
	t life.logs-tail.$r $r "one line" '[ "$out" = start ]' "\$X \$PB logs --tail 1 $n"
	t life.exec-rc.$r $r "rc 4" '[ $rc -eq 4 ]' "\$X \$PB exec $n sh -c 'exit 4'"
	t life.exec-e-w.$r $r "1 /tmp" '[ "$out" = "1 /tmp" ]' "\$X \$PB exec -e QX=1 -w /tmp $n sh -c 'echo \$QX \$(pwd)'"
	t life.exec-env-run.$r $r "exec sees the -e of run (docker)" '[ "$out" = runval ]' "\$PB rm -f qa-$r-e >/dev/null 2>&1; \$X \$PB run -d --name qa-$r-e -e RV=runval \$A sh -c 'while :; do sleep 1; done' >/dev/null && \$X \$PB exec qa-$r-e sh -c 'echo \$RV'; \$PB rm -f qa-$r-e >/dev/null"
	t life.exec-sees-run-fs.$r $r "a file the payload wrote is seen by exec" '[ "$out" = mark ]' "\$PB rm -f qa-$r-f >/dev/null 2>&1; \$X \$PB run -d --name qa-$r-f \$A sh -c 'echo mark > /tmp/mark; while :; do sleep 1; done' >/dev/null && sleep 1 && \$X \$PB exec qa-$r-f cat /tmp/mark; \$PB rm -f qa-$r-f >/dev/null"
	t life.exec-u.$r $r "1000" '[ "$out" = 1000 ]' "\$X \$PB exec -u 1000 $n id -u"
	t life.exec-stdin.$r $r "abc" '[ "$out" = abc ]' "printf 'abc\\n' | \$X \$PB exec -i $n cat"
	t life.cp-in.$r $r "cpin inside" '[ "$out" = cpin ]' "\$X \$PB cp \$W/cpin.txt $n:/tmp/cpin.txt >/dev/null && \$X \$PB exec $n cat /tmp/cpin.txt"
	t life.cp-out.$r $r "3.20.x on the host" 'has "$out" "^3.20"' "rm -f \$W/cpout-$r; \$X \$PB cp $n:/etc/alpine-release \$W/cpout-$r >/dev/null && cat \$W/cpout-$r"
	t life.cp-stdout.$r $r "docker cp prints nothing on stdout" '[ -z "$out" ]' "\$X \$PB cp \$W/cpin.txt $n:/tmp/cpin2.txt"
	t life.cp-dir.$r $r "-r of a directory" '[ $rc -eq 0 ]' "rm -rf \$W/cpdir-$r; \$X \$PB cp -r $n:/etc/apk \$W/cpdir-$r >/dev/null && ls \$W/cpdir-$r"
	t life.cp-dash.$r $r "docker cp - (tar stream) or a named refusal" INFO "\$X \$PB cp $n:/etc/alpine-release - | head -c 200"
	t life.stop.$r $r "rc 0, prints the name, under 12 s" '[ $rc -eq 0 ] && [ "$out" = "$n" ] && [ $ms -lt 12000 ]' "\$X \$PB stop $n"
	t life.ps-a.$r $r "Exited (143)" 'has "$out" "Exited (143)"' "\$X \$PB ps -a --filter name=$n"
	t life.wait-exited.$r $r "143" '[ "$out" = 143 ]' "\$X \$PB wait $n"
	t life.start.$r $r "rc 0, Up again" '[ $rc -eq 0 ] && has "$out" Up' "\$X \$PB start $n >/dev/null && \$X \$PB ps --filter name=$n"
	t life.restart.$r $r "rc 0, prints the name once" '[ $rc -eq 0 ] && [ "$out" = "$n" ]' "\$X \$PB restart $n"
	t life.kill.$r $r "rc 0; wait gives 137" '[ $rc -eq 0 ] && has "$out" 137' "\$X \$PB kill $n && \$X \$PB wait $n"
	t life.kill-s.$r $r "kill -s USR1 of a running one: wait 138" 'has "$out" 138' "\$X \$PB start $n >/dev/null && \$X \$PB kill -s USR1 $n && \$X \$PB wait $n"
	t life.kill-bad.$r $r "rc != 0, names the signal" '[ $rc -ne 0 ] && has "$err" -i bogus' "\$X \$PB start $n >/dev/null; \$X \$PB kill -s BOGUS $n"
	t life.rm-running.$r $r "rc != 0: refuses a running one" '[ $rc -ne 0 ]' "\$X \$PB rm $n"
	t life.rm-f.$r $r "rc 0, gone" '[ $rc -eq 0 ] && ! has "$out" "$n"' "\$X \$PB rm -f $n >/dev/null && \$X \$PB ps -a"
	t life.exec-gone.$r $r "rc != 0, names it" '[ $rc -ne 0 ] && has "$err" "$n"' "\$X \$PB exec $n true"
	t life.create-start.$r $r "create, start, wait 0, logs hi" '[ "$out" = "0
hi" ]' "\$X \$PB create --name qa-$r-c \$A echo hi >/dev/null && \$X \$PB start qa-$r-c >/dev/null && \$X \$PB wait qa-$r-c && \$X \$PB logs qa-$r-c; \$PB rm qa-$r-c >/dev/null"
	t life.create-ps.$r $r "a created one reads Created" 'has "$out" Created' "\$X \$PB create --name qa-$r-c2 \$A true >/dev/null && \$X \$PB ps -a --filter name=qa-$r-c2; \$PB rm qa-$r-c2 >/dev/null"
	t life.detached-py.$r $r "a detached python payload logs up" 'has "$out" up' "\$X \$PB run -d --name qa-$r-p \$PY python3 -u -c 'import time; print(\"up\"); time.sleep(30)' >/dev/null && sleep 3 && \$X \$PB logs qa-$r-p; \$PB rm -f qa-$r-p >/dev/null"
	t life.detached-exit.$r $r "a detached exit 5: wait 5, ps Exited (5)" 'has "$out" "^5" && has "$out" "Exited (5)"' "\$PB rm -f qa-$r-x >/dev/null 2>&1; \$X \$PB run -d --name qa-$r-x \$A sh -c 'exit 5' >/dev/null && \$X \$PB wait qa-$r-x; \$X \$PB ps -a --filter name=qa-$r-x; \$PB rm qa-$r-x >/dev/null"
	t life.stop-t0.$r $r "stop -t 0 kills at once" '[ $rc -eq 0 ] && [ $ms -lt 3000 ]' "\$PB rm -f qa-$r-t >/dev/null 2>&1; \$X \$PB run -d --name qa-$r-t \$A sh -c 'trap \"\" TERM; while :; do sleep 1; done' >/dev/null; \$X \$PB stop -t 0 qa-$r-t; \$PB rm -f qa-$r-t >/dev/null"
	t life.stop-trap.$r $r "stop -t 2 of a TERM-ignoring payload: about 2 s, then 137" '[ $rc -eq 0 ] && [ $ms -lt 6000 ] && has "$out" 137' "\$PB rm -f qa-$r-t2 >/dev/null 2>&1; \$X \$PB run -d --name qa-$r-t2 \$A sh -c 'trap \"\" TERM; while :; do sleep 1; done' >/dev/null; \$X \$PB stop -t 2 qa-$r-t2 >/dev/null; \$X \$PB wait qa-$r-t2; \$PB rm -f qa-$r-t2 >/dev/null"
	t life.inspect-c.$r $r "inspect of a container: JSON" 'python3 -I -c "import json,sys; json.load(sys.stdin)" <"$of"' "\$PB rm -f qa-$r-i >/dev/null 2>&1; \$X \$PB run --name qa-$r-i \$A true; \$X \$PB inspect qa-$r-i; \$PB rm qa-$r-i >/dev/null"
	t life.tmpfs-d.$r $r "-d --tmpfs refused by name (documented)" '[ $rc -eq 125 ]' "\$X \$PB run -d --tmpfs /x \$A true"
done
note ""
fi

# ---------------------------------------------------------------- sys
if on sys; then
note "== 5. system verbs"
t sys.info ns "rc 0" '[ $rc -eq 0 ] && has "$out" rung' '$PB system info'
t sys.info.ul ul "rung userland" '[ $rc -eq 0 ] && has "$out" userland' '$X $PB system info'
t sys.info-short ns "docker info works" '[ $rc -eq 0 ]' '$PB info'
for f in Parity ParityRows ExitCodes Version Rung EnteredRung StrictOk Store Banner Platform Kernel Verbs VerbsNative VerbsDegraded VerbsStub VerbsNone; do
	t sys.info-f.$f ns "a value" '[ $rc -eq 0 ] && [ -n "$out" ]' "\$PB system info --format '{{.$f}}'"
done
t sys.info-json ns "JSON" 'python3 -I -c "import json,sys; json.load(sys.stdin)" <"$of"' "\$PB system info --format '{{json .Parity}}'"
t sys.info-consistent ns "the run row does not say chroot where the rung is namespace" '! has "$out" "enters a chroot, never a namespace"' '$PB system info'
t sys.df ns "rc 0, rows" '[ $rc -eq 0 ] && has "$out" alpine' '$PB system df'
t sys.probe ns "rc 0" '[ $rc -eq 0 ]' '$PB probe'
t sys.probe.ul ul "rc 0, names userland" '[ $rc -eq 0 ] && has "$out$err" userland' '$X $PB probe'
t sys.probe-json ns "JSON" '[ $rc -eq 0 ] && python3 -I -c "import json,sys; json.load(sys.stdin)" <"$of"' '$PB probe --json'
t sys.probe-json.ul ul "JSON" '[ $rc -eq 0 ] && python3 -I -c "import json,sys; json.load(sys.stdin)" <"$of"' '$X $PB probe --json'
t sys.probe-rows ns "rc 0" '[ $rc -eq 0 ]' '$PB probe --rows'
t sys.probe-strict ns "rc 0 on namespace" '[ $rc -eq 0 ]' '$PB probe --strict'
t sys.probe-strict.ul ul "rc != 0 below namespace" '[ $rc -ne 0 ]' '$X $PB probe --strict'
t sys.probe-cached ns "rc 0" '[ $rc -eq 0 ]' '$PB probe --cached'
t sys.doctor ns "rc 0" '[ $rc -eq 0 ]' '$PB doctor'
t sys.doctor.ul ul "rc 0" '[ $rc -eq 0 ]' '$X $PB doctor'
t sys.doctor-strict ns "INFO" INFO '$PB doctor --strict'
t sys.version ns "0.2.0-beta.7" 'has "$out" 0.2.0' '$PB version'
t sys.version-v ns "commit, target, crt-static" 'has "$out" crt-static' '$PB version --verbose'
t sys.version-flag ns "docker --version works" '[ $rc -eq 0 ] && has "$out" 0.2.0' '$PB --version'
t sys.man ns "rc 0, a manual" '[ $rc -eq 0 ] && [ $(printf "%s" "$out" | wc -c) -gt 2000 ]' '$PB man --no-pager'
t sys.man-pipe ns "a pipe needs no --no-pager" '[ $rc -eq 0 ]' '$PB man | head -5'
t sys.help ns "rc 0, usage on stdout" '[ $rc -eq 0 ] && has "$out" usage' '$PB --help'
t sys.noargs ns "usage, rc 0 or 1" INFO '$PB'
for v in run exec probe doctor man profile notebook lab probe-inside compare pull save load import extract images rmi tag image inspect verify create start ps logs stop restart kill wait rm cp system version login logout machine remote windows; do
	t sys.help-$v ns "rc 0, usage on stdout" '[ $rc -eq 0 ] && [ -n "$out" ]' "\$PB $v --help"
done
t sys.help-sub ns "rc 0" '[ $rc -eq 0 ] && [ -n "$out" ]' '$PB system df --help'
t sys.help-h ns "-h as --help" '[ $rc -eq 0 ] && [ -n "$out" ]' '$PB run -h'
for v in "run --foo \$A true" "exec --foo x true" "ps --foo" "images --foo" "logs --foo x" "stop --foo x" "rm --foo x" "pull --foo \$A" "inspect --foo \$A" "cp --foo a b" "create --foo \$A" "start --foo x" "kill --foo x" "wait --foo x" "probe --foo" "system info --foo" "rmi --foo x" "tag --foo a b" "save --foo \$A" "load --foo" "version --foo" "man --foo" "lab plan --foo"; do
	vid=$(echo "$v" | cut -d' ' -f1-2 | tr ' ' '_' | tr -d '$')
	t sys.unknown.$vid ns "rc 125, names --foo, short message" '[ $rc -eq 125 ] && has "$err" -- "--foo"' "\$PB $v"
done
for v in "run --hostname h \$A true" "run --privileged \$A true" "run --network host \$A true" "run --memory 1g \$A true" "run --init \$A true" "run --restart always \$A true" "run --mount type=bind,src=/tmp,dst=/x \$A true" "exec -d x true" "top x" "build ." "network ls" "volume ls" "compose up" "commit x y" "export x"; do
	vid=$(echo "$v" | cut -d' ' -f1-2 | tr ' ' '_' | tr -d '$/.')
	t sys.none.$vid ns "rc 125 by name" '[ $rc -eq 125 ]' "\$PB $v"
done
t sys.unknown-verb ns "rc 1" '[ $rc -eq 1 ]' '$PB frobnicate'
t sys.unknown-verb-out ns "an unknown verb prints a short error, not the whole usage" '[ $(printf "%s" "$err" | wc -l) -lt 10 ]' '$PB frobnicate'
t sys.unknown-flag-short ns "an unknown flag prints a short error, not the whole usage (docker: 2 lines)" '[ $(printf "%s" "$err" | wc -l) -lt 10 ]' '$PB run --foo $A true'
t sys.profile-list ns "rc 0" '[ $rc -eq 0 ]' '$PB profile list'
t sys.profile-cols ns "aligned columns" '[ $(printf "%s\n" "$out" | awk "{print index(\$0, \"built-in\")}" | sort -u | wc -l) -le 2 ]' '$PB profile list'
t sys.docker-name ns "invoked as docker: runs" '[ $rc -eq 0 ] && [ "$out" = hi ]' 'ln -sf $PB $W/docker; $W/docker run --rm $A echo hi'
t sys.docker-name-ps ns "invoked as docker: ps" '[ $rc -eq 0 ]' '$W/docker ps -a'
t sys.podman-name ns "invoked as podman: runs" '[ $rc -eq 0 ] && [ "$out" = hi ]' 'ln -sf $PB $W/podman; $W/podman run --rm $A echo hi'
t sys.compare-help ns "rc 0" '[ $rc -eq 0 ]' '$PB compare --help'
TO=300 t sys.probe-inside ns "rc 0, a report" '[ $rc -eq 0 ] && [ -n "$out" ]' '$PB probe-inside $A'
TO=300 t sys.probe-inside.ul ul "rc 0, a report" '[ $rc -eq 0 ] && [ -n "$out" ]' '$X $PB probe-inside $A'
note ""
fi

# ---------------------------------------------------------------- perf
if on perf; then
note "== 6. performance: median of 5 runs of 'run --rm IMAGE true' (ms)"
median() { sort -n | awk '{a[NR]=$1} END {print a[int((NR+1)/2)]}'; }
for r in ns ul; do
	if [ "$r" = ul ]; then X="$FXCMD"; else X=""; fi
	for i in "$A" "$B" "$D" "$U" "$PY" "$N" "$G"; do
		for extra in "" "--no-source-fixup"; do
			$X "$PB" run --rm $extra "$i" true >/dev/null 2>&1
			ts=""
			for k in 1 2 3 4 5; do
				s=$(date +%s%N); timeout 300 $X "$PB" run --rm $extra "$i" true >/dev/null 2>&1; e=$(date +%s%N)
				ts="$ts $(( (e - s) / 1000000 ))"
			done
			m=$(printf '%s\n' $ts | median)
			cp=$(timeout 300 $X "$PB" run --rm --verbose $extra "$i" true 2>&1 | sed -n 's/.*container.s own rootfs: \(.*\) (TODO.*/\1/p' | head -1)
			note "perf.$(basename "$i") | run --rm $extra IMAGE true | $r | - | median=${m}ms all=[$ts] rootfs=[$cp] | INFO"
		done
	done
done
# The phases of one ubuntu run on ns, from the timestamps of strace.
if command -v strace >/dev/null 2>&1; then
	for extra in "" "--no-source-fixup"; do
		strace -f -tt -e trace=execve,copy_file_range,unlinkat,openat2,connect,sendto,exit_group -o "$W/st-$$.txt" "$PB" run --rm $extra "$U" true >/dev/null 2>&1
		python3 -I - "$W/st-$$.txt" "$extra" <<'EOF' | tee -a "$OUT"
import re, sys
L = [l for l in open(sys.argv[1]).read().splitlines() if re.match(r'\d+ \d', l)]
def ts(l):
    h, m, s = l.split()[1].split(':'); return int(h) * 3600 + int(m) * 60 + float(s)
t0 = ts(L[0])
def span(p):
    xs = [ts(l) - t0 for l in L if re.search(p, l)]
    return "%.3f-%.3f s (%d)" % (xs[0], xs[-1], len(xs)) if xs else "-"
print("perf.phases %s | strace -f, ubuntu true, ns | - | probe self-exec %s; rootfs copy %s; connect %s; payload exec %s; unlink %s; end %.3f s | INFO" % (
    sys.argv[2] or "(default)", span(r'execve\("/proc/self/exe'), span(r'copy_file_range'), span(r'connect\('),
    span(r'execve\("/usr/bin/true'), span(r'unlinkat'), ts(L[-1]) - t0))
for l in L:
    m = re.search(r'sendto\(\d+, "CONNECT ([^ ]+)', l)
    if m: print("perf.https-probe | %s | ns | - | CONNECT %s at %.3f s | INFO" % (sys.argv[2] or "(default)", m.group(1), ts(l) - t0))
EOF
		rm -f "$W/st-$$.txt"
	done
fi
s=$(date +%s%N); "$PB" probe >/dev/null 2>&1; e=$(date +%s%N)
note "perf.probe | podvm probe | ns | - | $(( (e - s) / 1000000 ))ms | INFO"
s=$(date +%s%N); "$PB" probe --cached >/dev/null 2>&1; e=$(date +%s%N)
note "perf.probe-cached | podvm probe --cached | ns | - | $(( (e - s) / 1000000 ))ms | INFO"
note ""
fi

# ---------------------------------------------------------------- lab
if on lab; then
note "== 7. lab"
t lab.scenarios ns "rc 0, a list" '[ $rc -eq 0 ] && has "$out" dual-nat' '$PB lab scenarios'
t lab.plan-dual-nat ns "rc 0, steps and verdicts" '[ $rc -eq 0 ] && has "$out" "== up" && has "$out" verdict' '$PB lab plan dual-nat'
t lab.plan-unknown ns "rc 125 named" '[ $rc -eq 125 ] && has "$err" nosuch' '$PB lab plan nosuch'
t lab.up ns "rc 125, names the missing guest parts" '[ $rc -eq 125 ]' '$PB lab up dual-nat'
t lab.mtu-big ns "names the range 1280 to 1499" 'has "$err" range' '$PB lab plan --nat both --mtu 99999'
t lab.nat-bad ns "names the valid words" 'has "$err" nat44' '$PB lab plan --nat bogus'
"$PB" lab scenarios >"$W/scenarios.txt" 2>/dev/null
while read -r name kind knobs; do
	[ -n "$name" ] || continue
	t lab.plan.$name ns "rc 0" '[ $rc -eq 0 ]' "\$PB lab plan $name"
	# The knob line of `lab scenarios` must be a valid flag set of plan.
	t lab.knobs.$name ns "the printed knobs plan at rc 0" '[ $rc -eq 0 ]' "\$PB lab plan $knobs"
done <"$W/scenarios.txt"
note ""
fi

# ---------------------------------------------------------------- machine
if on machine; then
note "== 8. machine tier (TCG only; no KVM)"
if command -v qemu-system-x86_64 >/dev/null 2>&1; then
	note "qemu        $(qemu-system-x86_64 --version | head -1)"
	note "kvm         $( [ -e /dev/kvm ] && echo present || echo absent )"
	TO=600 t machine.uname ns "a kernel release, rc 0" '[ $rc -eq 0 ] && [ -n "$out" ]' '$PB run --rm --podvm-tier=machine $A uname -r'
	TO=600 t machine.exit3 ns "rc 3" '[ $rc -eq 3 ]' "\$PB run --rm --podvm-tier=machine \$A sh -c 'exit 3'"
	TO=600 t machine.vol ns "invol through 9p" '[ "$out" = invol ]' '$PB run --rm --podvm-tier=machine -v $W/vol:/data $A cat /data/in.txt'
	TO=600 t machine.uname.ul ul "a kernel release, rc 0" '[ $rc -eq 0 ] && [ -n "$out" ]' '$X $PB run --rm --podvm-tier=machine $A uname -r'
else
	note "machine.uname | run --podvm-tier=machine A uname -r | ns | - | qemu-system-x86_64 absent | INFO"
fi
t machine.mem-elsewhere ns "rc 125: --podvm-mem without machine" '[ $rc -eq 125 ]' '$PB run --rm --podvm-mem 512M $A true'
t machine.tier-bad ns "rc 125 flag error" '[ $rc -eq 125 ]' '$PB run --rm --podvm-tier=bogus $A true'
cat >"$W/esc.py" <<'PY'
import io, sys, tarfile
t = tarfile.open(sys.argv[1], "w")
for n, d in ((".podvm-run.json", b"{}"), ("../qa-escape", b"x")):
    i = tarfile.TarInfo(n)
    i.size = len(d)
    t.addfile(i, io.BytesIO(d))
t.close()
PY
t machine.sealed-escape ns "rc 125 on the host: a member with .. leaves the guest root (T-1810)" '[ $rc -eq 125 ] && has "$err" -e "leave the guest root"' "python3 -I \$W/esc.py \$W/esc.tar && \$PB run --rm --podvm-tier=machine --podvm-sealed \$A < \$W/esc.tar"
for f in "--podvm-confine read=/tmp" "--user 1000" "--add-host a:1.2.3.4" "--read-only" "--tmpfs /mnt/x" "--ulimit nofile=512:512" "--security-opt no-new-privileges" "--cap-drop ALL" "--label a=b"; do
	n=${f%% *}; n=${n#--}
	t machine.refuse-$n ns "rc 125 before the boot, names the flag (T-1808)" '[ $rc -eq 125 ] && has "$err" -e "machine tier refused" && has "$err" -e "'"${f%% *}"'"' "\$PB run --rm --podvm-tier=machine $f \$A true"
done
note ""
fi

note "== summary"
note "pass $npass, fail $nfail, info $ninfo"
[ "$nfail" -eq 0 ] && exit 0
exit 1
