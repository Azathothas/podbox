#!/bin/sh
# Question: do the profiles of `run --profile` give one sandbox shape by
# name, refuse a misspelt key and an absent mechanism by name, and run one
# command line unchanged in each built-in shape that places /workspace?
#
# TODO/cli.md T-1654 (issue 166). The rootfs profiles run on the fixture
# of experiment 356 (user and mount namespaces, /dev/pts hidden, chroot(2)
# denied), where the userland loader family serves a volume, `--read-only`
# and `--tmpfs`. The machine profile runs a guest (KVM where /dev/kvm
# opens, else TCG).
#
# Pinned input: ALPINE of experiment 356 (alpine 3.20 by digest).
# PODVM_410_MIRROR=1 takes the same digest from mirror.gcr.io.
#
# Clauses:
#   1. `profile list` exits 0 and names the five built-in profiles.
#   2. `profile show workspace -s fs.user=1000 -s limits.file_max="1g"`
#      prints the effective profile (`user = 1000`, `file_max = "1g"`) and
#      its run flags (`--user=1000`, `--ulimit=fsize=1073741824`).
#   3. a profile that asks for an absent tier refuses with the probe's
#      reason: the incus profile where no incusd answers, and the machine
#      profile with PODVM_MACHINE_ACCEL=kvm where /dev/kvm does not open.
#   4. `sh -c 'pwd; ls /workspace'` gives the same output in each built-in
#      profile that places /workspace: workspace, readonly and machine.
#   5. `limits.memory` refuses by name on a rootfs tier (exit 125).
#   6. a user profile with a misspelt key refuses at `run`, naming the key,
#      before anything is pulled.
#   7. a user profile that extends a built-in one gives its `env.set` to the
#      command, and `-s` overrides it.
#   8. `-s` without `--profile` refuses.
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep410-work"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
OUT="$REPO/experiments/results/profiles.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
ALPINE='public.ecr.aws/docker/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
[ "${PODVM_410_MIRROR:-0}" = 1 ] && ALPINE='mirror.gcr.io/library/alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc'
export PODVM_STORE="$WORK/store"
export XDG_CONFIG_HOME="$WORK/cfg"
mkdir -p "$XDG_CONFIG_HOME/podvm/profiles" "$WORK/ws"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }

[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }
command -v cc >/dev/null 2>&1 || { echo "cc missing: COULD NOT RUN" >&2; exit 2; }
unshare -Urm true >/dev/null 2>&1 || { echo "user+mount namespaces refused: COULD NOT RUN" >&2; exit 2; }
cc -O2 -Wall -Werror -static -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" 2>/dev/null ||
	cc -O2 -Wall -Werror -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" ||
	{ echo "fixture COMPILE FAILED: COULD NOT RUN" >&2; exit 2; }
