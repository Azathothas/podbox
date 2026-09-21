#!/bin/sh
# 162-tar-symlink-modes.sh: does a tarball carrying symlinks unpack under
# podbox with the engine control beside it?
#
# Status: acceptance instrument for TODO/interpose.md T-1311. The
# implementation (that entry's Approach) writes the repro. This stub exists
# so the entry's citation resolves in a fresh clone. It runs nothing and
# reports that it could not run.
set -eu
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
printf '162-tar-symlink-modes\n'
printf 'repo: %s\n' "$HERE"
printf 'status: not implemented (TODO/interpose.md T-1311 owns the repro)\n'
printf 'date: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
exit 2
