#!/bin/sh
# Question: does a guest of the machine tier with `--podvm-engine` run a
# container engine: overlay2 on cgroup2, a user-defined bridge network
# where a container reaches another by its name, and a port mapping, with
# no NIC in the guest?
#
# TODO/machine.md T-1780 (PodUtils/podrunner#92). The engine is the static
# Docker 27.5.1 of podrunner's measure; `iptables` (nf_tables) comes from
# the packages of Alpine 3.20, unpacked into the guest's root. Each input
# is pinned by its SHA-256. The container image is an x86_64 alpine 3.20
# tree of the store, imported with `docker import`.
#
# Pinned inputs:
#   DOCKER   download.docker.com/linux/static/stable/x86_64/docker-27.5.1.tgz
#   APK      iptables 1.8.10-r3, libmnl 1.0.5-r2, libnftnl 1.2.6-r0,
#            libxtables 1.8.10-r3 of dl-cdn.alpinelinux.org v3.20/main
#   OCI      alpine 3.20 of the store PODVM_STORE (default the store of
#            experiment 443)
#   podvm    PODVM_447_BIN (default the release build of the workspace)
#
# Clauses:
#   1. without the flag, the guest has no `bridge` module and cgroup2 is
#      mounted at /sys/fs/cgroup (each guest).
#   2. with the flag: dockerd reports the storage driver overlay2 and
#      cgroup version 2.
#   3. `docker network create` makes a bridge network; a container on it
#      resolves another by its name (the embedded DNS) and pings it.
#   4. `-p 8080:80` maps a port: 127.0.0.1:8080 in the guest reaches the
#      container.
#   5. no emulator stays.
#
# A run has a bound of 1500 s (TCG). Exit: 0 each clause matched, 1 one or
# more failed, 2 the lane could not run.
set -u

REPO="$(cd "$(dirname "$0")/.." && pwd)"
PB="${PODVM_447_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
export PODVM_STORE="${PODVM_STORE:-$REPO/.work/qa/store}"
A=mirror.gcr.io/library/alpine:3.20
W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT INT TERM

[ -x "$PB" ] || { echo "no podvm binary at $PB: COULD NOT RUN" >&2; exit 2; }
"$PB" image inspect "$A" >/dev/null 2>&1 || { echo "no alpine 3.20 in $PODVM_STORE: COULD NOT RUN" >&2; exit 2; }

fetch() { # url sha file
	curl -qfsSL -o "$W/$3" "$1" || return 1
	[ "$(sha256sum "$W/$3" | cut -d' ' -f1)" = "$2" ]
}
B=https://dl-cdn.alpinelinux.org/alpine/v3.20/main/x86_64
fetch https://download.docker.com/linux/static/stable/x86_64/docker-27.5.1.tgz \
	4f798b3ee1e0140eab5bf30b0edc4e84f4cdb53255a429dc3bbae9524845d640 docker.tgz &&
	fetch $B/iptables-1.8.10-r3.apk f08fec06cf6cc580a16bbed6dea73d660b00eb94156ac2960bb2646548608c99 a1.apk &&
	fetch $B/libmnl-1.0.5-r2.apk 7176c398245aab484e1c665a18f3287ca4185eaa9c8410cc007018f91a2ec019 a2.apk &&
	fetch $B/libnftnl-1.2.6-r0.apk dc45a94a654cc9208cd659e7b315ced4f7c62629a5c884d9eccc270ef2c69f4b a3.apk &&
	fetch $B/libxtables-1.8.10-r3.apk 12b255f468d22dd8cd21592dbc6696ce7eec82082d76b4c15d53ff941f24962e a4.apk ||
	{ echo "an input does not fetch or does not match its pin: COULD NOT RUN" >&2; exit 2; }
