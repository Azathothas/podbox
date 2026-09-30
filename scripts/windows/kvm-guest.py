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
                        help="the operator is present and accepts nested KVM load on this host")
    options = parser.parse_args()
    if not options.accept_host_risk:
        # A 2026-09-30 run left a KVM emulator that SIGKILL did not remove,
        # and the Windows host then failed. An unattended agent must not start it.
        print("kvm-guest: refused: nested KVM can stop the Windows host. "
              "An operator who is present passes --accept-host-risk. See docs/limits.md.",
              file=sys.stderr)
        return 2
    tool = shutil.which("wsl-toolkit")
    if tool is None:
        print("kvm-guest: wsl-toolkit is not on PATH", file=sys.stderr)
        return 2
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
            result = subprocess.run(
                [tool, "--instance", "podbox", "base", "exec", "--timeout", "65m", "--script", str(script)],
                cwd=ROOT, capture_output=True, timeout=4200,
            )
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
