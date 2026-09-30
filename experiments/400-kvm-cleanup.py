#!/usr/bin/env python3
"""Prove that the KVM cleanup selects only its owned emulator argv. T-1350."""

from datetime import datetime, timezone
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent


def main():
    print('== conditions')
    print(datetime.now(timezone.utc).isoformat())
    print('Inputs: Linux processes with controlled argv; no guest or licensed image.')
    if sys.platform != 'linux':
        print('cannot run: the process selector requires Linux ps')
        return 2
    with tempfile.TemporaryDirectory(prefix='podbox-kvm-owned-') as temporary:
        scratch = Path(temporary)
        script = scratch / 'select.sh'
        helper = (ROOT / 'experiments/lib/kvm-owned.sh').read_text(encoding='utf-8')
        script.write_text(helper + '\nKVM=' + shlex.quote(str(scratch / 'owned')) +
                          '\ncase "$1" in list) owned_emulators ;; stop) stop_owned_emulators ;; esac\n',
                          encoding='utf-8', newline='\n')
        def call(action):
            return subprocess.run(['sh', str(script), action], capture_output=True,
                                  text=True, timeout=20, check=True).stdout.split()
        assert call('list') == [], 'the observer matched itself'
        processes = []
        try:
            for name in ('owned', 'other'):
                process = subprocess.Popen(['qemu-system-x86_64', '-c',
                    'import time; time.sleep(30)', str(scratch / name) + '/disk'],
                    executable=sys.executable, stdin=subprocess.DEVNULL,
                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                processes.append(process)
            deadline = time.monotonic() + 5
            while call('list') != [str(processes[0].pid)]:
                if time.monotonic() >= deadline:
                    raise AssertionError('the owned process was not selected alone')
                time.sleep(.05)
            call('stop')
            assert processes[0].wait(timeout=5) < 0
            assert processes[1].poll() is None, 'an unrelated process was stopped'
            assert call('list') == []
            print('ok: observer excluded; owned process stopped; unrelated process retained')
        finally:
            for process in processes:
                if process.poll() is None:
                    process.kill()
                process.wait(timeout=5)
    print('verdict KVM-CLEANUP-OK')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
