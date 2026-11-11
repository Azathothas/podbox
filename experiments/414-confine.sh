#!/bin/sh
# Question: does `run --podvm-confine` hold a payload to its paths and its
# TCP ports, with the seccomp deny filter and no_new_privs, and does each
# case with no mechanism refuse by name?
#
# TODO/enter.md T-1658 (issue 170). The layer is pod-confine's (Landlock,
# a seccomp deny filter, PR_SET_NO_NEW_PRIVS), applied by pod-enter in
# the payload's child before the exec. It runs on a rung with a chroot:
# the namespace rung of a host that grants user namespaces, as here.
#
# Pinned input: busybox:musl of mirror.gcr.io (PODVM_414_IMAGE overrides).
# The image has no /etc/hostname, so the outside read is /etc/shadow; the
# namespace rung mounts no /proc, so the probe of probe-inside reads
# no_new_privs and the seccomp mode with prctl(2).
#
# Clauses:
#   1. `read=/etc/passwd;connect=PORT_A`: /etc/passwd reads, /etc/shadow
#      fails with "Permission denied", a write to /tmp fails, a TCP connect
#      to a host listener on PORT_A answers, and one on PORT_B fails with
#      "Permission denied".
#   2. the same run with no layer reads /etc/shadow and reaches PORT_B (the
#      control: the image and the host allow both).
#   3. probe-inside with a profile `confine = true`: NoNewPrivs 1 and
#      Seccomp 2; with no profile: 0 and 0.
#   4. on the userland rung (chroot denied, the fixture of experiment 356):
#      a dynamic program (alpine) runs on the loader family with the layer
#      in host paths, through the table: /etc/passwd reads, /etc/shadow and
#      /tmp are denied, and a `-v` volume at /workspace reads and writes;
#      the static busybox (the native-static rung) refuses by name, 125.
#   5. `run -d` with the layer: the detached payload cannot read
#      /etc/shadow, and `exec` into the container takes the same layer from
#      the record.
#   6. the built-in profile `errand`: NoNewPrivs 1, Seccomp 2, the
#      namespace, mount, bpf and keyctl calls refused, `/` unlistable,
#      /etc/shadow denied and /etc/passwd read (the verdicts of bailey's
#      reference report in issue 170). Not measured: a write to /tmp, which
#      the namespace rung mounts after the rules (COORD-048).
#   The refusal on a kernel with no Landlock is a test:
#   `cargo test -p podvm --bin podvm confine` (each_case_with_no_mechanism_
#   refuses_by_name).
#
# Exit: 0 each clause matched, 1 a clause disagreed, 2 the lane could not
# run.
set -u

REPO="$(pwd)"
WORK="$REPO/experiments/.sweep414-work"
rm -rf "$WORK"; mkdir -p "$WORK/xdg/podvm/profiles" || exit 2
OUT="$REPO/experiments/results/confine.txt"
PB="${PODVM_BIN:-$REPO/target/x86_64-unknown-linux-musl/release/podvm}"
IMG="${PODVM_414_IMAGE:-mirror.gcr.io/library/busybox:musl}"
# A dynamic image, for the loader family (busybox:musl is static).
DYN="${PODVM_414_DYNAMIC:-mirror.gcr.io/library/alpine:3.20}"
export PODVM_STORE="$WORK/store"
export XDG_CONFIG_HOME="$WORK/xdg"
: >"$OUT"
note() { echo "$*" | tee -a "$OUT"; }
fail=0
bad() { note "  DISAGREES: $*"; fail=1; }
LPID=""
trap '[ -n "$LPID" ] && kill "$LPID" 2>/dev/null; rm -rf "$WORK"' EXIT INT TERM

[ -x "$PB" ] || { echo "SKIP: $PB is not built: cargo build --release -p podvm" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "python3 missing: COULD NOT RUN" >&2; exit 2; }

# Two listeners on the host loopback; each answers its own port.
cat >"$WORK/listen.py" <<'EOF'
import socket, sys, threading
socks = []
for _ in range(2):
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    s.listen(8)
    socks.append(s)
print(" ".join(str(s.getsockname()[1]) for s in socks), flush=True)
def serve(s):
    while True:
        c, _ = s.accept()
        c.sendall(b"hello-%d\n" % s.getsockname()[1])
        c.close()
for s in socks:
    threading.Thread(target=serve, args=(s,), daemon=True).start()
threading.Event().wait()
EOF
python3 "$WORK/listen.py" >"$WORK/ports" 2>/dev/null &
LPID=$!
i=0
while [ ! -s "$WORK/ports" ] && [ "$i" -lt 50 ]; do sleep 0.1; i=$((i + 1)); done
read -r A B <"$WORK/ports" || { note "the listeners did not start: COULD NOT RUN"; exit 2; }

