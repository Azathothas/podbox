#!/bin/sh
# Question: on this host, does incus run for a user with no root at run
# time (route A of errand-box), at what cost against podvm on the same
# work, and can an OCI image that podvm pulled become an incus instance
# with docker's semantics?
#
# TODO/machine.md T-1717. Route A of `errand-box:docs/research/rootless.md` of
# Azathothas/errand-box at 1ad2fc1 (reference issue 251): root prepares the
# host once (a user, a subordinate id range, cgroup subtrees that the user
# owns); then the user starts incusd as root of the user's own user
# namespace, after `research/experiments/rootless/start.sh` and
# `rincus-ns.sh` of errand-box. Run as root, which is the one preparation.
#
# Pinned inputs:
#   incus     the Ubuntu package 6.0.0-1ubuntu0.3 (apt)
#   image     alpine 3.21 amd64 default 20261109_13:00 of
#             images.linuxcontainers.org, incus.tar.xz and rootfs.squashfs
#             by SHA-256 (incusd sits in an empty network namespace and
#             cannot fetch; the client uploads)
#   OCI       alpine:3.21 by digest, pulled by podvm as the same user
#
# Clauses:
#   1. incusd runs as root of the user's namespace: uid 0 maps to the user,
#      and the driver lxc is up.
#   2. an instance launches; its PID 1 is /sbin/init, and its root is an id
#      of the user's range on the host, not 0.
#   3. the busybox workloads of experiments/lib/wl-busybox.sh, RUNS times
#      each, on the incus instance and on podvm as the same user; medians.
#   4. the life of an instance (launch, exec, delete, start) against
#      `podvm run --rm IMAGE true`.
#   5. the OCI image of podvm becomes an incus image (a unified tarball
#      with owner 0), launches with the image's command as PID 1
#      (`lxc.init.cmd`), and its files belong to root in the guest.
#   6. nothing of the run stays: no process of the user, no state.
#
# Exit: 0 every clause matched, 1 a clause disagreed, 2 the lane could not
# run (not root, no user namespaces, no package, no download).
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep433-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
REPORT="$WORK/out-433.txt"
RUNS=${PODVM_433_RUNS:-5}
U=podvm433
B=/srv/$U
INCUS_PKG=6.0.0-1ubuntu0.3
IMG=https://images.linuxcontainers.org/images/alpine/3.21/amd64/default/20261109_13:00
META_SHA=fe562d23dfe4399760c3a5bf706a534ade800d4d396c7b47a4c8a32458ccc43a
ROOTFS_SHA=871eaf9cc5c0f48777f4c7c5def07f496cfaac113bedf9a834524597a50aaeef
OCI='public.ecr.aws/docker/library/alpine@sha256:bc21487101ae47172509130ee1092cc273a831968d64b617776d1f69c81e42ab'
# ⚠ PODVM_433_MIRROR=1 takes the same digest from mirror.gcr.io.
[ "${PODVM_433_MIRROR:-0}" = 1 ] && OCI='mirror.gcr.io/library/alpine@sha256:bc21487101ae47172509130ee1092cc273a831968d64b617776d1f69c81e42ab'
{
echo "== conditions"
echo "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "host kernel       $(uname -sr)"
echo "cgroup            $(stat -fc %T /sys/fs/cgroup)"
echo "incus package     $INCUS_PKG"
echo "image             $IMG"
echo "OCI               $OCI"
echo "runs              $RUNS"
} >"$REPORT"
note() { echo "$*" >>"$REPORT"; }
fail=0

