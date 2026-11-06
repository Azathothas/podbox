#!/bin/sh
# Lane job: the gate's fast half on the current tree (markers especially).
set -u
cd /work || exit 2
./scripts/common/check-gate.sh --fast || exit 1
echo JOB_DONE