mkdir -p "$W/d/ipt"
tar xzf "$W/docker.tgz" -C "$W/d" || exit 2
for a in a1 a2 a3 a4; do tar xzf "$W/$a.apk" -C "$W/d/ipt" 2>/dev/null; done
[ -x "$W/d/ipt/sbin/iptables" ] || { echo "no iptables in the packages: COULD NOT RUN" >&2; exit 2; }
R=$(for d in "$PODVM_STORE"/rootfs/*/rootfs; do
	[ -f "$d/etc/alpine-release" ] && file "$d/bin/busybox" | grep -q x86-64 && echo "$d" && break
done)
[ -n "$R" ] || { echo "no x86_64 alpine tree in the store: COULD NOT RUN" >&2; exit 2; }
tar -C "$R" -cf "$W/d/alpine.tar" . || exit 2
cat >"$W/d/t.sh" <<'EOF'
cp -a /d/ipt/sbin/. /sbin/; cp -a /d/ipt/usr/lib/. /usr/lib/; cp -a /d/ipt/etc/. /etc/ 2>/dev/null
export PATH=/d/docker:$PATH DOCKER_RAMDISK=true
mkdir -p /var/lib/docker /run; mount -t tmpfs tmpfs /var/lib/docker
dockerd --storage-driver=overlay2 >/tmp/d.log 2>&1 &
i=0; while ! docker info >/dev/null 2>&1 && [ $i -lt 300 ]; do sleep 1; i=$((i+1)); done
echo "info $(docker info --format '{{.Driver}} {{.CgroupVersion}}' 2>&1)"
docker import /d/alpine.tar a:1 >/dev/null
docker network create n1 >/dev/null && echo "network $(docker network inspect n1 --format '{{.Driver}}')"
docker run -d --name srv --network n1 a:1 sleep 300 >/dev/null
echo "dns $(docker run --rm --network n1 a:1 nslookup srv 2>&1 | grep -c '^Name:.*srv')"
echo "ping $(docker run --rm --network n1 a:1 ping -c1 -W5 srv 2>&1 | grep -c '1 packets received')"
docker run -d --name web -p 8080:80 a:1 sh -c 'while true; do nc -l -p 80 -e echo mapped; done' >/dev/null
sleep 3
echo "map $(nc 127.0.0.1 8080 </dev/null 2>&1)"
EOF

FAIL=0
verdict() { # clause, test, what
	if eval "$2"; then echo "PASS $1: $3"; else echo "FAIL $1: $3"; FAIL=1; fi
}

plain=$(timeout 600 "$PB" run --rm --podvm-tier=machine --podvm-net none "$A" \
	sh -c 'grep -c "^bridge " /proc/modules; cat /sys/fs/cgroup/cgroup.controllers' 2>/dev/null)
echo "$plain" | sed 's/^/  1| /'
verdict 1 '[ "$(echo "$plain" | sed -n 1p)" = 0 ] && echo "$plain" | sed -n 2p | grep -q memory' \
	"no flag: no bridge module; cgroup2 is mounted"

out=$(timeout 1500 "$PB" run --rm --podvm-tier=machine --podvm-engine --podvm-net none \
	--podvm-mem 2G --podvm-cpus 2 -v "$W/d:/d" "$A" sh /d/t.sh 2>/dev/null)
echo "$out" | sed 's/^/  2| /'
verdict 2 'echo "$out" | grep -qx "info overlay2 2"' "dockerd: overlay2 on cgroup2"
verdict 3 'echo "$out" | grep -qx "network bridge" && echo "$out" | grep -qx "dns 1" && echo "$out" | grep -qx "ping 1"' \
	"a bridge network: a container resolves and pings another by its name"
verdict 4 'echo "$out" | grep -qx "map mapped"' "-p 8080:80 reaches the container from the guest"

sleep 1
left=$(pgrep -af "qemu-system-" 2>/dev/null | cut -c1-120)
verdict 5 '[ -z "$left" ]' "no emulator stays (${left:-none})"

if [ $FAIL = 0 ]; then echo "A GUEST RUNS A CONTAINER ENGINE"; fi
exit $FAIL
