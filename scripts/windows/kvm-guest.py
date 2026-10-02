#!/usr/bin/env python3
"""Run the KVM guest proof with explicit host inputs. See T-1350."""

import argparse
from datetime import datetime, timezone
from pathlib import Path
import os
import re
import shlex
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent.parent.parent


def guest_path(path):
    resolved = path.resolve(strict=True)
    if os.name != "nt" or len(resolved.drive) != 2 or resolved.drive[1] != ":":
        raise ValueError("the Windows proof requires a file on a local drive")
    return "/mnt/" + resolved.drive[0].lower() + "/" + "/".join(resolved.parts[1:])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--image", required=True, type=Path)
    parser.add_argument("--output", type=Path, default=ROOT / "experiments/results/kvm-guest.txt")
    parser.add_argument("--accept-host-risk", action="store_true",
                        help="the operator accepts nested KVM load on this host")
    parser.add_argument("--unattended", action="store_true",
                        help="run under the T-1609 watchdog instead of requiring the operator present")
    options = parser.parse_args()
    # T-1609. The operator's 2026-10-02 decision permits an unattended run
    # on this host conditional on a watchdog outside the guest. `--accept-host-risk`
    # is the operator-present form; `--unattended` is the guarded form, and
    # it is refused unless the watchdog is actually present and answering,
    # so this flag cannot become a promise nothing keeps.
    watchdog = ROOT / "scripts/windows/kvm-watchdog.py"
    if options.unattended:
        if not options.accept_host_risk:
            print("kvm-guest: --unattended still requires --accept-host-risk: "
                  "the host risk is accepted, and the watchdog bounds it.", file=sys.stderr)
            return 2
        if not watchdog.is_file():
            print(f"kvm-guest: --unattended needs {watchdog}, which is not there. "
                  "See TODO/gate.md T-1609.", file=sys.stderr)
            return 2
        probe = subprocess.run(
            [sys.executable, str(watchdog), "selftest", "--probe-only"],
            capture_output=True, timeout=120,
        )
        if probe.returncode != 0:
            print(f"kvm-guest: the watchdog does not answer, refusing an unguarded run: "
                  f"{probe.stderr.decode('utf-8', errors='replace').strip()}",
                  file=sys.stderr)
            return 2
    if not options.accept_host_risk:
        # A 2026-09-30 run left a KVM emulator that SIGKILL did not remove,
        # and the Windows host then failed. The operator accepted that risk
        # on 2026-10-02: either form above accepts it, and --unattended adds
        # the watchdog that bounds the failure.
        print("kvm-guest: refused: nested KVM can stop the Windows host. "
              "Pass --accept-host-risk, with --unattended to run under the "
              "T-1609 watchdog. See docs/limits.md.",
              file=sys.stderr)
        return 2
    tool = shutil.which("wsl-toolkit")
    if tool is None:
        print("kvm-guest: wsl-toolkit is not on PATH", file=sys.stderr)
        return 2
    guard = None
    try:
        binary = guest_path(options.binary)
        image = guest_path(options.image)
        if not options.binary.is_file() or not options.image.is_file():
            raise ValueError("both inputs must be regular files")
        body = (ROOT / "experiments/lib/kvm-owned.sh").read_text(encoding="utf-8")
        body += (ROOT / "experiments/lib/kvm-guest-base.sh").read_text(encoding="utf-8")
        prefix = (
            "PODBOX_BIN_SRC=" + shlex.quote(binary) + "\n"
            "IMG_HOST=" + shlex.quote(image) + "\n"
        )
        with tempfile.TemporaryDirectory(prefix="podbox-kvm-proof-") as temporary:
            script = Path(temporary) / "guest.sh"
            script.write_text(prefix + body, encoding="utf-8", newline="\n")
            command = [tool, "--instance", "podbox", "base", "exec"]
            if guard is None and options.unattended:
                # T-1609. The proof runs detached so the watchdog has a
                # session id to watch, and the watchdog's own bound sits
                # above this one, so it is the last resort rather than the
                # normal path. Without `--unattended` this is the original
                # synchronous call and no guard runs, which is the
                # operator-present form.
                command += ["--detach", "--json"]
            if not options.unattended:
                command += ["--timeout", "65m", "--script", str(script)]
                result = subprocess.run(
                    command, cwd=ROOT, capture_output=True, timeout=4200,
                )
            else:
                launched = subprocess.run(
                    command + ["--script", str(script)],
                    cwd=ROOT, capture_output=True, timeout=300,
                )
                if launched.returncode != 0:
                    result = launched
                else:
                    import json as _json
                    session = _json.loads(launched.stdout.decode("utf-8"))["id"]
                    guard = subprocess.Popen(
                        [sys.executable, str(watchdog), "sweep", session,
                         "--seconds", "4800"],
                        cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                    )
                    print(f"kvm-guest: session {session} under watchdog {guard.pid}")
                    # `logs --follow` returns the session's exit code, so the
                    # proof's verdict still comes from the tool that ran it.
                    result = subprocess.run(
                        [tool, "--instance", "podbox", "logs", session, "--follow"],
                        cwd=ROOT, capture_output=True, timeout=4500,
                    )
                    guard.wait(timeout=180)
        output = result.stdout.decode("utf-8", errors="replace")
        output += result.stderr.decode("utf-8", errors="replace")
        # Paths are host conditions. Publish stable labels and state this conversion.
        for path, label in ((binary, "$BINARY"), (image, "$IMAGE")):
            output = output.replace(path, label)
        output = re.sub(r"/home/[^/\s]+/podbox-kvm\.[A-Za-z0-9]+", "$SCRATCH", output)
        output = re.sub(r"/home/[^/\s]+", "$BASE_HOME", output)
        output = re.sub(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]",
                        lambda match: "\\x%02x" % ord(match.group()), output)
        lines = []
        for line in output.splitlines():
            if "instance podbox: distribution" not in line:
                lines.append(line)
        stamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        report = (
            "== conditions\n" + stamp + "\ninstance=podbox\n"
            "Host input paths use $BINARY and $IMAGE. Base scratch uses $SCRATCH.\n"
            "Control bytes use \\xNN labels.\n"
            "\n" + "\n".join(lines) + f"\nproof exit: {result.returncode}\n"
        )
        options.output.parent.mkdir(parents=True, exist_ok=True)
        options.output.write_text(report, encoding="utf-8", newline="\n")
        print(report, end="")
        if result.returncode == 0 and "verdict KVM-GUEST-OK" in report:
            return 0
        return 2 if result.returncode == 2 else 1
    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
        print(f"kvm-guest: cannot run: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