note "== conditions"
note "date              $(date -u +%Y-%m-%dT%H:%M:%SZ)"
note "host kernel       $(uname -sr)"
note "podvm             $("$PB" version 2>/dev/null | head -1)"
note "rung              $("$PB" system info --format '{{.EnteredRung}}' 2>/dev/null)"
note "image             $IMG"
note "listeners         127.0.0.1:A and 127.0.0.1:B"
note ""

timeout 600 "$PB" pull "$IMG" >"$WORK/pull.log" 2>&1 || { note "pull FAILED: COULD NOT RUN"; tail -2 "$WORK/pull.log"; exit 2; }

CMD='head -1 /etc/passwd; cat /etc/shadow >/dev/null; echo "shadow=$?"; echo x > /tmp/w; echo "write=$?"; nc 127.0.0.1 '"$A"' </dev/null; echo "a=$?"; nc 127.0.0.1 '"$B"' </dev/null; echo "b=$?"'

note "== clause 1  the layer"
timeout 120 "$PB" run --rm --podvm-confine="read=/etc/passwd;connect=$A" "$IMG" sh -c "$CMD" >"$WORK/c1.out" 2>&1
rc=$?
grep -v '^podvm: \|^  ' "$WORK/c1.out" | sed "s/$A/A/g; s/$B/B/g" | sed 's/^/    /' | tee -a "$OUT"
[ "$rc" -eq 0 ] || bad "the run exited $rc"
grep -q "^root:" "$WORK/c1.out" || bad "/etc/passwd did not read"
grep -q "shadow.*Permission denied" "$WORK/c1.out" || bad "/etc/shadow was not denied"
grep -q "^write=1" "$WORK/c1.out" || bad "the write to /tmp did not fail"
grep -q "^hello-$A" "$WORK/c1.out" || bad "the allowed port did not answer"
grep -q "^b=1" "$WORK/c1.out" || bad "the other port was not refused"
grep -q "Permission denied" "$WORK/c1.out" || bad "no EACCES for the refused calls"
note ""

note "== clause 2  the control, with no layer"
timeout 120 "$PB" run --rm "$IMG" sh -c "$CMD" >"$WORK/c2.out" 2>&1
grep -v '^podvm: \|^  ' "$WORK/c2.out" | sed "s/$A/A/g; s/$B/B/g" | sed 's/^/    /' | tee -a "$OUT"
grep -q "^shadow=0" "$WORK/c2.out" || bad "the control could not read /etc/shadow"
grep -q "^hello-$B" "$WORK/c2.out" || bad "the control did not reach the other port"
note ""

note "== clause 3  no_new_privs and the seccomp mode, read inside"
printf 'description = "414"\nconfine = true\n[fs]\nread = ["/etc/passwd"]\n' >"$XDG_CONFIG_HOME/podvm/profiles/c414.toml"
timeout 300 "$PB" probe-inside --profile c414 "$IMG" >"$WORK/c3.out" 2>&1
timeout 300 "$PB" probe-inside "$IMG" >"$WORK/c3b.out" 2>&1
for f in c3 c3b; do
	note "  $( [ "$f" = c3 ] && echo confined || echo control ): $(grep -E '	(nonewprivs|seccomp)	' "$WORK/$f.out" | awk '{print $2"="$4}' | tr '\n' ' ')"
done
grep -qE '	nonewprivs	ALLOW	1$' "$WORK/c3.out" || bad "NoNewPrivs is not 1 under the layer"
grep -qE '	seccomp	ALLOW	2$' "$WORK/c3.out" || bad "the seccomp mode is not 2 under the layer"
grep -qE '	seccomp	ALLOW	0$' "$WORK/c3b.out" || bad "the control has a seccomp filter"
note ""

