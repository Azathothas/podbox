#!/usr/bin/env python3
"""Watch the KVM proof from outside the guest and remove an emulator that
outlives its bound. T-1609.

The 2026-09-30 failure is the reason this exists. A proof left a 4 GiB
emulator that SIGKILL did not remove and the Windows host then failed. The
cleanup that should have caught it is `stop_owned_emulators`
(`experiments/lib/kvm-owned.sh`), and it is unreachable exactly when it is
needed: it runs inside the guest and is reached through that guest's own
EXIT trap. A guest that hangs, is killed, or is reaped takes its trap with
it, and the emulator outlives the session that started it.

So the guard is here, on the Windows side, where the session id is visible
and where nothing the guest does can take the guard down with it.

Two properties make it safe to leave running unattended:

  - It selects an emulator by scratch path, not by name. The guest names
    every emulator it owns with its own `$KVM/` directory, so the watchdog
    removes exactly the emulator this run started and nothing else. An
    emulator belonging to any other session is invisible to it.
  - Its bound sits above the driver's own deadline, so it is the last
    resort and not the normal path. The driver's guest script still owns
    cleanup on the happy path.

Usage:
    kvm-watchdog.py arm   -- session-id [--seconds N]
    kvm-watchdog.py selftest
    kvm-watchdog.py unarm -- session-id

Exit codes: 0 nothing needed removing and the bound held, 3 the watchdog
fired and removed an emulator that outlived its bound, 2 it could not
run. Firing is not a failure of the tool; it is the tool working, and a
caller reading it as an error learns the opposite of what happened.
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

TOOL = "wsl-toolkit"
INSTANCE = "podbox"

# Above the driver's own 4200 s subprocess timeout in
# `scripts/windows/kvm-guest.py`, so the driver is always the first thing
# to give up and the watchdog only ever runs on a session that already
# overran it. A guest that dies without its trap fires this; a guest that
# finishes never reaches it.
DEFAULT_BOUND_SECONDS = 4800

# The guest-side ownership rule, restated rather than reused: the guest
# names its scratch under $HOME, and only this path can be this run's.
GUEST_SCRATCH = re.compile(r"/home/[^/\s]+/podbox-kvm\.[A-Za-z0-9]+")

RUN_PODBOX = "podbox-kvm-watchdog.sh"


def state_dir():
    return Path(os.environ.get("LOCALAPPDATA", Path.home())) / "wsl-toolkit"


def arm_file():
    return state_dir() / "podbox-kvm-watchdog.json"


def tool():
    found = shutil.which(TOOL)
    if found is None:
        raise RuntimeError(f"{TOOL} is not on PATH")
    return found


def base_exec(command, timeout=120):
    """One command inside the base. Returns (rc, text)."""
    result = subprocess.run(
        [tool(), "--instance", INSTANCE, "base", "exec", "-c", command],
        capture_output=True,
        timeout=timeout,
    )
    text = result.stdout.decode("utf-8", errors="replace")
    text += result.stderr.decode("utf-8", errors="replace")
    return result.returncode, text


def owned_emulators():
    """Emulators belonging to a podbox KVM scratch, by scratch path.

    Selection is on the scratch directory the emulator was launched with,
    read from each process's own argument vector, and the executable is
    identified through `/proc/PID/exe` rather than `ps` output.

    `ps -eo args` was the first selector and it was wrong here, measured:
    this `ps` renders the interpreter, so a process that sets its argv[0]
    still reads as `sh /path/linger`, and a match on column two would have
    found nothing while reporting that nothing was wrong. `/proc/PID/exe`
    is the kernel's own answer to what the process is executing and cannot
    be set by the process itself, so it is the honest signal. The scratch
    is matched by the proof's own `podbox-kvm.` pattern rather than one
    run's `$KVM/`, because the watchdog outlives any single run.
    """
    script = (
        "for d in /proc/[0-9]*; do "
        "  p=${d#/proc/}; "
        "  e=$(readlink $d/exe 2>/dev/null) || continue; "
        "  case $e in *qemu-system-x86_64|*qemu-system-*) ;; *) continue ;; esac; "
        # `tr '\0'` is wrong across this boundary: the NUL cannot survive a
        # Windows command argument, and the escaping that replaces it sends
        # a literal backslash-zero that translates nothing, so `case` never
        # matched and the guard reported "nothing running" while a guest ran.
        # Reading the NUL-separated vector with the shell's own quoting
        # needs no escape at all.
        "  c=$(cat $d/cmdline 2>/dev/null) || continue; "
        "  case $c in *podbox-kvm.*) echo $p ;; esac; "
        "done"
    )
    _, text = base_exec(script)
    pids = []
    for line in text.splitlines():
        line = line.strip()
        if line.isdigit():
            pids.append(line)
    return pids


def scratch_dirs():
    _, text = base_exec("ls -d /home/*/podbox-kvm.* 2>/dev/null || true")
    found = []
    for line in text.splitlines():
        candidate = line.strip()
        if GUEST_SCRATCH.fullmatch(candidate):
            found.append(candidate)
    return found


def remove_emulator(pids):
    """TERM, then KILL, then report what survived. Nothing is guessed.

    Each signal re-derives the pid list immediately before it is sent and
    signals only pids still present in it. A bare `kill {pid}` taken from
    an earlier scan can land on an unrelated process that inherited the
    number inside the 15 s grace, and on this host the process it must not
    touch is somebody else's guest.
    """
    for pid in pids:
        if pid in owned_emulators():
            base_exec(f"kill {pid} 2>/dev/null || true")
    deadline = time.time() + 15
    while time.time() < deadline:
        if not owned_emulators():
            return []
        time.sleep(1)
    remaining = owned_emulators()
    for pid in remaining:
        if pid in owned_emulators():
            base_exec(f"kill -KILL {pid} 2>/dev/null || true")
    time.sleep(2)
    return owned_emulators()


def wait_for(session, bound):
    """Wait on the session. Returns its exit code, 124 on timeout.

    `--instance` is load-bearing, not decoration. Omitted, the global
    `wsl-toolkit wait` resolves the instance by `auto`, and this machine
    has seven: acc, base, muse, nobase, pg-toolkit, podbox,
    podbox-migrate. Measured on a live session: without `--instance` the
    wait answered `no such job` with rc 2 in 0 s, collapsing the whole
    bound; with it, rc 124 after the real timeout elapsed. A watchdog
    whose bound never starts is the one defect this tool exists to prevent.
    """
    result = subprocess.run(
        [tool(), "--instance", INSTANCE, "wait", session, "--timeout", f"{bound}s"],
        capture_output=True,
    )
    return result.returncode


def stop_session(session):
    """Stop a detached session, on the same instance that started it."""
    return subprocess.run(
        [tool(), "--instance", INSTANCE, "stop", session],
        capture_output=True, timeout=60,
    )


def sweep(session, bound):
    """The bound. Fires only on a session that overran it.

    A session that ends inside its bound is a success, and its scratch is
    evidence: `experiments/lib/kvm-guest-base.sh` prints the `$KVM`
    listing containing `ver.txt`, `setup.txt`, `doctor.txt`, `e42.txt`,
    `seam.txt` and the serial logs, and the proof's whole value is those
    bytes. So the scratch is removed only when the wait timed out, and
    never when the run finished on its own.
    """
    rc = wait_for(session, bound)
    timed_out = rc == 124
    pids = owned_emulators()
    scratch = scratch_dirs()
    report = {
        "session": session,
        "wait_rc": rc,
        "timed_out": timed_out,
        "emulators_found": pids,
        "scratch_found": scratch,
        # An emulator outliving the session is always worth reporting.
        # Scratch alone is not a failure: on a clean exit it is the
        # evidence the proof just produced.
        "fired": bool(pids or (timed_out and scratch)),
    }
    if pids:
        report["survivors"] = remove_emulator(pids)
    if timed_out and scratch:
        # Iterate the validated list, not a fresh glob. A directory the
        # validator rejects is not this proof's to delete.
        for directory in scratch:
            base_exec(f"rm -rf '{directory}' 2>/dev/null || true")
        report["scratch_left"] = scratch_dirs()
    return report


def arm(session, bound):
    path = arm_file()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps({"session": session, "bound_seconds": bound, "armed_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}, indent=2),
        encoding="utf-8",
    )
    print(f"{RUN_PODBOX}: armed for {session} bound {bound}s")
    return 0


def unarm(session):
    path = arm_file()
    if not path.is_file():
        print(f"{RUN_PODBOX}: nothing armed")
        return 0
    record = json.loads(path.read_text(encoding="utf-8"))
    if record.get("session") != session:
        print(f"{RUN_PODBOX}: armed for {record.get('session')}, not {session}; left alone")
        return 0
    path.unlink()
    print(f"{RUN_PODBOX}: disarmed {session}")
    return 0


def detached_session(script_path):
    """Start a detached base session and return its id.

    The answer is pretty-printed multi-line JSON, so the whole stream is
    parsed rather than its last line. A dropped session id here would mean
    an unwatchable emulator, which is the one failure this exists to stop.
    """
    launched = subprocess.run(
        [tool(), "--instance", INSTANCE, "base", "exec", "--detach", "--json",
         "--script", str(script_path)],
        capture_output=True,
        timeout=120,
    )
    if launched.returncode != 0:
        raise RuntimeError(
            "the base refused the detached session: "
            + launched.stderr.decode("utf-8", errors="replace").strip()
        )
    answer = json.loads(launched.stdout.decode("utf-8", errors="replace"))
    session = answer.get("id")
    if not session:
        raise RuntimeError(f"no session id in the detached answer: {answer}")
    return session


def selftest():
    """Prove the guard, both arms, with a process shaped like an emulator.

    Arm one: a session that ends on its own with nothing running must
    leave the watchdog quiet. Arm two: a session that lingers while a
    process wearing an emulator's name and carrying a proof scratch path
    sits in the guest must make the watchdog remove exactly that process
    and nothing else.

    The fixture is a shell process, not a real qemu: what is under test is
    the guard's selection and its removal, and a real 4 GiB guest is not
    what a self-test should start. It wears the name and the scratch path
    because those are the two things the guard selects on.
    """
    failures = []

    print("== control: nothing to remove")
    # Clear any fixture an earlier aborted run left behind. A self-test that
    # starts from whatever the last failure left behind is not a control,
    # and this suite has been interrupted before.
    stale = owned_emulators()
    if stale:
        print(f"..   clearing {len(stale)} stale fixture(s) from an earlier run")
        remove_emulator(stale)
    for directory in scratch_dirs():
        base_exec(f"rm -rf '{directory}' 2>/dev/null || true")
    before = owned_emulators()
    if before:
        failures.append(f"an emulator was already running before the selftest: {before}")
    report = sweep("selftest-absent-session", 3)
    if report["fired"]:
        failures.append("the control fired with no emulator present")
    print(f"ok   control stayed quiet, found {report['emulators_found']}")

    print("== fault: a lingering emulator is removed")
    with tempfile.TemporaryDirectory(prefix="podbox-kvm-watchdog-selftest-") as tmp:
        script = Path(tmp) / "linger.sh"
        # The fixture links the real qemu binary under a scratch path and
        # runs it on a real disk, so `/proc/PID/exe` and
        # `/proc/PID/cmdline` carry exactly what a real guest carries and
        # the process stays alive long enough to be caught. A shell process
        # wearing a name would not do: the guard reads the executable the
        # kernel reports, which argv[0] cannot change. Nor would a missing
        # disk: qemu then exits inside a second and there is nothing to
        # guard, which is what the first fixture attempt measured.
        #
        # The disk is a raw file made with `truncate`, not a qcow2 made
        # with `qemu-img`, because `qemu-img` belongs to the `qemu-img`
        # package and the proof does not need it. The fixture must not
        # widen the base's package set to test the guard.
        script.write_text(
            "#!/usr/bin/env sh\n"
            "d=/home/toolkit/podbox-kvm.selftest\n"
            "mkdir -p \"$d\"\n"
            "truncate -s 64M \"$d/base.img\" || exit 2\n"
            "ln -sf /usr/bin/qemu-system-x86_64 \"$d/qemu-system-x86_64\"\n"
            "exec \"$d/qemu-system-x86_64\" -accel kvm -display none "
            "-serial none -monitor none \"$d/base.img\"\n",
            encoding="utf-8",
            newline="\n",
        )
        session = None
        try:
            session = detached_session(script)
            print(f"ok   detached session {session}")
            # Wait for the fixture to be visible rather than sleeping a
            # guessed interval. A fixed wait measured as the whole bug here:
            # three seconds was sometimes short enough that the check ran
            # before qemu had started and reported "nothing running" while
            # the guest came up afterwards. The condition is what is being
            # proven, so the wait polls for it and names the bound it gave up
            # at.
            found = []
            deadline = time.time() + 45
            while time.time() < deadline:
                found = owned_emulators()
                if found:
                    break
                time.sleep(1)
            if not found:
                failures.append("the fixture emulator was not visible to the guard")
                print("FAIL the guard cannot see the fixture")
            else:
                print(f"ok   the guard sees the fixture: {found}")
            report = sweep(session, 2)
            if not report["fired"]:
                failures.append("the watchdog did not fire on a lingering emulator")
                print("FAIL the watchdog stayed quiet on a lingering emulator")
            else:
                print(f"ok   the watchdog fired and removed {report['emulators_found']}")
            if report.get("survivors"):
                failures.append(f"the emulator survived removal: {report['survivors']}")
            else:
                print("ok   no emulator survived")
            if report.get("scratch_left"):
                failures.append(f"scratch survived: {report['scratch_left']}")
            else:
                print("ok   no scratch survived")
        finally:
            if session:
                try:
                    stop_session(session)
                except Exception:
                    pass
            # `wsl-toolkit stop` signals the session's process group, which
            # measured as not reaching this fixture: an emulator outlived it
            # in testing. So the cleanup the self-test relies on is the same
            # guard it just tested, applied unconditionally rather than in a
            # `try`, and its result is reported instead of assumed.
            leftovers = owned_emulators()
            if leftovers:
                survivors = remove_emulator(leftovers)
                if survivors:
                    print(f"FAIL cleanup left {survivors} running")
                    failures.append(f"cleanup left emulators running: {survivors}")
                else:
                    print(f"ok   cleanup removed {len(leftovers)} fixture process(es)")
            base_exec("rm -rf /home/*/podbox-kvm.selftest 2>/dev/null || true")

    print("== isolation: an emulator that is not ours is left alone")
    # The arm that makes the fault arm safe. Without it, "removes the
    # emulator" could be satisfied by "removes every emulator", which on a
    # shared base would kill someone else's guest. Measured here: the guard
    # sees nothing and fires nothing while an unrelated qemu runs.
    with tempfile.TemporaryDirectory(prefix="podbox-kvm-watchdog-foreign-") as tmp:
        foreign = Path(tmp) / "foreign.sh"
        foreign.write_text(
            "#!/usr/bin/env sh\n"
            "d=/home/toolkit/someone-elses-run\n"
            "mkdir -p \"$d\"\n"
            "truncate -s 64M \"$d/other.img\" || exit 2\n"
            "ln -sf /usr/bin/qemu-system-x86_64 \"$d/qemu-system-x86_64\"\n"
            "exec \"$d/qemu-system-x86_64\" -accel kvm -display none "
            "-serial none -monitor none \"$d/other.img\"\n",
            encoding="utf-8",
            newline="\n",
        )
        foreign_session = None
        try:
            foreign_session = detached_session(foreign)
            # `[q]emu-system`, never `qemu-system`. Measured with zero
            # emulators running, `grep -c qemu-system` answers 1 because
            # the pipeline matches its own command line, so an arm that
            # waited for its fixture started would break instantly and an
            # arm that checked the fixture survived could never fail.
            # Both were live defects: this arm asserted nothing.
            def foreign_alive():
                _, text = base_exec('ps -eo args | grep -c "[q]emu-system" || true')
                for line in text.splitlines():
                    line = line.strip()
                    if line.isdigit():
                        return int(line) > 0
                return False

            deadline = time.time() + 45
            while time.time() < deadline and not foreign_alive():
                time.sleep(1)
            if not foreign_alive():
                failures.append("the foreign fixture never started, so the arm proved nothing")
                print("FAIL the foreign emulator never started")
            else:
                print("ok   the foreign emulator is running")
            if owned_emulators():
                failures.append("the guard selected an emulator outside the proof's scratch")
                print("FAIL the guard claimed a foreign emulator")
            else:
                print("ok   the guard does not select a foreign emulator")
            result = sweep(foreign_session, 2)
            if result["fired"]:
                failures.append("the watchdog fired on a foreign emulator")
                print("FAIL the watchdog fired on someone else's guest")
            else:
                print("ok   the watchdog stayed quiet")
            if not foreign_alive():
                failures.append("the foreign emulator was removed by the guard")
                print("FAIL the guard removed an emulator it does not own")
            else:
                print("ok   the foreign emulator is still running")
        finally:
            if foreign_session:
                try:
                    stop_session(foreign_session)
                except Exception:
                    pass
            base_exec("pkill -f someone-elses-run 2>/dev/null || true")
            base_exec("rm -rf /home/toolkit/someone-elses-run 2>/dev/null || true")

    print("== verdict")
    if failures:
        for line in failures:
            print(f"FAIL {line}")
        print(f"verdict {RUN_PODBOX.upper()}-FAIL")
        return 1
    print(f"verdict {RUN_PODBOX.upper()}-OK")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    arm_parser = sub.add_parser("arm")
    arm_parser.add_argument("session")
    arm_parser.add_argument("--seconds", type=int, default=DEFAULT_BOUND_SECONDS)
    unarm_parser = sub.add_parser("unarm")
    unarm_parser.add_argument("session")
    selftest_parser = sub.add_parser("selftest")
    selftest_parser.add_argument(
        "--probe-only", action="store_true",
        help="answer whether the guard can reach the base at all, without launching anything",
    )
    sweep_parser = sub.add_parser("sweep")
    sweep_parser.add_argument("session")
    sweep_parser.add_argument("--seconds", type=int, default=DEFAULT_BOUND_SECONDS)
    options = parser.parse_args()
    try:
        if options.command == "arm":
            return arm(options.session, options.seconds)
        if options.command == "unarm":
            return unarm(options.session)
        if options.command == "selftest":
            if options.probe_only:
                # What `--unattended` asks before it starts a guest: can
                # this guard reach the base and see processes at all? It
                # launches nothing, so it is safe to run before every
                # guarded proof.
                pids = owned_emulators()
                print(f"watchdog: base reachable, {len(pids)} owned emulator(s) visible")
                return 0
            return selftest()
        report = sweep(options.session, options.seconds)
        print(json.dumps(report, indent=2))
        return 3 if report["fired"] else 0
    except subprocess.TimeoutExpired:
        print(f"{RUN_PODBOX}: could not run: the base did not answer in time", file=sys.stderr)
        return 2
    except (OSError, RuntimeError, json.JSONDecodeError) as error:
        print(f"{RUN_PODBOX}: could not run: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())