[ "$(id -u)" -eq 0 ] || { echo "not root: the one preparation needs root: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
unshare -Ur true 2>/dev/null || { echo "user namespaces refused: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
DEBIAN_FRONTEND=noninteractive timeout 600 apt-get install -y -qq "incus=$INCUS_PKG" uidmap >"$WORK/apt.log" 2>&1 ||
	{ echo "apt-get install FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
INCUSD=/usr/libexec/incus/incusd
[ -x "$INCUSD" ] || { echo "no $INCUSD: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }
timeout 1800 cargo build >"$WORK/build.log" 2>&1 || { echo "build FAILED: COULD NOT RUN" | tee -a "$REPORT"; exit 2; }

# ---- the one preparation, by root -------------------------------------
# The guests' processes run as ids of the user's range, not as the user,
# and they outlive incusd: each one of the range goes too.
ranged() { ps -eo pid=,uid= | awk -v u="$(id -u "$U" 2>/dev/null || echo -1)" '$2 == u || ($2 >= 2000000000 && $2 < 2100000000) {print $1}'; }
cleanup() {
	i=0
	while [ "$i" -lt 10 ] && [ -n "$(ranged)" ]; do
		ranged | xargs -r kill -9 2>/dev/null; sleep 1; i=$((i + 1))
	done
	rm -rf "$B"
	for c in /sys/fs/cgroup/*/"$U"; do rmdir "$c" 2>/dev/null; done
}
cleanup
id "$U" >/dev/null 2>&1 || useradd -m -s /bin/sh "$U" || exit 2
# One range of 100000000 ids, and no other line: --map-auto takes the
# first, and useradd may have added one of 65536 (errand-box trap 1).
for f in /etc/subuid /etc/subgid; do sed -i "/^$U:/d" "$f"; echo "$U:2000000000:100000000" >>"$f"; done
for c in /sys/fs/cgroup/*/; do
	n=$(basename "$c"); [ "$n" = unified ] && continue
	mkdir -p "$c$U" 2>/dev/null || continue
	[ -f "$c/cpuset.cpus" ] && cat "$c/cpuset.cpus" >"$c$U/cpuset.cpus" && cat "$c/cpuset.mems" >"$c$U/cpuset.mems"
	chown -R "$U:$U" "$c$U"
done
mkdir -p "$B/state" "$B/img" /var/log/incus /var/cache/incus
cp "$REPO/experiments/lib/wl-busybox.sh" "$B/wl.sh"
cp "$REPO/target/x86_64-unknown-linux-musl/debug/podvm" "$B/podvm"
timeout 300 curl -qfsS --retry 3 -o "$B/img/meta.tar.xz" "$IMG/incus.tar.xz" &&
	timeout 300 curl -qfsS --retry 3 -o "$B/img/rootfs.squashfs" "$IMG/rootfs.squashfs" ||
	{ echo "image download FAILED: COULD NOT RUN" | tee -a "$REPORT"; cleanup; exit 2; }
printf '%s  %s\n%s  %s\n' "$META_SHA" "$B/img/meta.tar.xz" "$ROOTFS_SHA" "$B/img/rootfs.squashfs" | sha256sum -c - >/dev/null ||
	{ echo "image SHA-256 MISMATCH: COULD NOT RUN" | tee -a "$REPORT"; cleanup; exit 2; }
cat >"$B/ns.sh" <<EOF
#!/bin/sh
set -u
base=$B/state
mkdir -p \$base/lib \$base/log \$base/cache \$base/run/cg
mount --rbind /sys/fs/cgroup \$base/run/cg && mount --rbind \$base/run/cg /sys/fs/cgroup
mount -t proc proc /proc
mount -t tmpfs tmpfs /run && mkdir -p /run/incus
mount --bind \$base/lib /var/lib/incus
mount --bind \$base/log /var/log/incus
mount --bind \$base/cache /var/cache/incus
printf 'root:1000000:65536\n' >\$base/subid
mount --bind \$base/subid /etc/subuid
mount --bind \$base/subid /etc/subgid
echo "uid_map \$(tr -s ' ' </proc/self/uid_map | tr '\n' ';')"
export INCUS_DIR=/var/lib/incus
exec $INCUSD --group root --logfile /var/log/incus/incusd.log
EOF
cat >"$B/start.sh" <<EOF
#!/bin/sh
for h in /sys/fs/cgroup/*/$U; do echo \$\$ >"\$h/cgroup.procs" 2>/dev/null; done
exec unshare --user --map-root-user --map-auto --mount --pid --fork --propagation private sh $B/ns.sh
EOF
chown -R "$U:$U" "$B"

AS() { runuser -u "$U" -- "$@"; }
RI() { runuser -u "$U" -- env INCUS_DIR="$B/state/lib" incus "$@" </dev/null; }
P() { runuser -u "$U" -- env PODVM_STORE="$B/store" HOME="$B" "$B/podvm" "$@"; }
ms() { echo $(( $(date +%s%N) / 1000000 )); }
median() { tr ' ' '\n' | grep . | sort -n | awk '{a[NR]=$1} END {print (NR ? a[int((NR+1)/2)] : "-")}'; }

# ---- clause 1 -----------------------------------------------------------
(AS sh "$B/start.sh" >"$WORK/start.log" 2>&1 &)
i=0; while [ "$i" -lt 30 ] && [ ! -S "$B/state/lib/unix.socket" ]; do sleep 1; i=$((i + 1)); done
{ echo ""; echo "== 1. incusd as root of the user's namespace"; sed 's/^/  /' "$WORK/start.log"; } >>"$REPORT"
RI info >"$WORK/info.txt" 2>&1
grep -E "server_version|driver:|storage_supported" "$WORK/info.txt" | head -3 | sed 's/^/  /' >>"$REPORT"
grep -q "uid_map  *0 $(id -u "$U") 1" "$WORK/start.log" || { note "1: uid 0 does not map to the user"; fail=1; }
grep -q "driver: lxc" "$WORK/info.txt" || { note "1: the lxc driver is not up"; fail=1; }

# ---- clause 2 -----------------------------------------------------------
RI storage create default dir >/dev/null 2>&1
RI profile device add default root disk path=/ pool=default >/dev/null 2>&1
RI image import "$B/img/meta.tar.xz" "$B/img/rootfs.squashfs" --alias a321 >/dev/null 2>&1
RI launch -q a321 c1 >"$WORK/c2.txt" 2>&1
RI file push -q "$B/wl.sh" c1/run/wl.sh >>"$WORK/c2.txt" 2>&1
init=$(RI exec c1 -- sh -c 'cat /proc/1/comm' 2>&1)
hostuid=$(ps -o uid= -C init 2>/dev/null | awk '$1 >= 2000000000' | head -1 | tr -d ' ')
{ echo ""; echo "== 2. an instance"; echo "  PID 1           $init"; echo "  its uid on host $hostuid"; sed 's/^/  /' "$WORK/c2.txt"; } >>"$REPORT"
[ "$init" = init ] || { note "2: PID 1 is not init"; fail=1; }
[ -n "$hostuid" ] || { note "2: no init of the guest runs with an id of the user's range"; fail=1; }

# ---- clause 3 -----------------------------------------------------------
P pull "$OCI" >"$WORK/pull.log" 2>&1 || { note "3: podvm pull FAILED"; fail=1; }
rung=$(P system info --format '{{.EnteredRung}}' 2>/dev/null)
{ echo ""; echo "== 3. busybox workloads, median ms of $RUNS (podvm rung: $rung)"; printf '  %-8s %8s %8s\n' load incus podvm; } >>"$REPORT"
for l in spawn files cpu; do
	a=""; b=""; r=0
	while [ "$r" -lt "$RUNS" ]; do
		r=$((r + 1))
		a="$a $(RI exec c1 -- sh /run/wl.sh "$l" 2>/dev/null | sed -n 's/inner_ms=//p')"
		b="$b $(P run --rm -v "$B/wl.sh:/run/wl.sh" "$OCI" sh /run/wl.sh "$l" 2>/dev/null | sed -n 's/inner_ms=//p')"
	done
	ma=$(echo "$a" | median); mb=$(echo "$b" | median)
	printf '  %-8s %8s %8s\n' "$l" "$ma" "$mb" >>"$REPORT"
	[ "$ma" = - ] || [ "$mb" = - ] && { note "3: $l gave no time"; fail=1; }
done

# ---- clause 4 -----------------------------------------------------------
L=""; E=""; D=""; R=""
r=0
while [ "$r" -lt "$RUNS" ]; do
	r=$((r + 1))
	t0=$(ms); RI launch -q a321 "n$r" >/dev/null 2>&1; t1=$(ms); RI exec "n$r" -- true; t2=$(ms); RI delete -f "n$r" >/dev/null 2>&1; t3=$(ms)
	L="$L $((t1 - t0))"; E="$E $((t2 - t1))"; D="$D $((t3 - t2))"
	t0=$(ms); P run --rm "$OCI" true >/dev/null 2>&1; t1=$(ms); R="$R $((t1 - t0))"
done
RI stop c1 >/dev/null 2>&1; t0=$(ms); RI start c1 >/dev/null 2>&1; t1=$(ms)
{ echo ""; echo "== 4. the life of an instance, median ms of $RUNS"
echo "  incus launch       $(echo "$L" | median)"
echo "  incus exec true    $(echo "$E" | median)"
echo "  incus delete       $(echo "$D" | median)"
echo "  incus start        $((t1 - t0)) (one stopped instance)"
echo "  podvm run --rm     $(echo "$R" | median)"; } >>"$REPORT"

# ---- clause 5 -----------------------------------------------------------
P extract "$OCI" >/dev/null 2>&1
ROOT=$(P inspect --format '{{.RootfsPath}}' "$OCI" 2>/dev/null)
printf 'architecture: x86_64\ncreation_date: %s\nproperties:\n  description: an OCI image of podvm\n' "$(date +%s)" >"$B/metadata.yaml"
t0=$(ms)
tar -C "$B" -cf "$B/unified.tar" metadata.yaml &&
	tar -C "$(dirname "$ROOT")" --owner=0 --group=0 --numeric-owner -rf "$B/unified.tar" rootfs && chown "$U" "$B/unified.tar"
RI image import "$B/unified.tar" --alias oci >"$WORK/c5.txt" 2>&1
RI launch -q oci o1 -c raw.lxc='lxc.init.cmd = /bin/sleep 300' >>"$WORK/c5.txt" 2>&1
t1=$(ms)
pid1=$(RI exec o1 -- sh -c 'tr "\0" " " </proc/1/cmdline' 2>&1)
owner=$(RI exec o1 -- sh -c 'stat -c %u:%g /etc/passwd' 2>&1)
{ echo ""; echo "== 5. the OCI image as an incus instance"; echo "  tar, import, launch ms  $((t1 - t0))"; echo "  PID 1                   $pid1"; echo "  owner of /etc/passwd    $owner"; sed 's/^/  /' "$WORK/c5.txt"; } >>"$REPORT"
case "$pid1" in "/bin/sleep 300"*) ;; *) note "5: the image's command is not PID 1"; fail=1 ;; esac
[ "$owner" = 0:0 ] || { note "5: the files do not belong to root in the guest"; fail=1; }

# ---- clause 6 -----------------------------------------------------------
cleanup
left=$(ranged | wc -l)
{ echo ""; echo "== 6. cleanup"; echo "  processes of $U and its range  $left"; echo "  state            $([ -e "$B" ] && echo stayed || echo gone)"; } >>"$REPORT"
[ "$left" -eq 0 ] || { note "6: a process of the user stayed"; fail=1; }

{
echo ""
if [ "$fail" -eq 0 ]; then echo "verdict           INCUS RUNS WITHOUT ROOT AT RUN TIME"; else echo "verdict           A CLAUSE DISAGREED"; fi
} >>"$REPORT"
cp "$REPORT" "$REPO/experiments/results/incus-rootless.txt"
cat "$REPORT"
[ "$fail" -eq 0 ]
