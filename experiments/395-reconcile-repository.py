#!/usr/bin/env python3
"""Which changes does the retained publish branch add to main?"""

from datetime import datetime, timezone
from pathlib import Path
import argparse
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
PUBLISH = 'origin/publish/20260926T052210Z'


def git(*arguments):
    result = subprocess.run(['git', *arguments], cwd=ROOT,
        capture_output=True, text=True, encoding='utf-8', timeout=60)
    if result.returncode:
        raise RuntimeError(f'git {arguments[0]} exit {result.returncode}')
    return result.stdout.rstrip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--expect-deleted', action='store_true')
    options = parser.parse_args()
    print('== conditions')
    print(datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'))
    print('commit main:', git('rev-parse', 'main'))
    print('commit origin/main:', git('rev-parse', 'origin/main'))
    print('scope: Git history and patch comparison; runtime acceptance is separate')
    print('== remote branch refs')
    print(git('for-each-ref', '--format=%(refname:short) %(objectname)', 'refs/remotes/origin'))
    result = subprocess.run(['git', 'show-ref', '--verify', '--quiet',
        'refs/remotes/' + PUBLISH], cwd=ROOT, timeout=30)
    if options.expect_deleted:
        if result.returncode != 1:
            print('verdict REPOSITORY-FAIL: publish ref is still present or unreadable')
            return 1
        print('verdict REPOSITORY-OK: obsolete publish ref is absent after fetch')
        return 0
    if result.returncode:
        print('cannot run: the comparison requires the retained publish ref')
        return 2
    print('== patch identity comparison')
    print(git('cherry', 'main', PUBLISH))
    print('== equivalent series, with remaining context difference')
    print(git('range-diff', '--no-color', 'ce17a733^..82c12ba1', '82d4bff^..1c19cea'))
    print('verdict REPOSITORY-READ: compare the first patch source; do not infer identity from ancestry')
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f'cannot read repository state: {error}', file=sys.stderr)
        sys.exit(2)