cat >"$WORK/wrap.sh" <<'EOF'
#!/bin/sh
mount -t tmpfs tmpfs /dev/pts || exit 2
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF
fx() { unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$PB" "$@"; }

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "/dev/kvm          $([ -c /dev/kvm ] && echo present || echo absent)"
note "image             $ALPINE"
timeout 600 "$PB" pull -q "$ALPINE" >"$WORK/pull.log" 2>&1 ||
	{ note "pull FAILED: COULD NOT RUN"; sed 's/^/  /' "$WORK/pull.log"; exit 2; }
echo "probe" >"$WORK/ws/probe.txt"
note ""

note "== clause 1  profile list"
"$PB" profile list >"$WORK/c1.out" 2>&1
rc=$?
sed 's/^/    /' "$WORK/c1.out" | tee -a "$OUT"
note "  exit $rc  (want 0)"
[ "$rc" -eq 0 ] || bad "profile list exited $rc"
for n in default workspace readonly machine incus; do
	grep -q "^$n  *built-in" "$WORK/c1.out" || bad "$n is not listed as built-in"
done
note ""

note "== clause 2  profile show with -s"
(cd "$WORK/ws" && "$PB" profile show workspace -s fs.user=1000 -s 'limits.file_max="1g"') >"$WORK/c2.out" 2>&1
rc=$?
sed 's/^/    /' "$WORK/c2.out" | sed "s#$WORK#WORK#g" | tee -a "$OUT"
note "  exit $rc  (want 0)"
[ "$rc" -eq 0 ] || bad "profile show exited $rc"
grep -qx 'user = 1000' "$WORK/c2.out" || bad "the effective profile has no user = 1000"
grep -qx 'file_max = "1g"' "$WORK/c2.out" || bad "the effective profile has no file_max"
grep -q -- '--user=1000' "$WORK/c2.out" || bad "the run flags have no --user=1000"
grep -q -- '--ulimit=fsize=1073741824' "$WORK/c2.out" || bad "the run flags have no fsize"
note ""

note "== clause 3  an absent tier refuses with the probe's reason"
INCUS_DIR="$WORK/no-incus" PODVM_INCUS_SOCKET="$WORK/no-incus/unix.socket" \
	timeout 120 "$PB" run --rm --profile incus "$ALPINE" true >"$WORK/c3a.out" 2>&1
rc=$?
note "  incus             exit $rc: $(grep -m1 'incus' "$WORK/c3a.out" | sed "s#$WORK#WORK#g")"
[ "$rc" -eq 125 ] || bad "the incus profile exited $rc"
grep -q "no incusd answers" "$WORK/c3a.out" || bad "the incus refusal does not give the reason"
if [ -c /dev/kvm ] && [ -r /dev/kvm ]; then
	note "  machine (kvm)     /dev/kvm opens here, so this half does not apply"
else
	(cd "$WORK/ws" && PODVM_MACHINE_ACCEL=kvm timeout 120 "$PB" run --rm --profile machine "$ALPINE" true) >"$WORK/c3b.out" 2>&1
	rc=$?
	note "  machine (kvm)     exit $rc: $(grep -m1 'kvm' "$WORK/c3b.out")"
	[ "$rc" -eq 125 ] || bad "the machine profile under kvm exited $rc"
	grep -q "/dev/kvm does not open" "$WORK/c3b.out" || bad "the kvm refusal does not give the reason"
fi
note ""

note "== clause 4  one command line in each profile that places /workspace"
CMD='pwd; ls /workspace'
want=$(printf '/workspace\nprobe.txt')
for p in workspace readonly machine; do
	if [ "$p" = machine ]; then
		(cd "$WORK/ws" && timeout 600 "$PB" run --rm --profile "$p" "$ALPINE" sh -c "$CMD") >"$WORK/c4-$p.out" 2>"$WORK/c4-$p.err"
	else
		(cd "$WORK/ws" && fx run --rm --profile "$p" "$ALPINE" sh -c "$CMD") >"$WORK/c4-$p.out" 2>"$WORK/c4-$p.err"
	fi
	rc=$?
	note "  $p: exit $rc, output $(tr '\n' '|' <"$WORK/c4-$p.out")"
	[ "$rc" -eq 0 ] || { bad "$p exited $rc"; sed 's/^/      /' "$WORK/c4-$p.err" | tail -3 | tee -a "$OUT"; }
	[ "$(cat "$WORK/c4-$p.out")" = "$want" ] || bad "$p gives other output"
done
note ""

note "== clause 5  limits.memory on a rootfs tier"
(cd "$WORK/ws" && timeout 60 "$PB" run --rm --profile workspace -s limits.memory=1g "$ALPINE" true) >"$WORK/c5.out" 2>&1
rc=$?
note "  exit $rc: $(head -1 "$WORK/c5.out")"
[ "$rc" -eq 125 ] || bad "limits.memory exited $rc"
grep -q '`limits.memory` needs fs.backend = "machine"' "$WORK/c5.out" || bad "the refusal does not name limits.memory"
note ""

note "== clause 6  a misspelt key of a user profile"
printf 'extends = "workspace"\n[fs]\nread_onyl = true\n' >"$XDG_CONFIG_HOME/podvm/profiles/cage.toml"
timeout 60 "$PB" run --rm --profile cage never.invalid/none:1 true >"$WORK/c6.out" 2>&1
rc=$?
note "  exit $rc: $(head -1 "$WORK/c6.out")"
[ "$rc" -eq 125 ] || bad "the misspelt key exited $rc"
grep -q 'unknown key `fs.read_onyl`' "$WORK/c6.out" || bad "the refusal does not name fs.read_onyl"
grep -q "never.invalid" "$WORK/c6.out" && bad "the run reached the pull"
rm -f "$XDG_CONFIG_HOME/podvm/profiles/cage.toml"
note ""

note "== clause 7  a user profile extends a built-in one"
printf 'description = "hello"\nextends = "workspace"\n[env.set]\nGREETING = "from-profile"\n' >"$XDG_CONFIG_HOME/podvm/profiles/hello.toml"
(cd "$WORK/ws" && fx run --rm --profile hello "$ALPINE" sh -c 'echo "$GREETING $PWD"') >"$WORK/c7a.out" 2>"$WORK/c7a.err"
ra=$?
(cd "$WORK/ws" && fx run --rm --profile hello -s env.set.GREETING=from-s "$ALPINE" sh -c 'echo "$GREETING $PWD"') >"$WORK/c7b.out" 2>"$WORK/c7b.err"
rb=$?
note "  the profile       exit $ra: $(cat "$WORK/c7a.out")"
note "  with -s           exit $rb: $(cat "$WORK/c7b.out")"
[ "$(cat "$WORK/c7a.out")" = "from-profile /workspace" ] || bad "env.set or the workdir did not reach the command"
[ "$(cat "$WORK/c7b.out")" = "from-s /workspace" ] || bad "-s did not override env.set"
note ""

note "== clause 8  -s without --profile"
timeout 60 "$PB" run --rm -s fs.user=1 "$ALPINE" true >"$WORK/c8.out" 2>&1
rc=$?
note "  exit $rc: $(head -1 "$WORK/c8.out")"
[ "$rc" -eq 125 ] || bad "-s alone exited $rc"
grep -q -- "-s needs --profile" "$WORK/c8.out" || bad "the refusal does not name --profile"
note ""

if pgrep -f "qemu-system.*podvm-init" >/dev/null 2>&1; then bad "an emulator stayed"; fi
if [ "$fail" -eq 0 ]; then
	note "verdict: THE PROFILES GIVE ONE SHAPE BY NAME AND REFUSE BY NAME"
else
	note "verdict: A CLAUSE DISAGREED"
fi
rm -rf "$WORK"
exit "$fail"
