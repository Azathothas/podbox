#!/usr/bin/env python3
"""Read exact-commit CI and optional release and branch acceptance. No remote writes."""

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent.parent
REPO = 'Azathothas/podbox'


def command(arguments):
    environment = os.environ.copy()
    environment.update(GIT_TERMINAL_PROMPT='0', GH_PROMPT_DISABLED='1')
    result = subprocess.run(arguments, cwd=ROOT, capture_output=True,
                            text=True, encoding='utf-8', timeout=120,
                            stdin=subprocess.DEVNULL, env=environment)
    if result.returncode:
        raise RuntimeError('required command failed: ' + arguments[0])
    return result.stdout.strip()


def workflow(name, commit, branch):
    runs = json.loads(command(['gh', 'run', 'list', '--repo', REPO,
        '--workflow', name, '--limit', '30', '--json',
        'headSha,headBranch,status,conclusion,url']))
    matching = [run for run in runs if run['headSha'] == commit and run['headBranch'] == branch]
    if not matching or matching[0]['status'] != 'completed':
        raise RuntimeError('the exact-commit workflow is not complete: ' + name)
    run = matching[0]
    print(f"{name}: {run['conclusion']} {run['url']}")
    if run['conclusion'] != 'success':
        raise ValueError('the exact-commit workflow failed: ' + name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--release')
    parser.add_argument('--deleted-branch')
    options = parser.parse_args()
    print('== conditions')
    print(datetime.now(timezone.utc).isoformat())
    print('Scope: remote refs, workflow conclusions, and complete asset names.')
    print('Signature content verification uses scripts/verify-release.sh separately.')
    try:
        remote = command(['git', 'remote', 'get-url', 'origin'])
        if remote not in ('https://github.com/' + REPO + '.git', 'https://github.com/' + REPO):
            raise ValueError('origin differs from the approved HTTPS remote')
        commit = command(['git', 'rev-parse', 'HEAD'])
        print('local commit: ' + commit)
        remote_head = command(['git', 'ls-remote', '--heads', 'origin', 'refs/heads/main']).split()
        if not remote_head or remote_head[0] != commit:
            raise ValueError('remote main differs from local HEAD')
        workflow('gate.yml', commit, 'main')
        if options.deleted_branch:
            refs = command(['git', 'ls-remote', '--heads', 'origin', 'refs/heads/' + options.deleted_branch])
            if refs:
                raise ValueError('obsolete branch is still present')
            print('deleted branch absent: ' + options.deleted_branch)
        if options.release:
            if not re.fullmatch(r'v[0-9]+\.[0-9]+\.[0-9]+-beta\.[0-9]+', options.release):
                raise ValueError('release tag format differs')
            tag_commit = command(['git', 'rev-parse', options.release + '^{commit}'])
            print('release commit: ' + tag_commit)
            workflow('nightly.yml', tag_commit, options.release)
            release = json.loads(command(['gh', 'release', 'view', options.release,
                '--repo', REPO, '--json', 'tagName,assets,url,isPrerelease']))
            recipe = command(['git', 'show', options.release + ':.github/workflows/nightly.yml'])
            architectures = re.findall(r'^\s+- arch:\s*(\S+)\s*$', recipe, re.M)
            if not architectures or len(set(architectures)) != len(architectures):
                raise ValueError('cannot read a unique architecture matrix')
            assets = {asset['name']: asset['size'] for asset in release['assets']}
            expected = set()
            for architecture in architectures:
                for artifact in ('podbox-' + architecture, 'podbox-ssh-' + architecture + '.tar.gz'):
                    expected.update((artifact, artifact + '.sha256', artifact + '.sigstore'))
            if release['tagName'] != options.release or not release['isPrerelease']:
                raise ValueError('release identity or pre-release state differs')
            missing = sorted(name for name in expected if not assets.get(name))
            if missing:
                raise ValueError('missing or empty assets: ' + ', '.join(missing))
            print(f"release: {release['url']}; {len(architectures)} architectures; {len(expected)} required non-empty assets")
        print('verdict PUBLICATION-OK')
        return 0
    except ValueError as error:
        print('FAIL: ' + str(error))
        return 1
    except (OSError, KeyError, RuntimeError, subprocess.TimeoutExpired) as error:
        print('cannot run: ' + str(error))
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
