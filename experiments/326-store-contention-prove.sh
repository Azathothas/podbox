#!/bin/sh
# 326-store-contention-prove.sh: does the podbox-image suite pass repeatedly
# with default parallelism, and does the ceiling test pin the sixteen slots?
#
# Status: acceptance instrument for TODO/image.md T-1310. The implementation
# (task 3 of that entry) writes the loop. This stub exists so the entry's
# citation resolves in a fresh clone. It runs nothing and reports that it
# could not run.
set -eu
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
printf '326-store-contention-prove\n'
printf 'repo: %s\n' "$HERE"
printf 'status: not implemented (TODO/image.md T-1310 task 3 owns the loop)\n'
printf 'date: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
exit 2