note "== clause 4  the loader family of the userland rung (chroot denied)"
command -v cc >/dev/null 2>&1 || { note "cc missing: COULD NOT RUN"; exit 2; }
cc -O2 -Wall -Werror -o "$WORK/deny-chroot" "$REPO/experiments/deny-chroot.c" || { note "fixture COMPILE FAILED: COULD NOT RUN"; exit 2; }
cat >"$WORK/wrap.sh" <<'EOF'
#!/bin/sh
mount -t tmpfs tmpfs /dev/pts || exit 2
MNT="$1"; shift
exec "$MNT/deny-chroot" "$@"
EOF
timeout 600 "$PB" pull "$DYN" >>"$WORK/pull.log" 2>&1 || { note "pull of $DYN FAILED: COULD NOT RUN"; exit 2; }
mkdir -p "$WORK/proj" && echo in >"$WORK/proj/in.txt"
timeout 300 unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$PB" run --rm -v "$WORK/proj:/workspace" --podvm-confine="read=/etc/passwd;write=/workspace" "$DYN" sh -c 'head -1 /etc/passwd; cat /etc/shadow >/dev/null 2>&1; echo "shadow=$?"; cat /workspace/in.txt; echo out > /workspace/out.txt; echo "w=$?"; echo t > /tmp/x 2>/dev/null; echo "tmp=$?"' >"$WORK/c4.out" 2>&1
rc=$?
note "  loader family: exit $rc: $(grep -E '^(shadow|w|tmp)=' "$WORK/c4.out" | tr '\n' ' ')$( [ -f "$WORK/proj/out.txt" ] && echo 'out.txt on the host')"
[ "$rc" -eq 0 ] || bad "the loader-family run exited $rc"
grep -q "^root:" "$WORK/c4.out" || bad "loader family: /etc/passwd did not read"
grep -q "^shadow=1" "$WORK/c4.out" || bad "loader family: /etc/shadow read"
grep -q "^in$" "$WORK/c4.out" || bad "loader family: the volume did not read"
[ -f "$WORK/proj/out.txt" ] || bad "loader family: the write to the volume did not land"
grep -q "^tmp=1" "$WORK/c4.out" || bad "loader family: /tmp was writable"
timeout 300 unshare -Urm sh "$WORK/wrap.sh" "$WORK" "$PB" run --rm --podvm-confine="read=/etc/passwd" "$IMG" true >"$WORK/c4b.out" 2>&1
rc=$?
note "  native-static: exit $rc: $(grep -m1 -- '--podvm-confine' "$WORK/c4b.out" | cut -c1-150)"
[ "$rc" -eq 125 ] || bad "the native-static run exited $rc"
grep -q "dispatcher or an emulator" "$WORK/c4b.out" || bad "the refusal does not name the rung"
note ""

note "== clause 5  run -d and exec take the layer of the record"
"$PB" rm -f c414 >/dev/null 2>&1
timeout 120 "$PB" run -d --name c414 --podvm-confine="read=/etc/passwd" "$IMG" sh -c 'cat /etc/shadow >/dev/null; echo "d-shadow=$?"; sleep 30' >"$WORK/c5.out" 2>&1
rc=$?
sleep 2
timeout 60 "$PB" logs c414 >"$WORK/c5l.out" 2>&1
timeout 60 "$PB" exec c414 sh -c 'cat /etc/shadow >/dev/null; echo "e-shadow=$?"; head -1 /etc/passwd' >"$WORK/c5e.out" 2>&1
"$PB" rm -f c414 >/dev/null 2>&1
note "  run -d: exit $rc; $(grep -h 'shadow=' "$WORK/c5l.out" "$WORK/c5e.out" | tr '\n' ' ')"
[ "$rc" -eq 0 ] || bad "run -d exited $rc"
grep -q "^d-shadow=1" "$WORK/c5l.out" || bad "the detached payload read /etc/shadow"
grep -q "^e-shadow=1" "$WORK/c5e.out" || bad "exec read /etc/shadow"
grep -q "^root:" "$WORK/c5e.out" || bad "exec could not read /etc/passwd"
note ""

note "== clause 6  the built-in profile errand (bailey's policy of issue 170)"
timeout 300 "$PB" probe-inside --profile errand "$IMG" >"$WORK/c6.out" 2>&1
timeout 120 "$PB" run --rm --profile errand "$IMG" sh -c 'ls / >/dev/null 2>&1; echo "list=$?"; cat /etc/shadow >/dev/null 2>&1; echo "shadow=$?"; head -1 /etc/passwd' >"$WORK/c6r.out" 2>&1
note "  probe: $(grep -E '	(nonewprivs|seccomp|unshare-user|mount-tmpfs|bpf|keyctl)	' "$WORK/c6.out" | awk '{print $2"="$3":"$4}' | tr '\n' ' ')"
note "  run:   $(grep -E '^(list|shadow)=' "$WORK/c6r.out" | tr '\n' ' ')"
grep -qE '	nonewprivs	ALLOW	1$' "$WORK/c6.out" || bad "errand: NoNewPrivs is not 1"
grep -qE '	seccomp	ALLOW	2$' "$WORK/c6.out" || bad "errand: the seccomp mode is not 2"
for t in unshare-user mount-tmpfs bpf keyctl; do
	grep -qE "	$t	DENY	" "$WORK/c6.out" || bad "errand: $t is not refused"
done
grep -q "^list=1" "$WORK/c6r.out" || bad "errand: / is listable"
grep -q "^shadow=1" "$WORK/c6r.out" || bad "errand: /etc/shadow reads"
grep -q "^root:" "$WORK/c6r.out" || bad "errand: /etc/passwd does not read"
note ""

if [ "$fail" -eq 0 ]; then
	note "verdict: THE CONFINE LAYER HOLDS THE PAYLOAD, AND EACH CASE WITH NO MECHANISM REFUSES BY NAME"
else
	note "verdict: A CLAUSE DISAGREED"
fi
exit "$fail"
