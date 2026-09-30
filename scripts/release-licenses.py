#!/usr/bin/env python3
"""Retain licence text for the locked Cargo package set. T-1405."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    options = parser.parse_args()
    try:
        # Include the separately built interposer as well as the workspace.
        packages = {}
        for manifest in ('Cargo.toml', 'crates/podbox-interpose/Cargo.toml'):
            result = subprocess.run(
                ['cargo', 'metadata', '--locked', '--format-version', '1', '--manifest-path', manifest],
                cwd=ROOT, capture_output=True, text=True, encoding='utf-8', timeout=180, check=True,
            )
            for package in json.loads(result.stdout)['packages']:
                if package.get('source'):
                    packages[(package['name'], package['version'])] = package
        if not packages:
            raise ValueError('the locked package set is empty')
        records = []
        for (name, version), package in sorted(packages.items()):
            if not re.fullmatch(r'[A-Za-z0-9_.+-]+', name + version):
                raise ValueError('invalid package identifier')
            source = Path(package['manifest_path']).parent.resolve()
            texts = set()
            for path in source.iterdir():
                if path.name.lower().startswith(('license', 'licence', 'copying', 'notice')):
                    if path.is_file():
                        texts.add(path)
                    elif path.is_dir():
                        texts.update(p for p in path.rglob('*') if p.is_file())
            if package.get('license_file'):
                texts.add(source / package['license_file'])
            if not texts:
                raise ValueError(f'no retained licence text: {name} {version}')
            destination = options.output / (name + '-' + version)
            destination.mkdir(parents=True, exist_ok=True)
            files = []
            for path in sorted(texts):
                resolved = path.resolve(strict=True)
                if not resolved.is_relative_to(source):
                    raise ValueError(f'licence path leaves package: {name} {version}')
                relative = resolved.relative_to(source)
                target = destination / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(resolved, target)
                files.append({'path': relative.as_posix(), 'sha256': hashlib.sha256(target.read_bytes()).hexdigest()})
            records.append({'name': name, 'version': version, 'licence': package.get('license'), 'files': files})
        options.output.mkdir(parents=True, exist_ok=True)
        (options.output / 'inventory.json').write_text(json.dumps(records, indent=2) + '\n', encoding='utf-8')
        print(f'release-licenses: retained texts for {len(records)} locked registry packages')
        return 0
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f'release-licenses: cannot produce inventory: {error}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
