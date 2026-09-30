#!/usr/bin/env python3
"""Drive gate failure output, lost-output mutations, and skip states. T-1351."""

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
CHECKS = ('check-docs', 'check-markers', 'check-one-home', 'check-placeholders',
          'check-control-bytes', 'check-changelog', 'check-attribution',
          'check-no-secrets', 'check-remote-items')
MARKER = 'FIXTURE-LATE-DIAGNOSTIC'


def run(command, directory):
    return subprocess.run(command, cwd=directory, capture_output=True,
                          text=True, encoding='utf-8', errors='replace', timeout=90)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--powershell', action='store_true')
    options = parser.parse_args()
    print('== conditions')
    print(datetime.now(timezone.utc).isoformat())
    print(f'host={platform.system()} python={platform.python_version()}')
    print('Inputs: fixed stub-v1 checks; late message after 20 control lines.')
    shell = shutil.which('sh')
    powershell = shutil.which('pwsh')
    if not shell or (options.powershell and not powershell):
        print('cannot run: required interpreter is absent')
        return 2
    with tempfile.TemporaryDirectory(prefix='podbox-gate-output-') as temporary:
        directory = Path(temporary)
        for name in CHECKS:
            (directory / (name + '.sh')).write_text('exit 0\n', encoding='utf-8')
            (directory / (name + '.ps1')).write_text('exit 0\n', encoding='utf-8')
        for kind, interpreter, flags, json_flag in (
            ('sh', shell, ['--fast'], '--json'),
            ('ps1', powershell, ['-Fast'], '-Json'),
        ):
            if kind == 'ps1' and not options.powershell:
                print('PowerShell profile: not requested; Windows proof uses --powershell')
                continue
            source = (ROOT / f'scripts/common/check-gate.{kind}').read_text(encoding='utf-8')
            runner = directory / ('check-gate.' + kind)
            runner.write_text(source, encoding='utf-8', newline='\n')
            stub = directory / ('check-docs.' + kind)
            noise = '\n'.join(("echo control" if kind == 'sh' else "Write-Output 'control'") for _ in range(20))
            fail = noise + '\n' + (f'echo {MARKER}' if kind == 'sh' else f"Write-Output '{MARKER}'") + '\nexit 1\n'
            stub.write_text(fail, encoding='utf-8')
            prefix = [interpreter] if kind == 'sh' else [interpreter, '-NoProfile', '-File']
            command = prefix + [runner.as_posix()]
            result = run(command + flags, directory)
            assert result.returncode == 1 and MARKER in result.stdout, (kind, result.stdout, result.stderr)
            result = run(command + flags + [json_flag], directory)
            assert result.returncode == 1 and json.loads(result.stdout)['failed'] == 1
            assert MARKER not in result.stdout
            if kind == 'sh':
                mutation = source.replace('sed \'s/^/          /\' "$OUT/log" ;;', 'sed \'s/^/          /\' "$OUT/log" | head -12 ;;')
            else:
                mutation = source.replace('Get-Content -LiteralPath $logFile -ErrorAction', 'Get-Content -LiteralPath $logFile -TotalCount 12 -ErrorAction')
            assert mutation != source, 'mutation did not reach its runner'
            runner.write_text(mutation, encoding='utf-8', newline='\n')
            result = run(command + flags, directory)
            assert result.returncode == 1 and MARKER not in result.stdout
            runner.write_text(source, encoding='utf-8', newline='\n')
            stub.write_text('exit 0\n', encoding='utf-8')
            result = run(command + flags, directory)
            assert result.returncode == 0 and MARKER not in result.stdout
            # The same unavailable twin is skipped on both platforms.
            (directory / 'check-twins.sh').write_text('echo unavailable\nexit 2\n', encoding='utf-8')
            result = run(command + [json_flag], directory)
            verdict = json.loads(result.stdout)
            assert result.returncode == 0 and verdict['skipped'] == 1 and verdict['failed'] == 0
            strict = '--strict' if kind == 'sh' else '-Strict'
            result = run(command + [json_flag, strict], directory)
            assert result.returncode == 1 and json.loads(result.stdout)['skipped'] == 1
            print(f'ok: {kind} late failure, lost-output mutation, JSON, clean control, and unavailable twin')
    print('verdict GATE-DIAGNOSTICS-OK')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
